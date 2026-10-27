//! A test text system that keeps every string laid out.

use std::{borrow::Cow, collections::BTreeSet, sync::Mutex};

use gpui::{
    Bounds, DevicePixels, Font, FontId, FontMetrics, FontRun, GlyphId, LineLayout, NoopTextSystem,
    Pixels, PlatformTextSystem, RenderGlyphParams, Size, TextRenderingMode,
};

/// Gpui's test text, keeping each string it lays out.
#[derive(Default)]
pub struct LaidOut(Mutex<Vec<String>>);

impl LaidOut {
    /// Forgets what was laid out so far.
    pub fn clear(&self) {
        self.0.lock().expect("the laid-out texts").clear();
    }

    /// Whether some laid-out string holds `text`.
    pub fn saw(&self, text: &str) -> bool {
        let texts = self.0.lock().expect("the laid-out texts");
        texts.iter().any(|laid| laid.contains(text))
    }

    /// Latin words of two or more letters, less `allowed`.
    pub fn english(&self, allowed: &[&str]) -> BTreeSet<String> {
        let texts = self.0.lock().expect("the laid-out texts");
        texts
            .iter()
            .flat_map(|text| text.split(|ch: char| !ch.is_ascii_alphabetic()))
            .filter(|word| word.len() > 1 && !allowed.contains(word))
            .map(str::to_string)
            .collect()
    }
}

impl PlatformTextSystem for LaidOut {
    fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> gpui::Result<()> {
        NoopTextSystem.add_fonts(fonts)
    }

    fn all_font_names(&self) -> Vec<String> {
        NoopTextSystem.all_font_names()
    }

    fn font_id(&self, descriptor: &Font) -> gpui::Result<FontId> {
        NoopTextSystem.font_id(descriptor)
    }

    fn font_metrics(&self, font_id: FontId) -> FontMetrics {
        NoopTextSystem.font_metrics(font_id)
    }

    fn typographic_bounds(&self, font_id: FontId, glyph_id: GlyphId) -> gpui::Result<Bounds<f32>> {
        NoopTextSystem.typographic_bounds(font_id, glyph_id)
    }

    fn advance(&self, font_id: FontId, glyph_id: GlyphId) -> gpui::Result<Size<f32>> {
        NoopTextSystem.advance(font_id, glyph_id)
    }

    fn glyph_for_char(&self, font_id: FontId, ch: char) -> Option<GlyphId> {
        NoopTextSystem.glyph_for_char(font_id, ch)
    }

    fn glyph_raster_bounds(&self, params: &RenderGlyphParams) -> gpui::Result<Bounds<DevicePixels>> {
        NoopTextSystem.glyph_raster_bounds(params)
    }

    fn rasterize_glyph(
        &self,
        params: &RenderGlyphParams,
        raster_bounds: Bounds<DevicePixels>,
    ) -> gpui::Result<(Size<DevicePixels>, Vec<u8>)> {
        NoopTextSystem.rasterize_glyph(params, raster_bounds)
    }

    fn layout_line(&self, text: &str, font_size: Pixels, runs: &[FontRun]) -> LineLayout {
        self.0.lock().expect("the laid-out texts").push(text.to_string());
        NoopTextSystem.layout_line(text, font_size, runs)
    }

    fn recommended_rendering_mode(&self, font_id: FontId, font_size: Pixels) -> TextRenderingMode {
        NoopTextSystem.recommended_rendering_mode(font_id, font_size)
    }
}
