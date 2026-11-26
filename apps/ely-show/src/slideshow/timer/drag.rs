//! Dragging a countdown over its slide in the editor.

use std::{cell::Cell, rc::Rc};

use gpui::{
    Bounds, Context, IntoElement, Pixels, Point, Render, Size, Styled as _, Window, div, point,
};

/// Room at each side, as digits differ in width.
const SPARE: f32 = 0.015;

/// A countdown held by the mouse.
#[derive(Clone, Default)]
pub struct Drag {
    /// Where the clock was last drawn.
    pub drawn: Rc<Cell<Bounds<Pixels>>>,
    /// From the clock's middle to the pointer.
    grab: Rc<Cell<Point<Pixels>>>,
}

impl Drag {
    /// The drag starts, the pointer `offset` into the clock.
    pub fn hold(&self, offset: Point<Pixels>) {
        let Size { width, height } = self.drawn.get().size;
        self.grab.set(offset - point(width / 2.0, height / 2.0));
    }

    /// The clock's middle, as shares of `slide`, kept inside it.
    pub fn middle(&self, pointer: Point<Pixels>, slide: Bounds<Pixels>) -> (f32, f32) {
        let held = pointer - self.grab.get() - slide.origin;
        let clock = self.drawn.get().size;
        let share = |at: Pixels, clock: Pixels, slide: Pixels, spare: f32| {
            let half = (clock / slide / 2.0 + spare).min(0.5);
            (at / slide).clamp(half, 1.0 - half)
        };
        (
            share(held.x, clock.width, slide.size.width, SPARE),
            share(held.y, clock.height, slide.size.height, 0.0),
        )
    }
}

/// Nothing trails the pointer; the clock itself moves.
pub struct Ghost;

impl Render for Ghost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_0()
    }
}

#[cfg(test)]
mod tests {
    use gpui::{px, size};

    use super::*;

    #[test]
    fn a_dragged_clock_follows_its_grip_and_stays_on_the_slide() {
        let drag = Drag::default();
        drag.drawn.set(Bounds::new(point(px(0.0), px(0.0)), size(px(200.0), px(100.0))));
        drag.hold(point(px(150.0), px(50.0)));
        let slide = Bounds::new(point(px(10.0), px(20.0)), size(px(1000.0), px(500.0)));
        assert_eq!(drag.middle(point(px(560.0), px(270.0)), slide), (0.5, 0.5));
        let (x, y) = drag.middle(point(px(-900.0), px(9000.0)), slide);
        assert!((x - 0.115).abs() < 1e-4 && (y - 0.9).abs() < 1e-4, "{x} {y}");
    }
}
