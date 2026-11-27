//! Comparing with a copy that holds the same changes (#140).

use ely_office::Saving;
use gpui::TestAppContext;

use crate::review::TrackChanges;
use crate::tests::{blank, docs, settle, settled, type_into, with_window, written};

#[test]
fn an_identical_copy_with_the_same_changes_shows_no_difference() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let root = super::words::rooted(&mut cx, &docs);
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Kept words ");
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_track(&TrackChanges, window, cx));
    type_into(&mut cx, &docs, "tracked words");
    settled(&cx, &docs);
    let older = dir.path().join("older.odt");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, older.clone(), None, window, cx));
    written(&cx, &older, None);
    let before = std::fs::read(&older).expect("the older file reads");
    cx.update(|app| docs.update(app, |docs, cx| docs.compare(id, older.clone(), cx)));
    let said = cx.update(|app| ely_ui::i18n::text(app, "doc.compare.same", &[("name", "older.odt")]).to_string());
    settle(&cx, |app| root.read(app).toasts(app).iter().any(|(text, _)| *text == said));
    settle(&cx, |app| docs.read(app).core.tabs.len() == 1);
    assert_eq!(std::fs::read(&older).expect("reads"), before, "the older file and its change stay");
}
