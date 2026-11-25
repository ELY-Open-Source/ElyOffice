//! The dashed outline around copied cells (TABLE-08).

use gpui::{Bounds, Hsla, Pixels, Window, fill, point, px, size};

use super::{LokView, input::Input};
use crate::Rect;

/// VCL's Escape key code.
const ESCAPE: i32 = 1281;

/// Dash and gap length, and line width, in pixels.
const DASH: f32 = 4.0;
const WIDTH: f32 = 2.0;

/// The cells a copy or cut left on the clipboard.
pub(super) struct Ants {
    /// The copied range in document twips.
    pub(super) range: Rect,
    /// How many edits had landed; one more ends the dashes.
    pub(super) edited: u64,
    /// A cut, whose paste ends the dashes.
    pub(super) cut: bool,
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
        });
    }

    /// Escape ends the dashes; a paste ends a cut's only.
    pub(super) fn see_ants(&mut self, input: &Input) {
        match input {
            Input::Copy { cut } => self.cutting = *cut,
            Input::Key { key_code, .. } if key_code & 0x0FFF == ESCAPE => self.ants = None,
            Input::Paste | Input::PasteOnly(_) => match self.ants.as_mut() {
                Some(ants) if !ants.cut => ants.edited += 1,
                _ => self.ants = None,
            },
            _ => {}
        }
    }

    /// The copied range on screen, until an edit.
    pub(super) fn ants(&mut self) -> Option<Bounds<Pixels>> {
        if self
            .ants
            .as_ref()
            .is_some_and(|ants| self.edited > ants.edited)
        {
            self.ants = None;
        }
        let ants = self.ants.as_ref()?;
        Some(self.screen_rect(ants.range))
    }
}

/// Paints still dashes along the edge of `bounds`.
pub(super) fn paint(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    color: Hsla,
    ground: Hsla,
) {
    let (x, y) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    // Four sides clockwise: start, direction, length.
    let sides = [
        ((x, y), (1.0, 0.0), w),
        ((x + w, y), (0.0, 1.0), h),
        ((x + w, y + h), (-1.0, 0.0), w),
        ((x, y + h), (0.0, -1.0), h),
    ];
    let mut walked = 0.0;
    for ((sx, sy), (dx, dy), length) in sides {
        // A light line first, for dark cells.
        let thin = |from: f32, to: f32| {
            let (a, b) = (from.min(to), from.max(to));
            let (px0, py0) = (sx + dx * a, sy + dy * a);
            let (pw, ph) = (
                dx.abs() * (b - a) + dy.abs() * WIDTH,
                dy.abs() * (b - a) + dx.abs() * WIDTH,
            );
            // Inside the range, as the cell cursor.
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
        let mut at = -(walked % (2.0 * DASH));
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
