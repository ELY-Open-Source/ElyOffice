//! Starting a show: a copy read, screens picked, windows open.

use std::{
    io,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::{Choice, Select, Switch},
    overlays::Dialog as Card,
    primitives::Severity,
};
use ely_lok::{Doc, Worker, show::ShowSlide};
use ely_office::prelude::*;
use ely_platform::AppId;
use gpui::{
    AnyElement, AnyWindowHandle, App, AppContext as _, Bounds, Context, Entity, EntityId, IntoElement,
    ParentElement as _, Pixels, PlatformDisplay, Window, WindowBounds, WindowOptions,
};

use super::{Show, audience::Audience, presenter::Presenter};
use crate::deck::Deck;
use crate::dialog::{Dialog, edit};

/// Layer pixels never pass this width.
const WIDEST: f32 = 1920.0;

/// A show waiting for its screens.
pub struct Starting {
    tab: u64,
    slides: Vec<ShowSlide>,
    first: usize,
    /// The copy the show plays, and its file.
    doc: Doc,
    copy: PathBuf,
    /// The audience's screen, by place in the list.
    audience: usize,
    presenter: bool,
}

/// Closes and deletes the show's copy, off the window.
pub fn discard(worker: Arc<Mutex<Worker>>, doc: Doc, copy: PathBuf, cx: &App) {
    cx.background_spawn(async move {
        if let Err(error) = worker.lock().expect("the worker lock").close_doc(doc) {
            log::error!("show: the copy did not close: {error}");
        }
        if let Err(error) = std::fs::remove_file(&copy) {
            log::warn!("show: {} stays: {error}", copy.display());
        }
    })
    .detach();
}

/// Saves `doc` as the show's copy and reads it.
fn copy(
    worker: &mut Worker,
    doc: &Doc,
    copy: &std::path::Path,
) -> io::Result<(usize, Doc, Vec<ShowSlide>)> {
    let current = doc.sheets(worker)?.current;
    let url = ely_lok::file_url(copy);
    doc.save_as(worker, &url, "odp")?;
    let shown = worker.load(&url)?;
    let read = |worker: &mut Worker| -> io::Result<Vec<ShowSlide>> {
        worker.run(&shown, "view.theme", &[ely_lok::SCHEME])?;
        ely_lok::show::slide_show(&worker.values(&shown, ely_lok::show::SLIDE_SHOW)?)
    };
    match read(worker) {
        Ok(slides) => Ok((current, shown, slides)),
        Err(error) => {
            worker.close_doc(shown)?;
            Err(error)
        }
    }
}

/// macOS covers its menu bar and Dock by simple fullscreen.
const SIMPLE: bool = cfg!(target_os = "macos");

/// A show window's bounds: fullscreen, or macOS's simple kind.
fn bounds(area: Bounds<Pixels>) -> WindowBounds {
    match SIMPLE {
        true => WindowBounds::Windowed(area),
        false => WindowBounds::Fullscreen(area),
    }
}

fn options(display: &Rc<dyn PlatformDisplay>) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(bounds(display.bounds())),
        display_id: Some(display.id()),
        titlebar: None,
        focus: true,
        app_id: Some(AppId::Show.slug().into()),
        ..Default::default()
    }
}

/// Hides the menu bar and Dock over a macOS show.
fn cover(window: &mut Window) {
    if SIMPLE {
        window.toggle_simple_fullscreen();
    }
}

/// Opens one show window; its root scope holds the keys.
pub fn show_window(
    show: &Entity<Show>,
    display: &Rc<dyn PlatformDisplay>,
    presenter: bool,
    cx: &mut App,
) -> gpui::Result<AnyWindowHandle> {
    let show = show.clone();
    Ok(match presenter {
        true => cx
            .open_window(options(display), |window, cx| {
                cover(window);
                let view = cx.new(|cx| Presenter::new(show, cx));
                window.focus(&view.read(cx).focus.clone(), cx);
                view
            })?
            .into(),
        false => cx
            .open_window(options(display), |window, cx| {
                cover(window);
                let view = cx.new(|cx| Audience::new(show, cx));
                window.focus(&view.read(cx).focus.clone(), cx);
                view
            })?
            .into(),
    })
}

impl Deck {
    /// Plays a copy from the first or shown slide.
    pub fn start_show(&mut self, here: bool, cx: &mut Context<Self>) {
        if self.show.is_some() || self.show_starting {
            log::info!("show: already playing or starting");
            let text = Deck::text(cx, "show.play.busy", &[]);
            return self.toast(text, Severity::Warning, cx);
        }
        let Some(id) = self.core.active() else {
            return;
        };
        self.show_starting = true;
        log::info!("show: starts, here {here}");
        if !self.settled(id, cx) {
            return self.when_settled(id, move |deck, _, cx| deck.read_show(id, here, cx));
        }
        self.read_show(id, here, cx);
    }

    fn read_show(&mut self, id: u64, here: bool, cx: &mut Context<Self>) {
        let Some(tab) = self.tab(id) else {
            self.show_starting = false;
            return;
        };
        let path = tab.work.join(format!("show-{}.odp", self.shows));
        self.shows += 1;
        let into = path.clone();
        let worker = self.core.worker.clone();
        self.read_in(
            id,
            move |worker, doc| copy(worker, doc, &into),
            move |deck, read, _, cx| {
                deck.show_starting = false;
                let (current, doc, slides) = match read {
                    Ok(read) => read,
                    Err(error) => return deck.engine_failed(&format!("the show: {error}"), cx),
                };
                let first = match here {
                    true => Some(current).filter(|at| *at < slides.len()),
                    false => slides.iter().position(|slide| !slide.hidden),
                };
                let Some(first) = first.filter(|_| deck.core.active() == Some(id)) else {
                    discard(worker, doc, path, cx);
                    if deck.core.active() != Some(id) {
                        log::info!("show: tab {id} is no longer shown");
                        let text = Deck::text(cx, "show.play.gone", &[]);
                        return deck.toast(text, Severity::Warning, cx);
                    }
                    let text = Deck::text(cx, "show.play.none", &[]);
                    return deck.toast(text, Severity::Warning, cx);
                };
                let starting = Starting {
                    tab: id,
                    slides,
                    first,
                    doc,
                    copy: path,
                    audience: 0,
                    presenter: true,
                };
                match cx.displays().len() {
                    1 => deck.open_show(starting, cx),
                    _ => match deck.open_dialog(Dialog::Show(starting), cx) {
                        Ok(()) => deck.show_starting = true,
                        Err(Dialog::Show(refused)) => discard(worker, refused.doc, refused.copy, cx),
                        Err(_) => unreachable!("the show dialog comes back"),
                    },
                }
            },
            cx,
        );
    }

    fn open_show(&mut self, starting: Starting, cx: &mut Context<Self>) {
        self.show_starting = false;
        let worker = self.core.worker.clone();
        let displays = cx.displays();
        let (Some(tab), Some(audience)) = (self.tab(starting.tab), displays.get(starting.audience))
        else {
            log::warn!("show: the tab or screen {} is gone", starting.audience);
            let text = Deck::text(cx, "show.play.gone", &[]);
            self.toast(text, Severity::Warning, cx);
            return discard(worker, starting.doc, starting.copy, cx);
        };
        let presenter = (displays.iter())
            .find(|display| display.id() != audience.id())
            .filter(|_| starting.presenter);
        let slide = crate::design::mm100(tab.extra.size);
        let slide = (slide.0 as f32, slide.1 as f32);
        let room = audience.bounds().size;
        let fitted = super::stage::fit(slide, (room.width, room.height));
        let width = (f32::from(fitted.0) * 2.0).min(WIDEST).round() as u32;
        let pixels = (width, (width as f32 * slide.1 / slide.0).round() as u32);
        let deck = cx.weak_entity();
        let played = (worker, starting.doc, starting.copy);
        let show = cx.new(|cx| {
            Show::new(deck, played, (starting.slides, starting.first), (slide, pixels), cx)
        });
        let mut windows = Vec::new();
        // The audience window opens last, so it is in front.
        let screens = presenter.map(|d| (d, true)).into_iter().chain([(audience, false)]);
        for (display, presenter) in screens {
            match show_window(&show, display, presenter, cx) {
                Ok(handle) => windows.push(handle),
                Err(error) => {
                    show.update(cx, |show, cx| {
                        show.windows = windows;
                        show.close(cx);
                    });
                    return self.engine_failed(&format!("the show window: {error}"), cx);
                }
            }
        }
        log::info!("show: {} windows open", windows.len());
        show.update(cx, |show, _| {
            show.presenter = windows.len() > 1;
            show.windows = windows;
        });
        self.show = Some(show);
        cx.notify();
    }

    /// Show `id` closed its windows.
    pub fn show_ended(&mut self, id: EntityId, cx: &mut Context<Self>) {
        if self.show.as_ref().is_some_and(|show| show.entity_id() == id) {
            self.show = None;
            cx.notify();
        }
    }

    /// Picks screens before the show opens.
    pub fn show_dialog(&self, starting: &Starting, cx: &mut Context<Self>) -> AnyElement {
        let this = cx.entity();
        let label = |key: &str| Deck::text(cx, key, &[]);
        let screens: Vec<Choice> = (cx.displays().iter().enumerate())
            .map(|(at, display)| {
                let size = display.bounds().size;
                let name = Deck::text(
                    cx,
                    "show.play.screen",
                    &[
                        ("number", &(at + 1).to_string()),
                        ("width", &f32::from(size.width).round().to_string()),
                        ("height", &f32::from(size.height).round().to_string()),
                    ],
                );
                Choice::new(at.to_string(), name)
            })
            .collect();
        let (pick, flip) = (this.clone(), this.clone());
        let screen = Select::new("show-play-screen", screens)
            .label(label("show.play.audience"))
            .selected(starting.audience.to_string())
            .on_change(move |value, _, cx| {
                let at = value.parse().expect("screens go by number");
                edit(&pick, cx, |dialog, _| {
                    if let Dialog::Show(starting) = dialog {
                        starting.audience = at;
                    }
                })
            });
        let presenter = Switch::new("show-play-presenter", starting.presenter)
            .name(label("show.play.presenter"))
            .on_change(move |on, _, cx| {
                edit(&flip, cx, |dialog, _| {
                    if let Dialog::Show(starting) = dialog {
                        starting.presenter = on;
                    }
                })
            });
        let rows = DescriptionList::new()
            .item(label("show.play.audience"), screen)
            .item(label("show.play.presenter"), presenter);
        let closing = this.clone();
        let dismiss = move |_: &mut Window, cx: &mut App| {
            closing.update(cx, |deck, cx| {
                if let Some(Dialog::Show(starting)) = deck.dialog.take() {
                    deck.show_starting = false;
                    discard(deck.core.worker.clone(), starting.doc, starting.copy, cx);
                }
                cx.notify();
            })
        };
        let go = label("show.play.start");
        Card::new("show-play", label("show.play.title"), dismiss)
            .child(rows)
            .action(ely_office::cancel(cx))
            .action(move |_| {
                Button::new("show-play-go", go)
                    .variant(ButtonVariant::Primary)
                    .on_click(move |_, _, cx| {
                        this.update(cx, |deck, cx| {
                            if let Some(Dialog::Show(starting)) = deck.dialog.take() {
                                deck.open_show(starting, cx);
                            }
                        })
                    })
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, WindowBounds, point, px, size};

    #[test]
    fn a_show_covers_the_display_on_every_platform() {
        let area = Bounds::new(point(px(0.), px(0.)), size(px(1920.), px(1080.)));
        let want = match cfg!(target_os = "macos") {
            true => WindowBounds::Windowed(area),
            false => WindowBounds::Fullscreen(area),
        };
        assert_eq!(super::bounds(area), want);
    }

    #[test]
    fn a_start_whose_screen_is_gone_says_so_and_drops_its_copy() {
        use ely_office::prelude::*;
        use gpui::AppContext as _;

        use crate::tests::{settle, update};
        let dir = tempfile::tempdir().expect("a temp dir");
        let mut cx = gpui::TestAppContext::single();
        let Some(deck) = crate::tests::deck(&mut cx, dir.path()) else {
            eprintln!("SKIP: no LOK build");
            return;
        };
        update(&mut cx, &deck, |deck, cx| deck.new_blank(cx));
        settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| !tab.extra.slides.list.is_empty()));
        // The window's toasts live on its root.
        let handle = cx.update(|app| deck.read(app).core.window).expect("a window");
        let root = cx
            .update_window(handle, |_, window, app| {
                let root = ely_ui::document_root(ely_platform::AppId::Show, deck.clone().into(), &ely_ui::commands::FILE, Vec::new(), window, app);
                deck.update(app, |deck, _| deck.core.root = root.downgrade());
                root
            })
            .expect("the window lives");
        let path = dir.path().join("copy.odp");
        let starting = cx.update(|app| {
            let deck = deck.read(app);
            let tab = deck.active_tab().expect("a tab");
            let mut worker = deck.core.worker.lock().expect("the worker lock");
            let (_, doc, slides) = super::copy(&mut worker, &tab.doc, &path).expect("a copy");
            super::Starting { tab: tab.id, slides, first: 0, doc, copy: path.clone(), audience: 99, presenter: false }
        });
        update(&mut cx, &deck, |deck, cx| {
            deck.show_starting = true;
            deck.open_show(starting, cx);
        });
        settle(&cx, |_| !path.exists());
        let (toasts, gone) = cx.update(|app| (root.read(app).toasts(app), crate::deck::Deck::text(app, "show.play.gone", &[])));
        assert!(toasts.iter().any(|(text, _)| *text == gone), "{toasts:?}");
        assert!(cx.update(|app| !deck.read(app).show_starting && deck.read(app).show.is_none()));
    }
}
