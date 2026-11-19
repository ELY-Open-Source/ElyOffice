//! ElyShow's window: decks over one LOK worker (§10.1).

use std::{collections::HashSet, path::PathBuf};

use ely_gpui_component::{primitives::IconName, shell::StatusBar};
use ely_lok::{LokView, show::Design};
use ely_office::{Core, Kind, prelude::*};
use ely_platform::AppId;
use ely_ui::{Root, Shell, commands};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, Styled as _, Window, div,
};

use crate::design::Panel;
use crate::slides::Strip;

pub use crate::extra::Extra;

/// Menu checks: grid, guides, comments; undo, redo.
pub type Snapped = ((bool, bool, bool), (bool, bool));

pub struct Deck {
    pub core: Core<Deck>,
    pub dialog: Option<crate::dialog::Dialog>,
    pub strip: Entity<Strip>,
    pub panel: Entity<Panel>,
    /// The kernel's own blank deck, written once.
    pub blank: Option<PathBuf>,
    /// The kernel's font families; read once.
    pub fonts: Vec<SharedString>,
    reading_fonts: bool,
    pub guide_drag: Option<crate::guides::Drag>,
    pub notes: crate::notes::NotesField,
    /// The right column shows comments, not properties.
    pub remarks_open: bool,
    pub preview: Option<crate::preview::Preview>,
    /// Counts preview requests; only the last may play.
    pub preview_asked: u64,
    pub probe: Option<crate::media::Probe>,
    pub playing: Option<crate::media::Playing>,
    pub show: Option<Entity<crate::slideshow::Show>>,
    /// A show is being read or its screens picked.
    pub show_starting: bool,
    /// Shows started; each copy has its own file.
    pub shows: u64,
    /// Reads that end in a dialog; only the last opens.
    pub dialog_asked: u64,
    /// Dialog kind and tab last seen; a change ends asks.
    pub dialog_seen: (Option<std::mem::Discriminant<crate::dialog::Dialog>>, Option<u64>),
    pub active_remark: Option<SharedString>,
    pub remarks_pane: Entity<crate::remarks::Pane>,
    /// What the menus last showed: grid, guides, comments; undo, redo.
    pub snap_menus: Option<Snapped>,
    /// The canvas's right-click menu.
    pub menu: Option<ely_ui::menu::Offer<crate::menu::What>>,
    /// Pictures of closed tabs and ended previews, freed later.
    pub retired: ely_gpui_component::primitives::Retired,
}

impl Host for Deck {
    type Extra = Extra;
    type Engine = ely_office::Lok;
    const KIND: Kind = Kind {
        app: AppId::Show,
        kept: &["pptx", "odp", "ppt", "fodp"],
        save_as: &["pptx", "odp"],
        macros: Some(&["pptm", "potm", "ppsm"]),
    };

    fn core(&self) -> &Core<Self> {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core<Self> {
        &mut self.core
    }

    fn tab_closed(&mut self, tab: &ely_office::Tab<Extra, ely_office::Lok>, cx: &mut Context<Self>) {
        self.retire_tab(tab);
        self.sync_menus(cx);
    }

    fn tab_shown(&mut self, id: u64, cx: &mut Context<Self>) {
        if let Some(view) = self.tab(id).map(|tab| tab.view.clone()) {
            self.reread(id, &view, cx);
        }
        self.paint_next(id, cx);
    }

    /// Its pictures stay with it; this window frees its copies.
    fn tab_leaving(&mut self, id: u64, _: &mut Context<Self>) {
        self.hand_pictures(id);
    }

    /// Its dialogs, preview and show serve the active tab.
    fn asks_about(&self, id: u64) -> bool {
        let active = self.core.active() == Some(id);
        let showing = (self.preview.is_some() || self.show.is_some() || self.show_starting) && active;
        self.dialog.as_ref().is_some_and(|dialog| dialog.about(id, active)) || showing
    }

    fn read_at(&self, tab: u64, _: &gpui::App) -> Option<ely_lok::Doc> {
        self.tab(tab).and_then(|tab| tab.extra.read_at.clone())
    }

    fn observed(&mut self, id: u64, view: &Entity<LokView>, cx: &mut Context<Self>) {
        // Pictures before the scheme landed are repainted.
        let lit = view.read(cx).lit();
        if let Some(tab) = self.tab_mut(id)
            && tab.extra.lit != lit
        {
            tab.extra.lit = lit;
            self.repaint_all(id, cx);
        }
        self.offer_menu(id, view, cx);
        self.reread(id, view, cx);
        // Undo's state can change with no new read.
        if self.core.active() == Some(id) {
            self.sync_menus(cx);
        }
    }
}

/// ElyShow's menus after File.
pub fn menus(on: (bool, bool, bool), undo: (bool, bool), cx: &App) -> Vec<gpui::Menu> {
    let [insert, view] = crate::insert::menus(on, cx);
    let edit = gpui::Menu::new(ely_ui::i18n::text(cx, "menu.edit", &[])).items(ely_office::undo_items(undo, cx));
    vec![edit, crate::slides::menu(cx), insert, view, crate::slideshow::menu(cx)]
}

/// The window root with a blank deck.
pub fn open(app: AppId, window: &mut Window, cx: &mut App) -> Entity<Root> {
    ely_lok::init(cx);
    let (worker, stop, work) = match ely_office::start(app, cx) {
        Ok(started) => started,
        Err(error) => ely_platform::startup::fail(app, &format!("{error}")),
    };
    let deck = crate::window::deck(|cx| Core::new(worker, stop, work, cx), cx);
    crate::slideshow::bind_keys(cx);
    ely_office::bind_zoom_keys(cx);
    ely_office::bind_undo_keys(cx);
    let root = crate::window::root(&deck, window, cx);
    let launched = ely_ui::launch_files(cx);
    deck.update(cx, |deck, cx| match launched.is_empty() {
        true => deck.new_blank(cx),
        false => launched.into_iter().for_each(|path| deck.open_file(path, cx)),
    });
    ely_ui::on_ask(cx, |ask, cx| ely_office::remote::route(ask, cx, Deck::answer_request));
    // Files go to their window, else the one in front.
    ely_ui::on_open(cx, |path, window, cx| match crate::window::front(window, cx) {
        Some(front) => front.update(cx, |deck, cx| deck.open_file(path, cx)),
        None => log::error!("show: a file came with no window open"),
    });
    root
}

impl Deck {
    pub fn new(core: Core<Self>, cx: &mut Context<Self>) -> Self {
        let deck = cx.weak_entity();
        // Closing or switching tabs stops playback.
        cx.observe_self(|deck, _| {
            deck.check_playing();
            deck.watch_dialogs();
        })
        .detach();
        // The window that comes to the front shows its menus.
        cx.observe_global::<ely_ui::tray::Front>(|deck: &mut Deck, cx| {
            if ely_ui::tray::main(cx) == deck.core.window {
                deck.snap_menus = None;
                deck.sync_menus(cx);
            }
        })
        .detach();
        Deck {
            core,
            dialog: None,
            strip: cx.new(|cx| Strip::new(deck.clone(), cx)),
            panel: cx.new(|cx| Panel::new(deck.clone(), cx)),
            remarks_pane: cx.new(|_| crate::remarks::Pane { deck }),
            blank: None,
            fonts: Vec::new(),
            reading_fonts: false,
            guide_drag: None,
            snap_menus: Some(((false, false, false), (false, false))),
            menu: None,
            notes: Default::default(),
            remarks_open: false,
            preview: None,
            preview_asked: 0,
            probe: None,
            playing: None,
            show: None,
            show_starting: false,
            shows: 0,
            dialog_asked: 0,
            retired: Default::default(),
            dialog_seen: (None, None),
            active_remark: None,
        }
    }

    pub fn text(cx: &App, key: &str, args: &[(&str, &str)]) -> SharedString {
        ely_ui::i18n::text(cx, key, args)
    }

    /// Rereads slides and design when the kernel moves them.
    fn reread(&mut self, id: u64, view: &Entity<LokView>, cx: &mut Context<Self>) {
        let read = view.read(cx);
        let seen = (
            read.cell().sheets,
            read.edited(),
            read.caret(),
            read.graphic(),
        );
        // Hidden decks read when shown; Replace All reports parts too.
        if self.core.active() != Some(id) {
            return;
        }
        let Some(tab) = self.tab_mut(id) else {
            return;
        };
        // Numbered commands need the objects read at the latest revision.
        let behind = tab.extra.read_at.as_ref().is_some_and(|at| view.read(cx).read_at().newer_than(at));
        if tab.extra.reading || tab.extra.read == Some(seen) && !behind {
            return;
        }
        // Edits dirty their slides; leaving text edit, the read one.
        let mut edited = view.update(cx, |view, _| view.take_edited_parts());
        if tab.extra.read.is_some_and(|(_, _, caret, _)| caret && !seen.2) {
            edited.insert(tab.extra.slides.current);
        }
        for slide in edited.into_iter().filter_map(|part| tab.extra.slides.list.get(part)) {
            tab.extra.stale.insert(slide.name.clone());
        }
        (tab.extra.reading, tab.extra.read) = (true, Some(seen));
        let landed = tab.extra.transition_landed(view.read(cx));
        let sized = tab.extra.size_landed(view.read(cx));
        let doc = tab.doc.clone();
        // The tab stays here until this lands.
        let busy = self.hold(id);
        ely_lok::read_off(
            &self.core.worker,
            &doc,
            |worker, doc| {
                let at = worker.now(doc)?;
                let slides = doc.sheets(worker)?;
                let design = ely_lok::show::design(&worker.values(doc, ely_lok::show::DESIGN)?)?;
                let mut steps = |command| ely_lok::history::steps(&worker.values(doc, command)?);
                let history = (steps(".uno:Undo")?, steps(".uno:Redo")?);
                let objects = ely_lok::show::objects(&worker.values(doc, ely_lok::show::OBJECTS)?)?;
                let notes = ely_lok::show::notes(&worker.values(doc, ely_lok::show::NOTES)?)?;
                let remarks = ely_lok::show::remarks(&worker.values(doc, ".uno:ViewAnnotations")?)?;
                let media = ely_lok::show::media(&worker.values(doc, ely_lok::show::MEDIA)?)?;
                let motion = match design.master_view {
                    true => None,
                    false => Some((
                        ely_lok::show::transition(&worker.values(doc, ely_lok::show::TRANSITION)?)?,
                        ely_lok::show::effects(&worker.values(doc, ely_lok::show::ANIMATIONS)?)?,
                    )),
                };
                Ok((
                    (slides, at),
                    (design, objects, notes, remarks, (motion, media)),
                    doc.size(worker)?,
                    history,
                ))
            },
            move |deck: &mut Self, read, cx| {
                let _busy = busy;
                let Some(tab) = deck.tab_mut(id) else {
                    return;
                };
                tab.extra.reading = false;
                let view = tab.view.clone();
                match read {
                    Ok(((slides, at), (design, objects, notes, remarks, (motion, media)), size, history)) => {
                        tab.extra.read_at = Some(at.clone());
                        log::info!("show: tab {id} has {} parts", slides.list.len());
                        // A queued turn or part outranks an earlier read.
                        view.update(cx, |view, _| {
                            if view.settled() {
                                view.set_part(slides.current)
                            }
                        });
                        let names: HashSet<&String> =
                            slides.list.iter().map(|slide| &slide.name).collect();
                        tab.extra.keep_pictures(&names);
                        // The kernel sends no repaint when the mode flips.
                        if tab.extra.masters != design.master_view {
                            log::info!("show: tab {id} shows masters {}", design.master_view);
                            tab.extra.masters = design.master_view;
                            tab.extra.drop_pictures();
                            view.update(cx, |view, cx| view.invalidate(cx));
                        }
                        // Size, colors and masters touch every picture.
                        let global =
                            |design: &Design| (design.theme.clone(), design.masters.clone());
                        // Undo and redo may reach any slide.
                        let undone = tab.extra.history.as_ref().is_some_and(|before| crate::extra::undid(before, &history));
                        if undone {
                            log::info!("show: tab {id} undid or redid; repainting all");
                        }
                        // Any new step may change the shown slide.
                        let stepped = tab
                            .extra
                            .history
                            .as_ref()
                            .is_some_and(|(undo, _)| *undo != history.0);
                        if let Some(slide) = slides.list.get(slides.current).filter(|_| stepped) {
                            tab.extra.stale.insert(slide.name.clone());
                        }
                        tab.extra.history = Some(history);
                        if undone
                            || tab.extra.size != size
                            || tab.extra.design.as_ref().map(global) != Some(global(&design))
                        {
                            tab.extra.stale = tab.extra.thumbs.keys().cloned().collect();
                        }
                        (tab.extra.slides, tab.extra.design) = (slides, Some(design));
                        let slide = ely_lok::Slide { part: tab.extra.slides.current, size: tab.extra.shown_size(), objects: objects.clone(), at };
                        view.update(cx, |view, _| view.set_slide(Some(slide)));
                        tab.extra.objects = Some(objects);
                        (tab.extra.notes, tab.extra.remarks) = (Some(notes), remarks);
                        (tab.extra.motion, tab.extra.media) = (motion, media);
                        tab.extra.settle_transition(landed);
                        tab.extra.reads += 1;
                        tab.extra.size = size;
                        tab.extra.settle_size(sized);
                        tab.extra.failed.clear();
                        deck.paint_next(id, cx);
                        deck.read_fonts(id, cx);
                        deck.sync_menus(cx);
                    }
                    Err(error) => deck.engine_failed(&error.to_string(), cx),
                }
                cx.notify();
                // The kernel may have moved again meanwhile.
                deck.reread(id, &view, cx);
            },
            cx,
        );
    }

    /// Reads slides and design again once input lands.
    pub fn force_reread(&mut self, id: u64, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |deck, _, cx| deck.force_reread(id, cx));
        }
        let Some(tab) = self.tab_mut(id) else {
            return;
        };
        tab.extra.read = None;
        let view = tab.view.clone();
        self.reread(id, &view, cx);
    }

    /// Checks snapping and greys Undo, Redo as the deck stands.
    pub fn sync_menus(&mut self, cx: &mut Context<Self>) {
        // The app has one menu: the front window's.
        if ely_ui::tray::main(cx) != self.core.window {
            return;
        }
        let shown = match self.active_tab() {
            // Its objects not read yet: the menus wait.
            Some(tab) => match tab.extra.objects.as_ref() {
                Some(objects) => (
                    (objects.snap_grid, objects.snap_guides, self.remarks_open),
                    ely_office::undo_able(tab.view.read(cx)),
                ),
                None => return,
            },
            None => ((false, false, self.remarks_open), (false, false)),
        };
        let on = shown.0;
        if self.snap_menus != Some(shown) {
            self.snap_menus = Some(shown);
            ely_ui::set_document_menus(AppId::Show, &ely_ui::commands::FILE, menus(on, shown.1, cx), cx);
        }
    }

    /// Reads the font families once, off the window.
    fn read_fonts(&mut self, id: u64, cx: &mut Context<Self>) {
        if !self.fonts.is_empty() || self.reading_fonts {
            return;
        }
        let Some(doc) = self.tab_mut(id).map(|tab| tab.doc.clone()) else {
            return;
        };
        self.reading_fonts = true;
        ely_lok::read_off(
            &self.core.worker,
            &doc,
            |worker, doc| ely_lok::commands::fonts(&worker.values(doc, ".uno:CharFontName")?),
            |deck: &mut Self, read, cx| {
                deck.reading_fonts = false;
                match read {
                    Ok(fonts) => deck.fonts = fonts.into_iter().map(Into::into).collect(),
                    Err(error) => deck.engine_failed(&format!("the font list: {error}"), cx),
                }
                cx.notify();
            },
            cx,
        );
    }

    /// Marks every slide's picture stale.
    pub fn repaint_all(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(tab) = self.tab_mut(id) else {
            return;
        };
        tab.extra.stale = tab.extra.thumbs.keys().cloned().collect();
        self.paint_next(id, cx);
    }

    pub(crate) fn body(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(tab) = self.active_tab() else {
            return self.empty_body("show-empty", IconName::Monitor, cx);
        };
        if let Some(page) = self.stopped_page(cx) {
            return page;
        }
        let view = tab.view.clone();
        let guides = self.guide_layer(&view, cx);
        let marks = self.remark_layer(&view, cx);
        let playing = self.media_layer(&view, window, cx);
        let preview = self.preview_layer(&view, window, cx);
        let notes = self.notes_pane(window, cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .child(view)
                    .children(guides)
                    .children(marks)
                    .children(playing)
                    .children(preview),
            )
            .children(notes)
            .into_any_element()
    }
}

impl Render for Deck {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.release_pictures(window);
        // Switching or closing decks changes Undo too.
        self.sync_menus(cx);
        let title = self
            .active_tab()
            .map_or_else(|| AppId::Show.name().into(), |tab| tab.name.clone());
        let shown = self.active_tab().is_some();
        let status = StatusBar::new()
            .left(self.state_item("show-state", cx))
            .when_some(self.zoom_control(cx), |status, zoom| status.right(zoom));
        let shell = Shell::new(title, &self.core.focus)
            .tabs(self.tab_strip(cx))
            .left(shown.then(|| self.strip.clone().into()))
            .right(shown.then(|| match self.remarks_open {
                true => self.remarks_pane.clone().into(),
                false => self.panel.clone().into(),
            }))
            .status(status)
            .child(self.body(window, cx));
        let root = div()
            .size_full()
            .key_context(commands::CONTEXT)
            .track_focus(&self.core.focus)
            .on_action(cx.listener(|deck, _: &commands::New, _, cx| deck.ask_new(cx)))
            .on_action(cx.listener(Self::on_open))
            .on_action(cx.listener(Self::on_save))
            .on_action(cx.listener(Self::on_save_as))
            .on_action(cx.listener(Self::on_export))
            .on_action(cx.listener(Self::on_print))
            .on_action(cx.listener(Self::on_close_tab))
            .on_action(cx.listener(Self::on_show_risk))
            .child(shell)
            .children(self.core_dialog(cx))
            .children(self.dialog_element(cx))
            .children(self.context_menu(cx))
            .children(self.tab_menu(cx));
        let root = crate::slideshow::bind(crate::insert::bind(crate::slides::bind(root, cx), cx), cx);
        Self::bind_undo(Self::bind_zoom(root, cx), cx)
    }
}

impl Zooming for Deck {
    const FITS: bool = true;
}
