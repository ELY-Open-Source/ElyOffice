//! Another program reads, replaces and saves the text.

use std::cell::RefCell;

use ely_office::prelude::*;
use gpui::{Entity, TestAppContext};

use crate::docs::Docs;
use crate::tests::{blank, docs, settle, settled, text_of, type_into, with_window};

/// The answer to `words`, once it comes.
fn ask(cx: &mut TestAppContext, docs: &Entity<Docs>, words: &[&str]) -> Result<String, String> {
    let (ask, answered) = ely_ui::Ask::for_tests(&serde_json::json!(words).to_string());
    cx.update(|app| docs.update(app, |docs, cx| docs.answer_request(ask, cx)));
    let answer = RefCell::new(None);
    settle(cx, |_| {
        let mut got = answer.borrow_mut();
        if got.is_none() {
            *got = answered.try_recv().ok();
        }
        got.is_some()
    });
    answer.into_inner().expect("answered")
}

#[test]
fn text_reads_replaces_in_one_step_and_saves_with_the_cursor_kept() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "cat Cat 世界 😀 cat");
    let path = dir.path().join("doc.odt");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
    settle(&cx, |app| docs.read(app).core.tabs[0].name == "doc.odt");
    let file = path.display().to_string();
    let docs_listed = ask(&mut cx, &docs, &["docs"]).expect("docs");
    assert_eq!(std::fs::canonicalize(&docs_listed).ok(), std::fs::canonicalize(&path).ok(), "{docs_listed}");
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("cat Cat 世界 😀 cat".into()));
    assert_eq!(ask(&mut cx, &docs, &["replace", &file, "cat", "dog"]), Ok(String::new()));
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("dog Cat 世界 😀 dog".into()));
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("dog Cat 世界 😀 dog".into()));
    settled(&cx, &docs);
    cx.update(|app| {
        let tab = &docs.read(app).core.tabs[0];
        assert!(tab.dirty(app), "a read leaves the edit unsaved");
        // macOS keeps temp files behind /var's link.
        let kept = tab.session.path().and_then(|kept| std::fs::canonicalize(kept).ok());
        assert_eq!(kept, std::fs::canonicalize(&path).ok(), "a read keeps the file");
    });
    let missing = ask(&mut cx, &docs, &["replace", &file, "zebra", "x"]);
    assert!(missing.as_ref().is_err_and(|why| why.starts_with("nothing replaced")), "{missing:?}");
    assert!(ask(&mut cx, &docs, &["replace", &file, "dog", "a\nb"]).is_err(), "a break in WITH");
    let other = dir.path().join("other.odt");
    std::fs::write(&other, b"x").expect("a stray file");
    assert!(ask(&mut cx, &docs, &["get", &other.display().to_string()]).is_err(), "a file not open");
    // Typing lands where the cursor was, not on a match.
    type_into(&mut cx, &docs, "!");
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("dog Cat 世界 😀 dog!".into()));
    cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.undo", &[], cx)));
    settled(&cx, &docs);
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("dog Cat 世界 😀 dog".into()));
    cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.undo", &[], cx)));
    settled(&cx, &docs);
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("cat Cat 世界 😀 cat".into()), "one undo step");
    cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.redo", &[], cx)));
    settled(&cx, &docs);
    let saved = ask(&mut cx, &docs, &["save", &file]).expect("saved");
    assert_eq!(std::fs::canonicalize(&saved).ok(), std::fs::canonicalize(&path).ok());
    assert!(text_of(&mut cx, &docs, &path).contains("dog Cat 世界 😀 dog"), "on disk at the answer");
    assert_eq!(ask(&mut cx, &docs, &["replace", &file, " ", "_"]), Ok(String::new()));
    assert_eq!(ask(&mut cx, &docs, &["replace", &file, "_", "\t"]), Ok(String::new()));
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("dog\tCat\t世界\t😀\tdog".into()));
    assert_eq!(ask(&mut cx, &docs, &["replace", &file, "\t", " "]), Ok(String::new()));
    let second = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "second");
    let two = dir.path().join("two.odt");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(second, two.clone(), None, window, cx));
    settle(&cx, |app| docs.read(app).core.tabs[1].name == "two.odt");
    assert_eq!(ask(&mut cx, &docs, &["get", &two.display().to_string()]), Ok("second".into()));
    assert_eq!(ask(&mut cx, &docs, &["get", &file]), Ok("dog Cat 世界 😀 dog".into()), "each open document reads");
}
