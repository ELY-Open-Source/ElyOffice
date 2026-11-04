use gpui::{Entity, TestAppContext};

use super::*;
use crate::tests::{blank, docs, settle, settled, text_of, type_into, with_window, written};

/// The open prompt's request and field.
fn asked(cx: &TestAppContext, docs: &Entity<Docs>) -> (Ask, Entity<TextInput>) {
    // Prompts open once their kernel read comes back.
    cx.run_until_parked();
    cx.update(|app| match &docs.read(app).dialog {
        Some(Dialog::Ask { ask, field }) => (ask.clone(), field.clone()),
        _ => panic!("a prompt is open"),
    })
}

fn answer(cx: &mut TestAppContext, docs: &Entity<Docs>, text: &str) {
    let (ask, _) = asked(cx, docs);
    cx.update(|app| docs.update(app, |docs, cx| docs.answer(&ask, text, cx)));
    settled(cx, docs);
}

fn graphic(app: &gpui::App, docs: &Entity<Docs>) -> bool {
    let tab = docs.read(app).active_tab().expect("a tab");
    tab.view.read(app).graphic().is_some()
}

#[test]
fn references_go_in_through_their_dialogs() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Start");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_reference(&InsertReference, window, cx)
    });
    cx.run_until_parked();
    assert!(
        cx.update(|app| docs.read(app).dialog.is_none()),
        "no bookmark, no cross-reference dialog"
    );
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_bookmark(&InsertBookmark, window, cx)
    });
    answer(&mut cx, &docs, "Here");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_bookmark(&InsertBookmark, window, cx)
    });
    let (ask, _) = asked(&cx, &docs);
    assert!(
        matches!(&ask, Ask::Bookmark(taken) if taken == &["Here"]),
        "the prompt knows the taken names"
    );
    cx.update(|app| docs.update(app, |docs, _| docs.dialog = None));
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_reference(&InsertReference, window, cx)
    });
    cx.run_until_parked();
    cx.update(|app| docs.update(app, |docs, cx| docs.insert_reference(cx)));
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_link(&InsertLink, window, cx)
    });
    cx.update(|app| {
        docs.update(app, |docs, cx| {
            let Some(Dialog::Link { url, .. }) = &docs.dialog else {
                panic!("the link dialog is open");
            };
            url.update(cx, |url, cx| url.set_text("https://example.org/", cx));
            docs.insert_link(cx);
        })
    });
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_footnote(&InsertFootnote, window, cx);
    });
    type_into(&mut cx, &docs, "Foot");
    cx.update(|app| {
        let view = docs.read(app).active_tab().expect("a tab").view.clone();
        view.update(app, |view, cx| view.reveal((1500, 1500, 0, 0), cx));
    });
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_contents(&InsertContents, window, cx);
        docs.on_update(&UpdateContents, window, cx);
    });
    settled(&cx, &docs);
    let path = dir.path().join("refs.odt");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, path.clone(), None, window, cx)
    });
    written(&cx, &path, None);
    let text = text_of(&mut cx, &docs, &path);
    for mark in [
        "<text:bookmark text:name=\"Here\"",
        "<text:bookmark-ref ",
        "xlink:href=\"https://example.org/\"",
        ">https://example.org/<",
        "<text:note ",
        "Foot",
        "<text:table-of-content ",
    ] {
        assert!(text.contains(mark), "{mark} lands");
    }
}

#[test]
fn formulas_edit_in_place_and_take_captions() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Ratio");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_caption(&InsertCaption, window, cx)
    });
    assert!(
        cx.update(|app| docs.read(app).dialog.is_none()),
        "nothing to caption"
    );
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_formula(&InsertFormula, window, cx)
    });
    assert!(matches!(asked(&cx, &docs).0, Ask::Formula(false)));
    answer(&mut cx, &docs, "a over b");
    settle(&cx, |app| graphic(app, &docs));
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_formula(&InsertFormula, window, cx)
    });
    let (ask, field) = asked(&cx, &docs);
    assert!(matches!(ask, Ask::Formula(true)), "the formula edits");
    assert_eq!(
        cx.update(|app| field.read(app).text().to_string()),
        "a over b"
    );
    answer(&mut cx, &docs, "c over d");
    settle(&cx, |app| graphic(app, &docs));
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_caption(&InsertCaption, window, cx)
    });
    assert!(matches!(asked(&cx, &docs).0, Ask::Caption("Figure")));
    answer(&mut cx, &docs, "ratio");
    settled(&cx, &docs);
    let path = dir.path().join("formula.odt");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, path.clone(), None, window, cx)
    });
    written(&cx, &path, None);
    let text = text_of(&mut cx, &docs, &path);
    assert_eq!(text.matches("<draw:object").count(), 1, "the edit replaces");
    for mark in ["c over d", ": ratio", "text:sequence"] {
        assert!(text.contains(mark), "{mark} lands");
    }
}

#[test]
fn prompts_refuse_empty_and_taken_answers() {
    let taken = Ask::Bookmark(vec!["Here".into()]);
    assert_eq!(refuse(&taken, "Here"), Err("doc.bookmark.taken"));
    assert_eq!(refuse(&taken, ""), Err("doc.bookmark.empty"));
    assert_eq!(refuse(&taken, "There"), Ok(()));
    assert_eq!(
        refuse(&Ask::Caption("Figure"), ""),
        Err("doc.caption.empty")
    );
    assert_eq!(refuse(&Ask::Formula(true), ""), Err("doc.formula.empty"));
    assert_eq!(refuse(&Ask::Comment, "Note"), Ok(()));
}

#[test]
fn a_long_formula_prefills_whole() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Long");
    let long = format!("{}1", "x + ".repeat(300));
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_formula(&InsertFormula, window, cx)
    });
    answer(&mut cx, &docs, &long);
    settle(&cx, |app| graphic(app, &docs));
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_formula(&InsertFormula, window, cx)
    });
    let (_, field) = asked(&cx, &docs);
    assert_eq!(cx.update(|app| field.read(app).text().to_string()), long);
}

#[test]
fn a_late_dialog_read_never_replaces_a_newer_dialog() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &docs);
    // The link dialog opens before the bookmarks come back.
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_bookmark(&InsertBookmark, window, cx);
        docs.on_link(&InsertLink, window, cx);
    });
    cx.run_until_parked();
    assert!(cx.update(|app| matches!(docs.read(app).dialog, Some(Dialog::Link { .. }))), "the link dialog stays");
    // Opened and closed meanwhile: the old ask lapses too.
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.dialog = None;
        cx.notify();
        docs.on_bookmark(&InsertBookmark, window, cx);
        docs.on_link(&InsertLink, window, cx);
    });
    cx.update(|app| docs.update(app, |docs, cx| {
        docs.dialog = None;
        cx.notify();
    }));
    cx.run_until_parked();
    assert!(cx.update(|app| docs.read(app).dialog.is_none()), "nothing pops up later");
    // A close prompt shown and cancelled meanwhile counts too.
    let id = cx.update(|app| docs.read(app).core.active().expect("a tab"));
    type_into(&mut cx, &docs, "edited");
    settled(&cx, &docs);
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_bookmark(&InsertBookmark, window, cx);
        docs.request_close(id, ely_office::After::Tab, window, cx);
    });
    assert!(cx.update(|app| docs.read(app).core.dialog.is_some()), "the close asks");
    cx.update(|app| docs.update(app, |docs, cx| {
        docs.core.dialog = None;
        cx.notify();
    }));
    cx.run_until_parked();
    assert!(cx.update(|app| docs.read(app).dialog.is_none()), "no bookmark prompt after the cancel");
}
