//! The slide strip and the Slide menu (SHOW-01).

use std::sync::Arc;

use ely_gpui_component::{
    menus::{ContextMenu, Menu as Popup, MenuItem as PopupItem},
    primitives::{FocusRing as _, Icon, IconName},
    theme::ActiveTheme as _,
};
use ely_office::prelude::*;
use gpui::{
    Action, App, Context, Div, InteractiveElement as _, IntoElement, Menu, MenuItem,
    ParentElement as _, Render, RenderImage, Role, StatefulInteractiveElement as _, Styled as _,
    WeakEntity, Window, actions, div, img, prelude::FluentBuilder as _, px,
};
use image::{Frame, ImageBuffer, Rgba};

use crate::deck::Deck;

actions!(
    show,
    [
        NewSlide,
        DuplicateSlide,
        DeleteSlide,
        HideSlide,
        ShowSlide,
        MoveSlideUp,
        MoveSlideDown,
        MoveSlideFirst,
        MoveSlideLast
    ]
);

/// Thumbnail pixels: twice the shown size.
const PICTURE: (u32, u32) = (320, 180);
/// The shown width in pixels.
const WIDTH: f32 = 160.0;

pub fn menu(cx: &App) -> Menu {
    let label = |key: &str| Deck::text(cx, key, &[]);
    Menu::new(label("menu.slide")).items([
        MenuItem::action(label("show.slide.new"), NewSlide),
        MenuItem::action(label("show.slide.duplicate"), DuplicateSlide),
        MenuItem::action(label("show.slide.delete"), DeleteSlide),
        MenuItem::separator(),
        MenuItem::action(label("show.slide.hide"), HideSlide),
        MenuItem::action(label("show.slide.show"), ShowSlide),
        MenuItem::separator(),
        MenuItem::action(label("show.slide.up"), MoveSlideUp),
        MenuItem::action(label("show.slide.down"), MoveSlideDown),
        MenuItem::action(label("show.slide.first"), MoveSlideFirst),
        MenuItem::action(label("show.slide.last"), MoveSlideLast),
    ])
}

/// The same commands on a right click.
fn popup(cx: &App) -> Popup {
    let item = |key: &str, action: Box<dyn Action>| {
        PopupItem::new(Deck::text(cx, key, &[]))
            .on_click(move |window, cx| window.dispatch_action(action.boxed_clone(), cx))
    };
    Popup::new()
        .item(item("show.slide.new", Box::new(NewSlide)))
        .item(item("show.slide.duplicate", Box::new(DuplicateSlide)))
        .item(item("show.slide.delete", Box::new(DeleteSlide)))
        .separator()
        .item(item("show.slide.hide", Box::new(HideSlide)))
        .item(item("show.slide.show", Box::new(ShowSlide)))
}

/// Binds an action to a slide command.
macro_rules! run {
    ($root:expr, $cx:expr, $action:ty, $id:literal) => {
        $root.on_action($cx.listener(|deck, _: &$action, _, cx| deck.slide_command($id, cx)))
    };
}

pub fn bind(root: Div, cx: &mut Context<Deck>) -> Div {
    let root = run!(root, cx, NewSlide, "slide.insert");
    let root = run!(root, cx, DuplicateSlide, "slide.duplicate");
    let root = run!(root, cx, DeleteSlide, "slide.delete");
    let root = run!(root, cx, HideSlide, "slide.hide");
    let root = run!(root, cx, ShowSlide, "slide.show");
    let root = run!(root, cx, MoveSlideUp, "slide.up");
    let root = run!(root, cx, MoveSlideDown, "slide.down");
    let root = run!(root, cx, MoveSlideFirst, "slide.first");
    run!(root, cx, MoveSlideLast, "slide.last")
}

impl Deck {
    /// Runs a slide command; masters take none.
    fn slide_command(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.active_tab().is_some_and(|tab| tab.extra.masters) {
            let text = Deck::text(cx, "show.masters.busy", &[]);
            return self.toast(text, ely_gpui_component::primitives::Severity::Warning, cx);
        }
        self.run_active(id, &[], cx);
    }

    /// Paints the shown deck's next missing or stale picture.
    pub fn paint_next(&mut self, id: u64, cx: &mut Context<Self>) {
        // Only the shown strip; others wait their turn.
        if self.core.active() != Some(id) {
            return;
        }
        let Some(tab) = self.tab(id) else {
            return;
        };
        let extra = &tab.extra;
        if extra.painting {
            return;
        }
        let next = extra.slides.list.iter().enumerate().find(|(_, slide)| {
            !extra.failed.contains(&slide.name)
                && (!extra.thumbs.contains_key(&slide.name) || extra.stale.contains(&slide.name))
        });
        let Some((part, slide)) = next else {
            return;
        };
        if !self.settled(id, cx) {
            return self.when_settled(id, move |deck, _, cx| deck.paint_next(id, cx));
        }
        let (name, master) = (slide.name.clone(), extra.masters);
        let failing = name.clone();
        let doc = tab.doc.clone();
        let tab = self.tab_mut(id).expect("the tab is open");
        tab.extra.painting = true;
        tab.extra.stale.remove(&name);
        ely_lok::read_off(
            &self.core.worker,
            &doc,
            move |worker, doc| doc.paint_part(worker, (part, master), PICTURE),
            move |deck: &mut Self, painted, cx| {
                let Some(tab) = deck.tab_mut(id) else {
                    return;
                };
                tab.extra.painting = false;
                match painted {
                    Ok(pixels) => {
                        let image =
                            ImageBuffer::<Rgba<u8>, _>::from_raw(PICTURE.0, PICTURE.1, pixels)
                                .expect("the worker paints the asked size");
                        let image = Arc::new(RenderImage::new([Frame::new(image)]));
                        if let Some(old) = tab.extra.thumbs.insert(name, image) {
                            tab.extra.retired.push(old);
                        }
                    }
                    Err(error) => {
                        tab.extra.failed.insert(failing);
                        deck.engine_failed(&error.to_string(), cx)
                    }
                }
                cx.notify();
                deck.paint_next(id, cx);
            },
            cx,
        );
    }
}

/// The left column: one picture per slide or master.
pub struct Strip {
    deck: WeakEntity<Deck>,
}

impl Strip {
    pub fn new(deck: WeakEntity<Deck>, cx: &mut Context<Self>) -> Self {
        if let Some(deck) = deck.upgrade() {
            cx.observe(&deck, |_, _, cx| cx.notify()).detach();
        }
        Strip { deck }
    }
}

impl Deck {
    /// Frees pictures no longer drawn, in every tab.
    pub fn release_pictures(&mut self, window: &mut Window) {
        self.retired.release(window);
        for tab in &mut self.core.tabs {
            tab.extra.retired.release(window);
        }
    }

    /// A closing tab's pictures wait to be freed.
    pub fn retire_tab(&mut self, tab: &ely_office::Tab<crate::deck::Extra, ely_office::Lok>) {
        for picture in tab.extra.thumbs.values() {
            self.retired.push(picture.clone());
        }
        self.retired.append(tab.extra.retired.clone());
    }
}

impl crate::deck::Extra {
    /// Pictures of slides no longer there go.
    pub fn keep_pictures(&mut self, names: &std::collections::HashSet<&String>) {
        let retired = &mut self.retired;
        self.thumbs.retain(|name, picture| {
            let kept = names.contains(name);
            if !kept {
                retired.push(picture.clone());
            }
            kept
        });
    }

    /// Every picture goes, to be painted again.
    pub fn drop_pictures(&mut self) {
        for (_, picture) in self.thumbs.drain() {
            self.retired.push(picture);
        }
    }
}

impl Render for Strip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(deck) = self.deck.upgrade() else {
            return div();
        };
        let colors = cx.theme().colors.clone();
        let read = deck.read(cx);
        let Some(tab) = read.active_tab() else {
            return div();
        };
        let height = WIDTH * PICTURE.1 as f32 / PICTURE.0 as f32;
        let hidden = Deck::text(cx, "show.slide.hidden", &[]);
        let count = tab.extra.slides.list.len();
        let slides = tab
            .extra
            .slides
            .list
            .iter()
            .enumerate()
            .map(|(part, slide)| {
                let current = part == tab.extra.slides.current;
                let picture = tab.extra.thumbs.get(&slide.name).cloned();
                let (view, pressed) = (tab.view.clone(), tab.view.clone());
                let label = format!("{} {}", part + 1, slide.name);
                div()
                    .id(("show-slide", part))
                    .debug_selector(move || format!("show-slide-{part}"))
                    .tab_index(0)
                    .focus_ring(cx)
                    .flex()
                    .gap_2()
                    .items_start()
                    .p_1()
                    .cursor_pointer()
                    .on_click(move |_, _, cx| view.update(cx, |view, _| view.show_part(part)))
                    .on_mouse_down(gpui::MouseButton::Right, move |_, _, cx| {
                        pressed.update(cx, |view, _| view.show_part(part))
                    })
                    .child(
                        div()
                            .w_5()
                            .text_xs()
                            .text_color(colors.fg_subtle)
                            .child((part + 1).to_string()),
                    )
                    .child(
                        div()
                            .w(px(WIDTH))
                            .h(px(height))
                            .relative()
                            .border_1()
                            .border_color(if current { colors.focus } else { colors.border })
                            .when(!slide.visible, |frame| frame.opacity(0.5))
                            .when_some(picture, |frame, picture| {
                                frame.child(img(picture).id(("show-picture", part)).size_full())
                            })
                            .when(!slide.visible, |frame| {
                                frame.child(
                                    div()
                                        .absolute()
                                        .top_1()
                                        .right_1()
                                        .child(Icon::new(IconName::EyeOff)),
                                )
                            }),
                    )
                    .role(Role::ListItem)
                    .aria_label(label)
                    .aria_selected(current)
                    .aria_position_in_set(part + 1)
                    .aria_size_of_set(count)
                    .when(!slide.visible, |row| row.aria_description(hidden.clone()))
            });
        let list = div()
            .id("show-strip")
            .role(Role::List)
            .aria_label(Deck::text(
                cx,
                if tab.extra.masters {
                    "show.strip.masters"
                } else {
                    "show.strip.slides"
                },
                &[],
            ))
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .children(slides);
        div()
            .size_full()
            // A sized host lets the list overflow and scroll.
            .child(ContextMenu::new("show-strip-menu", popup(cx)).size_full().child(list))
    }
}
