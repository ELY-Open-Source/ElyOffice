use ely_office::prelude::*;
use gpui::TestAppContext;

use crate::tests::{blank, docs, settle, settled, type_into, with_window, written};

#[test]
fn a_document_exports_as_markdown() {
    // Exports ask ElyMark, under the test data root.
    ely_platform::test_root();
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
    // Exports ask ElyMark, under the test data root.
    ely_platform::test_root();
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

#[test]
fn a_failed_markdown_export_says_markdown() {
    // Exports ask ElyMark, under the test data root.
    ely_platform::test_root();
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let root = super::words::rooted(&mut cx, &docs);
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "lost");
    settled(&cx, &docs);
    // No such folder, so the kernel cannot write it.
    let path = dir.path().join("missing").join("notes.md");
    with_window(&mut cx, &docs, |docs, _, cx| docs.write_markdown(id, path.clone(), cx));
    settled(&cx, &docs);
    let mark = "\u{1}";
    let said = cx.update(|app| ely_ui::i18n::text(app, "doc.output.markdown_failed", &[("error", mark)]).to_string());
    let (before, after) = said.split_once(mark).expect("the message holds the error");
    settle(&cx, |app| root.read(app).toasts(app).iter().any(|(text, _)| text.starts_with(before) && text.ends_with(after)));
    assert!(!path.exists(), "nothing was written");
}

#[test]
fn a_file_open_in_elymark_is_not_overwritten() {
    use ely_platform::{AppId, instance};
    // Exports ask ElyMark, under the test data root.
    ely_platform::test_root();
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("notes.md");
    std::fs::write(&path, "# kept in ElyMark\n").expect("writes");
    // This process stands in for an ElyMark showing it.
    let shown = path.display().to_string();
    let door = match instance::hold(AppId::Mark, &instance::Launch::default()).expect("takes ElyMark's lock") {
        instance::Start::First(door) => door,
        instance::Start::Handed => panic!("another ElyMark holds this test's data root"),
    };
    let reply = move |request: String| match request.as_str() {
        r#"["docs"]"# => Ok(shown.clone()),
        other => Err(format!("only docs is asked, not {other}")),
    };
    door.serve(|_| false, reply).expect("serves");
    let root = super::words::rooted(&mut cx, &docs);
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "lost");
    settled(&cx, &docs);
    with_window(&mut cx, &docs, |docs, _, cx| docs.write_markdown(id, path.clone(), cx));
    let said = cx.update(|app| ely_ui::i18n::text(app, "doc.output.open", &[("name", "notes.md")]).to_string());
    settle(&cx, |app| root.read(app).toasts(app).iter().any(|(text, _)| *text == said));
    settled(&cx, &docs);
    assert_eq!(std::fs::read_to_string(&path).expect("reads"), "# kept in ElyMark\n");
}
