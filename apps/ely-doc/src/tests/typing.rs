//! The text group's case rows and the Insert Symbol dialog.

use ely_office::prelude::*;
use gpui::TestAppContext;

use crate::refs::{Ask, RemoveWatermark};

use crate::actions::Dialog;
use crate::symbols::InsertSymbol;
use crate::tests::{blank, docs, settled, text_of, type_into, with_window, written};

#[test]
fn case_rows_and_a_symbol_change_the_saved_text() {
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
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_symbol(&InsertSymbol, window, cx);
        assert!(matches!(docs.dialog, Some(Dialog::Symbols)), "the menu opens the grid");
        docs.type_symbol('€', cx);
        assert!(docs.dialog.is_none(), "a pick closes the grid");
    });
    settled(&cx, &docs);
    let path = dir.path().join("symbol.docx");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
    written(&cx, &path, None);
    assert!(text_of(&mut cx, &docs, &path).contains('€'), "the symbol is in the file");
    cx.update(|app| {
        docs.update(app, |docs, cx| {
            docs.run_active("draw.shape", &["ellipse", "true"], cx);
            docs.run_active("draw.text_box", &["true"], cx);
        })
    });
    settled(&cx, &docs);
    let path = dir.path().join("objects.odt");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
    written(&cx, &path, None);
    let flat = text_of(&mut cx, &docs, &path);
    assert!(flat.contains("draw:custom-shape"), "the shape is drawn: {}", flat.len());
    assert!(flat.contains("draw:text-box"), "the text box is drawn");
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

#[test]
fn a_watermark_goes_into_the_page_and_comes_out() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "body");
    let mut saved = |cx: &mut TestAppContext, name: &str| {
        settled(cx, &docs);
        let path = dir.path().join(format!("{name}.odt"));
        with_window(cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
        written(cx, &path, None);
        text_of(cx, &docs, &path)
    };
    cx.update(|app| docs.update(app, |docs, cx| docs.answer(&Ask::Watermark, "DRAFT", cx)));
    let marked = saved(&mut cx, "marked");
    assert!(marked.contains("PowerPlusWaterMarkObject") && marked.contains("DRAFT"), "the mark");
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_remove_watermark(&RemoveWatermark, window, cx));
    assert!(!saved(&mut cx, "clean").contains("PowerPlusWaterMarkObject"), "the mark is gone");
}
