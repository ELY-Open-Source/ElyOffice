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

#[test]
fn a_file_of_another_kind_says_so_in_chinese_and_spanish() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let note = dir.path().join("note.txt");
    std::fs::write(&note, "plain text").expect("writes");
    let mut cx = TestAppContext::single();
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[], true);
    // The window's toasts live on its root.
    let root = cx.update(|window, app| {
        let root = ely_ui::document_root(ely_platform::AppId::Pdf, viewer.clone().into(), &ely_ui::commands::FILE, Vec::new(), window, app);
        viewer.update(app, |viewer, _| viewer.core.root = root.downgrade());
        root
    });
    for (locale, said) in [("zh-CN", "文件既不是 PDF 也不是 EPUB"), ("es-ES", "el archivo no es ni PDF ni EPUB")] {
        cx.update(|_, app| ely_ui::i18n::install_as(locale, app));
        cx.update(|_, app| viewer.update(app, |viewer, cx| ely_office::Opening::open_file(viewer, note.clone(), cx)));
        settle(cx, |app| root.read(app).toasts(app).iter().any(|(text, _)| text.contains(said)));
    }
    assert!(cx.update(|_, app| viewer.read(app).core.tabs.is_empty()), "no tab opened");
}

#[gpui::test]
fn every_key_the_worker_words_is_in_each_catalog(cx: &mut TestAppContext) {
    cx.update(|app| {
        ely_ui::settings::install_defaults(app);
        for locale in ["en-US", "zh-CN", "es-ES"] {
            ely_ui::i18n::install_as(locale, app);
            for key in ely_pdf_engine::proto::WORDED {
                assert_ne!(ely_ui::i18n::text(app, key, &[("report", "r")]), *key, "{locale} words {key}");
            }
        }
    });
}
