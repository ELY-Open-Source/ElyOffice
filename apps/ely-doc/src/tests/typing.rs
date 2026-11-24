//! Case rows, symbols, drawn objects and paragraph shading.

use ely_office::prelude::*;
use gpui::TestAppContext;

use crate::actions::Dialog;
use crate::docs::Docs;
use crate::objects::{InsertEllipse, InsertRectangle, InsertTextBox, InsertTriangle};
use crate::symbols::InsertSymbol;
use crate::tests::words::rooted;
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
        ("format.case_sentence", "Hello world"),
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
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_rectangle(&InsertRectangle, window, cx);
        docs.on_ellipse(&InsertEllipse, window, cx);
        docs.on_triangle(&InsertTriangle, window, cx);
        docs.on_text_box(&InsertTextBox, window, cx);
    });
    settled(&cx, &docs);
    let path = dir.path().join("objects.odt");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
    written(&cx, &path, None);
    let flat = text_of(&mut cx, &docs, &path);
    for kind in ["rectangle", "ellipse", "isosceles-triangle"] {
        let drawn = flat.matches(&format!("draw:type=\"{kind}\"")).count();
        assert_eq!(drawn, 1, "one {kind} is drawn");
    }
    assert!(flat.contains("draw:text-box"), "the text box is drawn");
}

#[test]
fn a_symbol_without_a_document_says_so() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let root = rooted(&mut cx, &docs);
    let said = cx.update(|app| Docs::text(app, "command.blocked.no_document", &[]));
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_symbol(&InsertSymbol, window, cx));
    assert!(cx.update(|app| docs.read(app).dialog.is_none()), "no grid opens");
    // A pick after the last tab closed.
    with_window(&mut cx, &docs, |docs, _, cx| docs.type_symbol('€', cx));
    let toasts = cx.update(|app| root.read(app).toasts(app));
    assert_eq!(toasts.iter().filter(|(text, _)| *text == said).count(), 2, "{toasts:?}");
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
    let flat = |cx: &mut TestAppContext, name: &str, value: &str| {
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
    let red = flat(&mut cx, "red", "16711680");
    let carry = |xml: &str, tag: &str, mark: &str| {
        xml.split(tag).skip(1).any(|it| it.split('>').next().is_some_and(|props| props.contains(mark)))
    };
    assert!(carry(&red, "<style:paragraph-properties", "#ff0000"), "the paragraph is shaded");
    assert!(!carry(&red, "<style:text-properties", "background-color"), "no letters are highlighted");
    assert!(!flat(&mut cx, "none", "-1").contains("#ff0000"), "the shading is gone");
}
