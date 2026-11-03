//! The selected object's frame and its eight handles (DOC-08).

use gpui::{Pixels, Point};

use super::LokView;
use crate::Rect;

/// A held handle: which, from where, to where.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Grip {
    pub(super) at: usize,
    pub(super) from: (i32, i32),
    pub(super) to: (i32, i32),
}

/// Corners and edge middles, left to right, top to bottom.
pub(super) fn points((x, y, w, h): Rect) -> [(i32, i32); 8] {
    let (cx, cy, r, b) = (x + w / 2, y + h / 2, x + w, y + h);
    [
        (x, y),
        (cx, y),
        (r, y),
        (x, cy),
        (r, cy),
        (x, b),
        (cx, b),
        (r, b),
    ]
}

/// The frame with handle `at` moved to `to`.
pub(super) fn dragged((x, y, w, h): Rect, at: usize, (tx, ty): (i32, i32)) -> Rect {
    let (mut left, mut top, mut right, mut bottom) = (x, y, x + w, y + h);
    if matches!(at, 0 | 3 | 5) {
        left = tx;
    }
    if matches!(at, 2 | 4 | 7) {
        right = tx;
    }
    if matches!(at, 0..=2) {
        top = ty;
    }
    if matches!(at, 5..=7) {
        bottom = ty;
    }
    (
        left.min(right),
        top.min(bottom),
        (right - left).abs(),
        (bottom - top).abs(),
    )
}

impl LokView {
    /// Grabs the handle under `position`, if a frame is selected.
    pub(super) fn grab(&mut self, position: Point<Pixels>) -> bool {
        let Some(frame) = self.graphic else {
            return false;
        };
        let reach = self.handle_size;
        let hit = points(frame).iter().position(|&point| {
            let at = self.hit_point(point);
            (at.x - position.x).abs() <= reach && (at.y - position.y).abs() <= reach
        });
        let Some(at) = hit else {
            return false;
        };
        let from = points(frame)[at];
        log::info!("lok: handle {at} grabbed");
        self.grip = Some(Grip { at, from, to: from });
        true
    }

    /// The frame to paint, previewing a held handle.
    pub(super) fn frame(&self) -> Option<Rect> {
        let frame = self.graphic?;
        Some(match self.grip {
            Some(grip) => dragged(frame, grip.at, grip.to),
            None => frame,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_move_their_own_sides() {
        let frame = (100, 200, 300, 400);
        assert_eq!(points(frame)[7], (400, 600));
        assert_eq!(
            dragged(frame, 7, (300, 500)),
            (100, 200, 200, 300),
            "a corner moves two sides"
        );
        assert_eq!(
            dragged(frame, 4, (500, 0)),
            (100, 200, 400, 400),
            "an edge moves one"
        );
        assert_eq!(
            dragged(frame, 0, (500, 700)),
            (400, 600, 100, 100),
            "past the far side flips"
        );
    }
}
