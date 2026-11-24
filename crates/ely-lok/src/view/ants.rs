//! The marching dashes around copied cells, as Excel shows them (TABLE-08).

use std::time::Instant;

use gpui::{Bounds, Hsla, Pixels, Window, fill, point, px, size};

use super::LokView;
use crate::Rect;

/// Dash and gap length, and line width, in pixels.
const DASH: f32 = 4.0;
const WIDTH: f32 = 2.0;
/// Pixels the dashes march each second.
const SPEED: f32 = 16.0;

/// The cells a copy or cut left on the clipboard.
pub(super) struct Ants {
    /// The copied range in document twips.
    pub(super) range: Rect,
    /// How many edits had landed; one more ends the dashes.
    pub(super) edited: u64,
    /// A cut's dashes end with its paste; a copy's go on.
    pub(super) cut: bool,
    pub(super) since: Instant,
}

/// The rect holding all of `rects`.
fn union(rects: impl IntoIterator<Item = Rect>) -> Option<Rect> {
    rects.into_iter().fold(None, |all, (x, y, w, h)| match all {
        None => Some((x, y, w, h)),
        Some((ax, ay, aw, ah)) => {
            let (left, top) = (ax.min(x), ay.min(y));
            let (right, bottom) = ((ax + aw).max(x + w), (ay + ah).max(y + h));
            Some((left, top, right - left, bottom - top))
        }
    })
}

impl LokView {
    /// Starts the dashes around the selected cells.
    pub(super) fn light_ants(&mut self) {
        if !self.doc.is_sheet() {
            return;
        }
        let range = union(self.selection.iter().copied().chain(self.cell.cursor));
        self.ants = range.map(|range| Ants {
            range,
            edited: self.edited,
            cut: self.cutting,
            since: Instant::now(),
        });
    }

    /// A paste is queued: it ends a cut's dashes and spares a copy's.
    pub(super) fn paste_ants(&mut self) {
        match self.ants.as_mut() {
            Some(ants) if !ants.cut => ants.edited += 1,
            _ => self.ants = None,
        }
    }

    /// Puts the dashes out, as Escape does.
    pub(super) fn put_out_ants(&mut self) {
        self.ants = None;
    }

    /// The copied range on screen; ends once the sheet is edited.
    pub(super) fn ants(&mut self) -> Option<(Bounds<Pixels>, f32)> {
        if self
            .ants
            .as_ref()
            .is_some_and(|ants| self.edited > ants.edited)
        {
            self.ants = None;
        }
        let ants = self.ants.as_ref()?;
        let phase = ants.since.elapsed().as_secs_f32() * SPEED;
        Some((self.screen_rect(ants.range), phase))
    }
}

/// Paints the dashes along the edge of `bounds`; `phase` moves them.
pub(super) fn paint(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    phase: f32,
    color: Hsla,
    ground: Hsla,
) {
    let (x, y) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    // Four sides, clockwise from the top left, as a start and a direction.
    let sides = [
        ((x, y), (1.0, 0.0), w),
        ((x + w, y), (0.0, 1.0), h),
        ((x + w, y + h), (-1.0, 0.0), w),
        ((x, y + h), (0.0, -1.0), h),
    ];
    let mut walked = 0.0;
    for ((sx, sy), (dx, dy), length) in sides {
        // A light line first so dashes read on dark cells too.
        let thin = |from: f32, to: f32| {
            let (a, b) = (from.min(to), from.max(to));
            let (px0, py0) = (sx + dx * a, sy + dy * a);
            let (pw, ph) = (
                dx.abs() * (b - a) + dy.abs() * WIDTH,
                dy.abs() * (b - a) + dx.abs() * WIDTH,
            );
            // Keep the line inside the range, as the cell cursor does.
            let (inset_x, inset_y) = (
                if dy > 0.0 { -WIDTH } else { 0.0 },
                if dx < 0.0 { -WIDTH } else { 0.0 },
            );
            Bounds {
                origin: point(px(px0 + inset_x), px(py0 + inset_y)),
                size: size(px(pw), px(ph)),
            }
        };
        window.paint_quad(fill(thin(0.0, length), ground));
        let mut at = -((walked + phase) % (2.0 * DASH));
        while at < length {
            let (from, to) = (at.max(0.0), (at + DASH).min(length));
            if to > from {
                window.paint_quad(fill(thin(from, to), color));
            }
            at += 2.0 * DASH;
        }
        walked += length;
    }
}

#[cfg(test)]
mod tests {
    use super::union;

    #[test]
    fn the_range_holds_every_selected_rect() {
        assert_eq!(union([]), None);
        assert_eq!(
            union([(100, 100, 50, 20), (150, 120, 60, 40)]),
            Some((100, 100, 110, 60))
        );
    }
}
