//! Line height leaves out the font's gap; main thread (R22.1).
fn main() {
    #[cfg(any(windows, target_os = "macos"))]
    a_line_keeps_the_font_ascent_and_descent_without_its_gap();
}

#[cfg(any(windows, target_os = "macos"))]
fn a_line_keeps_the_font_ascent_and_descent_without_its_gap() {
    use std::borrow::Cow;

    use gpui::{SharedString, TestAppContext, TestDispatcher, TextRun, WindowTextSystem, font, px};

    let system = gpui_platform::current_platform(false).text_system();
    let cx = TestAppContext::build_with_text_system(TestDispatcher::new(0), None, system);
    let gap = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/ui/gap.ttf"))
        .expect("the gap fixture reads");
    cx.update(|cx| cx.text_system().add_fonts(vec![Cow::Owned(gap)]).expect("the gap font loads"));
    cx.update(|cx| {
        let fonts = cx.text_system().clone();
        let face = font("Ely Gap Sans");
        let id = fonts.resolve_font(&face);
        let size = px(20.);
        let text = "Regarding gjpqy";
        let run = TextRun {
            len: text.len(),
            font: face,
            color: gpui::black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line = WindowTextSystem::new(fonts.clone()).shape_line(SharedString::from(text), size, &[run], None);
        // A font's descent is below the baseline, so negative.
        let (ascent, descent) = (fonts.ascent(id, size), -fonts.descent(id, size));
        // Gap included, the line would be twenty pixels taller.
        assert!((line.ascent - ascent).abs() < px(0.1), "ascent {:?}, font {ascent:?}", line.ascent);
        assert!((line.descent - descent).abs() < px(0.1), "descent {:?}, font {descent:?}", line.descent);
    });
    println!("line_gap: 1 case passes");
}
