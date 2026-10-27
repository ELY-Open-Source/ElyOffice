//! Document and view units (LOK-04).

use crate::{TILE_TWIPS, TWIPS_PER_PIXEL};

/// Document/view conversion under one zoom and DPI (LOK-04).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geom {
    pub zoom: f32,
    pub dpi: f32,
}

impl Geom {
    /// Twips to logical pixels; GPUI scales to devices.
    pub fn px_of(&self, twips: i32) -> f32 {
        twips as f32 * self.zoom / TWIPS_PER_PIXEL
    }

    /// Logical pixels to twips.
    pub fn twips_of(&self, pixels: f32) -> i32 {
        (pixels * TWIPS_PER_PIXEL / self.zoom).round() as i32
    }

    /// Texture pixels for one tile edge at this DPI.
    pub(super) fn tile_pixels(&self) -> u32 {
        (self.px_of(TILE_TWIPS) * self.dpi).round().max(1.0) as u32
    }
}
