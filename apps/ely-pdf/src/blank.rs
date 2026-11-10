//! New: one blank page on the chosen paper (#80).

use ely_office::prelude::*;
use gpui::Context;

use crate::viewer::Viewer;

/// Points: A4 is 210 by 297 mm.
const A4: (&str, &str) = ("595.276", "841.89");
const LETTER: (&str, &str) = ("612", "792");

/// A PDF of one empty page, `width` by `height` points.
fn blank((width, height): (&str, &str)) -> String {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Resources << >> >>"),
    ];
    let mut pdf = String::from("%PDF-1.7\n");
    let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for (index, object) in objects.iter().enumerate() {
        // Each entry is the object's byte offset.
        table += &format!("{:010} 00000 n \n", pdf.len());
        pdf += &format!("{} 0 obj\n{object}\nendobj\n", index + 1);
    }
    let start = pdf.len();
    pdf + &table + &format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n", objects.len() + 1)
}

impl Viewer {
    /// A new untitled PDF of one blank page.
    pub fn new_blank(&mut self, cx: &mut Context<Self>) {
        let (paper, region) = (ely_ui::settings::current(cx).paper, ely_platform::locale::region());
        let letter = paper.letter(region.as_deref());
        log::info!("pdf: new page {paper:?}, region {region:?}, letter {letter}");
        self.new_from_text("blank.pdf", &blank(if letter { LETTER } else { A4 }), cx);
    }
}

#[cfg(test)]
mod tests {
    use ely_platform::settings::Paper;
    use gpui::TestAppContext;

    use super::*;
    use crate::tests::{settle, shown, windowed};

    #[test]
    fn the_table_points_at_each_object() {
        let pdf = blank(LETTER);
        let start: usize = pdf.rsplit("startxref\n").next().and_then(|rest| rest.lines().next()).and_then(|n| n.parse().ok()).expect("a start");
        assert!(pdf[start..].starts_with("xref\n0 4\n"), "startxref misses the table");
        let entries = pdf[start..].lines().skip(3).take(3);
        for (index, entry) in entries.enumerate() {
            let at: usize = entry[..10].parse().expect("an offset");
            assert!(pdf[at..].starts_with(&format!("{} 0 obj\n", index + 1)), "entry {index} points elsewhere");
        }
    }

    #[test]
    fn new_opens_an_untitled_page_that_saves_as_a_pdf() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let mut cx = TestAppContext::single();
        let (viewer, cx) = windowed(&mut cx, dir.path(), &[], true);
        for (paper, size) in [(Paper::A4, (595.276, 841.89)), (Paper::Letter, (612.0, 792.0))] {
            let count = cx.update(|_, app| viewer.read(app).core.tabs.len());
            cx.update(|window, app| {
                app.global_mut::<ely_ui::settings::Current>().shared.paper = paper;
                // As the File menu's New does.
                let focus = viewer.read(app).core.focus.clone();
                focus.focus(window, app);
                window.dispatch_action(Box::new(ely_ui::commands::New), app);
            });
            settle(cx, |app| viewer.read(app).core.tabs.len() == count + 1);
            let view = shown(cx, &viewer);
            let (sizes, repair) = cx.update(|_, app| (view.read(app).sizes.clone(), view.read(app).repair.clone()));
            assert_eq!(sizes, [size], "{paper:?}");
            assert_eq!(repair, None, "the blank page needed repair");
        }
        let tab = cx.update(|_, app| viewer.read(app).active_tab().map(|tab| (tab.id, tab.session.path().is_none(), tab.registry))).expect("a tab");
        assert_eq!((tab.1, tab.2), (true, None), "the new page is no one's file yet");
        let saved = dir.path().join("saved.pdf");
        cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.write_as(tab.0, saved.clone(), None, window, cx)));
        settle(cx, |app| saved.is_file() && viewer.read(app).active_tab().is_some_and(|tab| tab.session.path() == Some(saved.as_path())));
        assert!(std::fs::read(&saved).expect("reads").starts_with(b"%PDF-"), "the saved file is no PDF");
    }
}
