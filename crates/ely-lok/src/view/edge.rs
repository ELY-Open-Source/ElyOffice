//! A cell drag past the view's edge scrolls it (#15).

use std::time::Duration;

use gpui::{Context, Modifiers, Pixels, Point, point, px};

use super::LokView;

/// How often a drag held past the edge scrolls.
const TICK: Duration = Duration::from_millis(40);
/// The most one tick scrolls, in pixels.
const FASTEST: f32 = 120.0;

/// The drag's last pointer, and whether the ticks run.
#[derive(Default)]
pub(super) struct Edge {
    at: Option<(Point<Pixels>, Modifiers)>,
    ticking: bool,
}

impl LokView {
    /// The drag follows the pointer, in the view or out.
    pub(super) fn drag_to(&mut self, at: Point<Pixels>, modifiers: Modifiers, cx: &mut Context<Self>) {
        if self.carry_to(at, modifiers) {
            return cx.notify();
        }
        self.edge.at = Some((at, modifiers));
        let inside = self.inside(at);
        self.mouse(2, inside, 1, 1, &modifiers);
        if inside != at && !self.edge.ticking {
            log::info!("lok: a drag past the edge scrolls");
            self.edge.ticking = true;
            cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(TICK).await;
                    match this.update(cx, |view, cx| view.edge_tick(cx)) {
                        Ok(true) => {}
                        _ => break,
                    }
                }
            })
            .detach();
        }
    }

    /// One scroll toward the pointer; false ends the ticks.
    fn edge_tick(&mut self, cx: &mut Context<Self>) -> bool {
        let held = self.edge.at.filter(|_| self.dragging);
        let Some((at, modifiers)) = held.filter(|(at, _)| self.inside(*at) != *at) else {
            self.edge.ticking = false;
            return false;
        };
        let inside = self.inside(at);
        // Farther out, faster.
        let step = |by: Pixels| f32::from(by).clamp(-FASTEST, FASTEST);
        self.scroll_by(step(inside.x - at.x), step(inside.y - at.y), cx);
        self.mouse(2, inside, 1, 1, &modifiers);
        true
    }

    /// `at` pulled a pixel inside the view.
    fn inside(&self, at: Point<Pixels>) -> Point<Pixels> {
        let (origin, (width, height)) = (self.origin, self.viewport);
        let right = origin.x + px(width - 1.0).max(px(0.));
        let bottom = origin.y + px(height - 1.0).max(px(0.));
        point(at.x.clamp(origin.x, right), at.y.clamp(origin.y, bottom))
    }

    /// The drag is over: no more ticks.
    pub(super) fn end_drag(&mut self) {
        self.dragging = false;
        self.edge.at = None;
    }
}
