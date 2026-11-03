//! An EPUB book opens read-only in a tab (#46).

use gpui::TestAppContext;

use super::{frames, settle, shown, windowed};
use crate::viewer::Viewer;

#[test]
fn a_book_opens_and_a_change_says_it_only_reads() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let book = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/epub/sample.epub");
    let mut cx = TestAppContext::single();
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[book], true);
    let view = shown(cx, &viewer);
    assert!(cx.update(|_, app| view.read(app).sizes.len()) >= 3, "every page laid out");
    // The second chapter's bookmark jumps to its page.
    let mut marks = Vec::new();
    cx.update(|_, app| crate::marks::flat(&view.read(app).marks, &[], &mut marks));
    let (page, y) = marks[1].to.expect("a page to go to");
    assert!(page >= 1, "the second chapter starts later");
    cx.update(|_, app| view.update(app, |view, cx| view.show(page, y, cx)));
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).current), page, "the jump lands");
    // The window's toasts live on its root.
    let root = cx.update(|window, app| {
        let root = ely_ui::document_root(ely_platform::AppId::Pdf, viewer.clone().into(), &ely_ui::commands::FILE, Vec::new(), window, app);
        viewer.update(app, |viewer, _| viewer.core.root = root.downgrade());
        root
    });
    cx.update(|_, app| view.update(app, |view, cx| view.edit("pdf.doing.delete_pages", |worker, doc| worker.delete_pages(doc, &[0]), cx)));
    let said = cx.update(|_, app| Viewer::text(app, "pdf.epub.read_only", &[]));
    settle(cx, |app| !root.read(app).toasts(app).is_empty());
    let toasts = cx.update(|_, app| root.read(app).toasts(app));
    assert!(toasts.iter().any(|(text, _)| *text == said), "{toasts:?}");
    assert_eq!(cx.update(|_, app| view.read(app).edited), 0, "nothing changed");
}
