//! #147: numbered items offer the submenu and take values.

use gpui::{Entity, TestAppContext};

use super::*;
use crate::tests::{docs, settle, settled, with_window};

/// Three numbered items, then a plain paragraph.
const BODY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text"><office:automatic-styles><text:list-style style:name="L1"><text:list-level-style-number text:level="1" style:num-format="1" style:num-suffix="."/></text:list-style></office:automatic-styles><office:body><office:text><text:list text:style-name="L1"><text:list-item><text:p>one</text:p></text:list-item><text:list-item><text:p>two</text:p></text:list-item><text:list-item><text:p>three</text:p></text:list-item></text:list><text:p>plain</text:p></office:text></office:body></office:document>"#;

const DOWN: i32 = 1024;

fn number(docs: &Entity<Docs>, app: &App) -> Option<String> {
    let view = &docs.read(app).active_tab()?.view;
    view.read(app).state(NUMBER).map(str::to_owned)
}

fn press(cx: &mut TestAppContext, docs: &Entity<Docs>, key: i32) {
    cx.update(|app| {
        let docs = docs.read(app);
        let doc = docs.active_tab().expect("a tab").doc.clone();
        docs.core.worker.lock().expect("the worker").post_key(&doc, 0, key).expect("the key goes");
    });
}

#[test]
fn a_numbered_item_offers_numbering_and_takes_a_value() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    let path = dir.path().join("numbered.fodt");
    std::fs::write(&path, BODY).expect("the file writes");
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(path.clone(), cx));
    settle(&cx, |app| docs.read(app).core.tabs.len() == 1);
    settled(&cx, &docs);
    // The cursor opens on "one", item 1.
    settle(&cx, |app| number(&docs, app).as_deref() == Some("1"));
    assert!(cx.update(|app| docs.read(app).numbered(app).is_some()), "a numbered item offers numbering");
    for bad in ["", "0", "x", "10000", "-3"] {
        assert_eq!(refuse(bad), Some("doc.number.invalid"), "{bad:?} is refused");
    }
    assert_eq!(refuse("9999"), None);
    // The prompt's answer sets this item; the rest follow.
    cx.update(|app| docs.update(app, |docs, cx| docs.answer(&Ask::Number, "7", cx)));
    settle(&cx, |app| number(&docs, app).as_deref() == Some("7"));
    press(&mut cx, &docs, DOWN);
    settle(&cx, |app| number(&docs, app).as_deref() == Some("8"));
    // "Restart at 1" sends 1.
    cx.update(|app| docs.update(app, |docs, cx| docs.run_active("list.number_from", &["1"], cx)));
    settle(&cx, |app| number(&docs, app).as_deref() == Some("1"));
    // A plain paragraph has no submenu.
    press(&mut cx, &docs, DOWN);
    press(&mut cx, &docs, DOWN);
    settle(&cx, |app| number(&docs, app).as_deref() == Some(""));
    assert!(cx.update(|app| docs.read(app).numbered(app).is_none()), "a plain paragraph offers none");
}

#[test]
fn each_format_reads_as_its_first_number() {
    let labels: Vec<String> = FORMATS.into_iter().map(sample).collect();
    assert_eq!(
        labels,
        ["1.", "1)", "(1)", "I.", "i.", "A.", "a)", "①", "一、", "（一）", "第1条", "第一条"]
    );
}
