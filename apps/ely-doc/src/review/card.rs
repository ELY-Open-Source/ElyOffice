//! Accept or reject the change under the caret (#125).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    primitives::raise,
    theme::{ActiveTheme as _, ControlSize, Elevation, Radius, TextSize},
};
use ely_lok::Rect;
use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _, Styled as _, anchored, div,
};

use super::{Review, nav};
use crate::docs::Docs;

/// The armed caret, and the open card's change.
#[derive(Default)]
pub(super) struct Card {
    /// Armed by a click or a move: the caret then.
    armed: Option<Option<Rect>>,
    /// Where the left button went down, in twips.
    pressed: Option<(i32, i32)>,
    /// The open card's change.
    pub(super) open: Option<u64>,
}

impl Review {
    /// A click, move key or arrow may open it.
    pub(crate) fn arm_card(&mut self, cx: &Context<Self>) {
        let caret = self.shown.as_ref().and_then(|shown| shown.view.read(cx).cursor());
        self.card.armed = Some(caret);
    }

    /// An arrow's change: open now, kept until the caret lands.
    pub(super) fn open_card(&mut self, index: u64, cx: &mut Context<Self>) {
        self.arm_card(cx);
        self.card.open = Some(index);
        cx.notify();
    }

    /// A click opens its change, caret moved or not.
    pub(crate) fn click_card(&mut self, up: gpui::Point<gpui::Pixels>, cx: &mut Context<Self>) {
        let (Some(down), Some(shown)) = (self.card.pressed.take(), self.shown.as_ref()) else {
            return;
        };
        let up = shown.view.read(cx).doc_point(up);
        let under = |(x, y): (i32, i32)| {
            let spot = (x, y, 0, 1);
            self.changes.iter().find(|change| nav::holds(&change.ranges, spot)).map(|change| change.index)
        };
        // A drag across changes opens nothing.
        let Some(index) = under(down).filter(|&index| under(up) == Some(index)) else {
            return;
        };
        if self.card.open != Some(index) {
            log::info!("doc: a click opens the change card");
            self.card.open = Some(index);
            cx.notify();
        }
    }

    /// Typing never opens it.
    pub(crate) fn disarm_card(&mut self) {
        self.card.armed = None;
    }

    /// Escape closes the card; whether one was open.
    pub(crate) fn close_card(&mut self, cx: &mut Context<Self>) -> bool {
        self.card.armed = None;
        let open = self.card.open.take().is_some();
        if open {
            log::info!("doc: the change card closes");
            cx.notify();
        }
        open
    }

    /// An armed move opens it; leaving its change closes it.
    pub(super) fn follow_card(&mut self, caret: Option<Rect>, cx: &mut Context<Self>) {
        let at = caret.and_then(|caret| self.changes.iter().find(|change| nav::holds(&change.ranges, caret)));
        let at = at.map(|change| change.index);
        let was = self.card.open;
        match self.card.armed {
            Some(then) if then != caret => {
                self.card.armed = None;
                self.card.open = at;
            }
            // The caret has yet to move.
            Some(_) => {}
            None if self.card.open.is_some() && self.card.open != at => self.card.open = None,
            None => {}
        }
        if self.card.open != was {
            log::info!("doc: the change card {}", if self.card.open.is_some() { "opens" } else { "closes" });
            cx.notify();
        }
    }

    /// The open card's change gone, the card goes.
    pub(super) fn keep_card(&mut self) {
        if let Some(open) = self.card.open
            && !self.changes.iter().any(|change| change.index == open)
        {
            self.card.open = None;
        }
    }

    /// The card under its change's first line, over the page.
    pub(crate) fn card(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let open = self.card.open?;
        let change = self.changes.iter().find(|change| change.index == open)?;
        let shown = self.shown.as_ref()?;
        let view = shown.view.read(cx);
        let first = *change.ranges.first()?;
        if !view.is_visible(first) {
            return None;
        }
        let at = view.screen_rect(first).bottom_left();
        let this = cx.entity();
        let index = change.index.to_string();
        let decide = |id: &'static str, key: &str| {
            let (this, index) = (this.clone(), index.clone());
            Button::new(id, Docs::text(cx, key, &[]))
                .size(ControlSize::Sm)
                .variant(ButtonVariant::Ghost)
                .on_click(move |_, _, cx| {
                    this.update(cx, |review, cx| {
                        log::info!("doc: the change card runs {id}");
                        review.run(id, &[&index], cx);
                        review.close_card(cx);
                    })
                })
        };
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let card = div()
            .block_mouse_except_scroll()
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .max_w(theme.label_width() * 3.0)
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.overlay)
            .shadow(theme.elevation(Elevation::Floating))
            .text_size(theme.text_size(TextSize::Sm))
            .child(nav::tip(change, cx))
            .child(div().text_color(colors.fg_subtle).child(change.words.clone()))
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(decide("track.accept", "doc.review.accept_one"))
                    .child(decide("track.reject", "doc.review.reject_one")),
            );
        let place = anchored().position(at).snap_to_window().child(card);
        Some(raise("doc-change-card", place).into_any_element())
    }
}

/// Keys that move the caret without typing.
const MOVES: [&str; 8] = ["left", "right", "up", "down", "home", "end", "pageup", "pagedown"];

/// Watches the page's clicks and keys for the card.
pub(crate) fn watch(review: &gpui::Entity<Review>, page: gpui::Div) -> gpui::Div {
    let (keys, clicks) = (review.clone(), review.clone());
    page.capture_key_down(move |event, _, cx| {
        let key = event.keystroke.key.clone();
        keys.update(cx, |review, cx| {
            if key == "escape" {
                if review.close_card(cx) {
                    cx.stop_propagation();
                }
            } else if MOVES.contains(&key.as_str()) {
                review.arm_card(cx);
            } else {
                review.disarm_card();
            }
        })
    })
    .capture_any_mouse_down({
        let clicks = clicks.clone();
        move |event, _, cx| {
            if event.button == gpui::MouseButton::Left {
                clicks.update(cx, |review, cx| {
                    review.arm_card(cx);
                    review.card.pressed =
                        review.shown.as_ref().map(|shown| shown.view.read(cx).doc_point(event.position));
                });
            }
        }
    })
    .capture_any_mouse_up(move |event, _, cx| {
        if event.button == gpui::MouseButton::Left {
            clicks.update(cx, |review, cx| review.click_card(event.position, cx));
        }
    })
}
