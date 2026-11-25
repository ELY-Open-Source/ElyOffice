//! Arrows and wheels past a single page turn pages.

use std::time::{Duration, Instant};

use gpui::{
    Context, DispatchPhase, Entity, HitboxBehavior, IntoElement, Pixels, ScrollDelta, ScrollEvent,
    Styled as _, TouchPhase, canvas,
};

use crate::layout::{GAP, Mode, pair};
use crate::view::PdfView;

/// Trackpad pixels past the edge that turn a page.
const PUSH: f32 = 120.0;
/// A pause this long ends a swipe and its momentum.
const QUIET: Duration = Duration::from_millis(180);
/// Pixels an arrow key scrolls.
const ARROW: f32 = 40.0;

/// A swipe's push past a single page's edge.
#[derive(Default)]
pub struct Flip {
    push: f32,
    /// This swipe turned already.
    spent: bool,
    last: Option<Instant>,
}

/// Which way: across or down, and toward the end.
#[derive(Clone, Copy)]
pub struct Way {
    pub across: bool,
    pub on: bool,
}

impl PdfView {
    /// The scroll, a pending jump first, and its ends.
    fn spot(&self) -> ((f32, f32), (f32, f32)) {
        let offset = self.scroll.offset();
        let at = self.go_to.unwrap_or((-f32::from(offset.x), -f32::from(offset.y)));
        let laid = self.laid_out();
        let room = (f32::from(self.room.size.width), f32::from(self.room.size.height));
        (at, ((laid.size.0 - room.0).max(0.0), (laid.size.1 - room.1).max(0.0)))
    }

    /// A wheel past a lone page turns; true if taken.
    pub fn wheel_turn(&mut self, event: &ScrollEvent, line: Pixels, cx: &mut Context<Self>) -> bool {
        if self.mode != Mode::Single {
            return false;
        }
        let delta = event.delta.pixel_delta(line);
        let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
        let across = dx.abs() > dy.abs();
        let push = if across { dx } else { dy };
        if push == 0.0 {
            return false;
        }
        // A wheel notch, not a swipe: each turns alone.
        let notch = match event.delta {
            ScrollDelta::Lines(lines) => (if across { lines.x } else { lines.y }).abs() >= 1.0,
            ScrollDelta::Pixels(_) => false,
        };
        // The executor's clock, which tests drive.
        let now = cx.background_executor().now();
        let pause = self.flip.last.is_none_or(|last| now - last > QUIET);
        if notch || pause || event.touch_phase == TouchPhase::Started {
            self.flip = Flip::default();
        }
        self.flip.last = Some(now);
        // The rest of a swipe that turned.
        if self.flip.spent {
            return true;
        }
        // Scrolling down lowers the offset.
        let way = Way { across, on: push < 0.0 };
        let ((x, y), (end_x, end_y)) = self.spot();
        let (at, end) = if across { (x, end_x) } else { (y, end_y) };
        if if way.on { at < end - 0.5 } else { at > 0.5 } {
            self.flip.push = 0.0;
            return false;
        }
        if !notch {
            self.flip.push += push.abs();
            if self.flip.push < PUSH {
                return true;
            }
            self.flip.spent = true;
        }
        log::debug!("pdf: a wheel past the page turns it");
        self.flip.push = 0.0;
        self.turn_from_edge(way, cx);
        true
    }

    /// An arrow: one step, or a turn at the edge.
    pub fn arrow(&mut self, way: Way, cx: &mut Context<Self>) {
        let ((x, y), (end_x, end_y)) = self.spot();
        let (at, (low, high)) = match way.across {
            true => (x, self.span_across(end_x)),
            false => (y, (0.0, end_y)),
        };
        let room = if way.on { high - at } else { at - low };
        if room > 0.5 {
            let step = ARROW.min(room) * if way.on { 1.0 } else { -1.0 };
            self.forget_place();
            self.go_to = Some(if way.across { (x + step, y) } else { (x, y + step) });
            return cx.notify();
        }
        if way.across || self.mode == Mode::Single {
            self.turn_from_edge(way, cx);
        }
    }

    /// Scroll range across that keeps the current page or pair.
    fn span_across(&self, end: f32) -> (f32, f32) {
        let (laid, row) = (self.laid_out(), pair(self.mode, self.current));
        let mine = laid.placed.iter().filter(|place| pair(self.mode, place.page) == row);
        let (left, right) = mine.fold((f32::INFINITY, 0.0f32), |(l, r), p| (l.min(p.x), r.max(p.x + p.w)));
        let low = (left - GAP).clamp(0.0, end);
        (low, (right + GAP - f32::from(self.room.size.width)).clamp(low, end))
    }

    /// Turns one way; back down lands at the page's end.
    fn turn_from_edge(&mut self, way: Way, cx: &mut Context<Self>) {
        let from = self.current;
        self.turn(if way.on { 1 } else { -1 }, cx);
        if way.on || way.across || self.current == from {
            return;
        }
        let end = self.spot().1.1;
        if let Some((x, _)) = self.go_to {
            self.go_to = Some((x, end));
        }
    }
}

/// Hands a wheel to the view before its pages scroll.
pub fn edge_layer(view: Entity<PdfView>) -> impl IntoElement {
    canvas(
        |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
        move |_, hitbox, window, _| {
            window.on_mouse_event(move |event: &ScrollEvent, phase, window, cx| {
                if phase == DispatchPhase::Capture && !event.modifiers.secondary(cx) && hitbox.should_handle_scroll(window) {
                    let line = window.line_height();
                    if view.update(cx, |view, cx| view.wheel_turn(event, line, cx)) {
                        cx.stop_propagation();
                    }
                }
            })
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}
