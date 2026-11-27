//! The shown slide's overlays while editing.

use std::rc::Rc;

use ely_gpui_component::{feedback::InlineMessage, primitives::Severity};
use ely_lok::LokView;
use ely_office::prelude::*;
use gpui::prelude::FluentBuilder as _;
use gpui::{AnyElement, App, Context, Entity, InteractiveElement as _, IntoElement as _, ParentElement as _, Styled as _, Window, div};

use super::{Ask, Board, ask_table};
use crate::deck::Deck;
use crate::slideshow::{overlays, timer::{Drag, Hand}};

/// The overlays' opacity while an object is picked.
const PICKED: f32 = 0.2;

impl Deck {
    /// Follows the front deck's sheets, unless a show does.
    pub fn follow_slides(&mut self, ask: Ask, cx: &mut Context<Self>) {
        let editing = self.active_tab().filter(|tab| self.show.is_none() && !tab.extra.masters);
        let slides: Vec<_> = editing.map_or_else(Vec::new, |tab| {
            let held = &tab.extra.scoreboards;
            tab.extra.slides.list.iter().map(|slide| held.get(&slide.name).cloned()).collect()
        });
        if self.board.as_ref().is_some_and(|board| board.follows(&slides)) {
            return;
        }
        let old = self.board.take();
        self.board = Board::follow(slides, ask, |deck: &mut Deck| deck.board.as_mut(), cx);
        if let (Some(board), Some(old)) = (&mut self.board, old) {
            board.inherit(old);
        }
    }

    /// Over the slide; clicks pass through to it.
    pub fn scoreboard_layer(&mut self, view: &Entity<LokView>, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.follow_slides(ask_table, cx);
        let tab = self.active_tab()?;
        let part = tab.extra.slides.current;
        let slide = tab.extra.slides.list.get(part).filter(|_| !tab.extra.masters)?;
        let timed = tab.extra.timers.get(&slide.name).copied().zip(tab.extra.clocks.clone());
        let board = self.board.as_ref().filter(|board| board.linked(part));
        if board.is_none() && timed.is_none() {
            return None;
        }
        let (w, h) = tab.extra.shown_size();
        // 1/100 mm to twips.
        let lok = view.read(cx);
        let from = lok.place((0, 0));
        let to = lok.place(((w * 72 / 127) as i32, (h * 72 / 127) as i32));
        let size = (to.x - from.x, to.y - from.y);
        // An empty sheet still says it is linked.
        let said = board.and_then(|board| board.of(part)).and_then(|scores| {
            (scores.failed.clone().map(|why| (Severity::Danger, why)))
                .or_else(|| scores.rows.is_empty().then(|| (Severity::Info, Deck::text(cx, "show.score.blank", &[]))))
        });
        let face = timed.map(|(timer, clocks)| {
            let face = clocks.read(cx).face(&timer, cx.background_executor().now());
            let deck = cx.entity();
            let ask = Rc::new(move |window: &mut Window, cx: &mut App| deck.update(cx, |deck, cx| deck.ask_timer(window, cx)));
            let (reset, edit) = (Deck::text(cx, "show.timer.reset", &[]), Deck::text(cx, "show.timer.edit", &[]));
            let run = Deck::text(cx, face.phase.run_key(), &[]);
            (face, Hand { clocks, timer, run, reset, edit: Some((edit, ask as _)) })
        });
        // Fades while an object is picked, to show it.
        let picked = tab.extra.objects.as_ref().is_some_and(|objects| objects.selection.is_some());
        let card = div().absolute().inset_0().when(picked, |card| card.opacity(PICKED)).children(overlays(board, face, part, size, cx));
        let name = tab.extra.slides.list.get(part)?.name.clone();
        let (id, deck) = (tab.id, cx.entity());
        let slide = div()
            .id("show-slide-overlays")
            .on_drag_move::<Drag>(move |event, _, cx| {
                let at = event.drag(cx).middle(event.event.position, event.bounds);
                deck.update(cx, |deck, cx| deck.drag_timer(id, &name, at, cx));
            })
            .absolute()
            .left(from.x)
            .top(from.y)
            .w(size.0)
            .h(size.1)
            .child(card)
            .children(said.map(|(severity, why)| div().absolute().bottom_2().left_2().child(InlineMessage::new(severity, why))));
        Some(div().absolute().inset_0().overflow_hidden().child(slide).into_any_element())
    }
}
