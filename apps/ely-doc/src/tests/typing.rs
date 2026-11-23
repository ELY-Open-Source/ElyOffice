//! The text group's case rows and the paragraph shading.

use ely_office::prelude::*;
use gpui::TestAppContext;

use crate::tests::{blank, docs, settled, text_of, type_into, with_window, written};

#[test]
fn case_rows_change_the_saved_text() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "hello world");
    for (command, shown) in [
        ("format.case_upper", "HELLO WORLD"),
        ("format.case_title", "Hello World"),
        ("format.case_lower", "hello world"),
    ] {
        cx.update(|app| {
            docs.update(app, |docs, cx| {
                docs.run_active("edit.select_all", &[], cx);
                docs.run_active(command, &[], cx);
            })
        });
        settled(&cx, &docs);
        let path = dir.path().join(format!("{command}.docx"));
        with_window(&mut cx, &docs, |docs, window, cx| {
            docs.write_as(id, path.clone(), None, window, cx)
        });
        written(&cx, &path, None);
        assert!(text_of(&mut cx, &docs, &path).contains(shown), "{command}");
    }
}

#[test]
fn a_paragraph_takes_shading() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "shaded");
    let mut flat = |cx: &mut TestAppContext, name: &str, value: &str| {
        cx.update(|app| {
            docs.update(app, |docs, cx| {
                docs.run_active("edit.select_all", &[], cx);
                docs.run_active("paragraph.shading", &[value], cx);
            })
        });
        settled(cx, &docs);
        let path = dir.path().join(format!("{name}.odt"));
        with_window(cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
        written(cx, &path, None);
        text_of(cx, &docs, &path)
    };
    assert!(flat(&mut cx, "red", "16711680").contains("fo:background-color=\"#ff0000\""), "the shading");
    assert!(!flat(&mut cx, "none", "-1").contains("fo:background-color=\"#ff0000\""), "the shading is gone");
}
