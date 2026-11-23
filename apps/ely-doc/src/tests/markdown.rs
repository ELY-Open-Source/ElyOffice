use ely_office::prelude::*;
use gpui::TestAppContext;

use crate::tests::{blank, docs, settle, settled, type_into, with_window, written};

#[test]
fn a_document_exports_as_markdown() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "plain words");
    settled(&cx, &docs);
    let path = dir.path().join("notes.md");
    with_window(&mut cx, &docs, |docs, _, cx| docs.write_markdown(id, path.clone(), cx));
    written(&cx, &path, None);
    let text = std::fs::read_to_string(&path).expect("reads");
    assert!(text.contains("plain words"), "{text}");
    // The export is a copy: the document stays open, untouched.
    settle(&cx, |app| docs.read(app).core.tabs.len() == 1);
}

#[test]
fn an_open_document_is_not_overwritten_by_the_export() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "kept");
    settled(&cx, &docs);
    let other = dir.path().join("other.odt");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, other.clone(), None, window, cx));
    written(&cx, &other, None);
    let before = std::fs::read(&other).expect("reads");
    with_window(&mut cx, &docs, |docs, _, cx| docs.write_markdown(id, other.clone(), cx));
    settled(&cx, &docs);
    assert_eq!(std::fs::read(&other).expect("reads"), before, "the open file is left alone");
}
