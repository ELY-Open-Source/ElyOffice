//! The body font picker, driven as a user does.

use std::{borrow::Cow, sync::Arc};

use gpui::{
    Bounds, DevicePixels, Font, FontId, FontMetrics, FontRun, GlyphId, LineLayout, NoopTextSystem, Pixels,
    PlatformTextSystem, RenderGlyphParams, Result, Size, TestAppContext, TestDispatcher, TextRenderingMode,
};

use crate::desk::tests::{open, start};
use crate::view::Mode;

/// No glyphs, as tests have; three families to pick.
struct Families(NoopTextSystem);

impl PlatformTextSystem for Families {
    fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        self.0.add_fonts(fonts)
    }
    fn all_font_names(&self) -> Vec<String> {
        ["Arial", "Consolas", "Georgia"].map(String::from).to_vec()
    }
    fn font_id(&self, descriptor: &Font) -> Result<FontId> {
        self.0.font_id(descriptor)
    }
    fn font_metrics(&self, font_id: FontId) -> FontMetrics {
        self.0.font_metrics(font_id)
    }
    fn typographic_bounds(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Bounds<f32>> {
        self.0.typographic_bounds(font_id, glyph_id)
    }
    fn advance(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Size<f32>> {
        self.0.advance(font_id, glyph_id)
    }
    fn glyph_for_char(&self, font_id: FontId, ch: char) -> Option<GlyphId> {
        self.0.glyph_for_char(font_id, ch)
    }
    fn glyph_raster_bounds(&self, params: &RenderGlyphParams) -> Result<Bounds<DevicePixels>> {
        self.0.glyph_raster_bounds(params)
    }
    fn rasterize_glyph(&self, params: &RenderGlyphParams, bounds: Bounds<DevicePixels>) -> Result<(Size<DevicePixels>, Vec<u8>)> {
        self.0.rasterize_glyph(params, bounds)
    }
    fn layout_line(&self, text: &str, font_size: Pixels, runs: &[FontRun]) -> LineLayout {
        self.0.layout_line(text, font_size, runs)
    }
    fn recommended_rendering_mode(&self, font_id: FontId, font_size: Pixels) -> TextRenderingMode {
        self.0.recommended_rendering_mode(font_id, font_size)
    }
}

#[test]
fn a_typed_family_is_picked_kept_and_typing_goes_on() {
    let mut cx = TestAppContext::build_with_text_system(TestDispatcher::new(0), None, Arc::new(Families(NoopTextSystem)));
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let path = files.path().join("a.md");
    std::fs::write(&path, "a\n").expect("a file");
    let (desk, cx) = start(&mut cx, data.path());
    open(&desk, &path, cx);
    let view = desk.read_with(cx, |desk, _| desk.active_view().expect("a tab"));
    view.update_in(cx, |view, window, cx| view.switch(Mode::Source, window, cx));
    let text = |cx: &mut gpui::VisualTestContext| view.read_with(cx, |view, cx| view.document().read(cx).text(cx));
    cx.simulate_input("b");
    assert_eq!(text(cx), "ba\n");
    cx.dispatch_action(super::OtherFont);
    cx.simulate_keystrokes("tab");
    cx.simulate_input("Cons");
    cx.simulate_keystrokes("enter");
    assert!(desk.read_with(cx, |desk, _| desk.font_picking.is_none()), "a pick closes the dialog");
    let shown = view.read_with(cx, |view, cx| view.document().read(cx).look().font.clone());
    assert_eq!(shown.as_deref(), Some("Consolas"));
    let kept = super::load(&data.path().join("look.json")).expect("kept");
    assert_eq!(kept.font, super::Font::Named("Consolas".into()));
    cx.simulate_input("x");
    assert_eq!(text(cx), "bxa\n", "typing reaches the document again");
}
