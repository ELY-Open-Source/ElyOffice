//! Reads come from the kernel's body, never an export (R23.2b).

use std::time::{Duration, Instant};

use ely_office::prelude::*;
use gpui::{Entity, TestAppContext};

use crate::docs::Docs;
use crate::review::TrackChanges;
use crate::tests::{docs, settle, settled, with_window};
use super::ask;

/// vcl key codes and modifiers.
const DOWN: i32 = 1024;
const RIGHT: i32 = 1027;
const HOME: i32 = 1028;
const END: i32 = 1029;
const DELETE: i32 = 1286;
const MOD1: i32 = 0x2000;

/// A flat document around `body`.
fn flat(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text"><office:styles><style:style style:name="Heading_20_1" style:display-name="Heading 1" style:family="paragraph"/></office:styles><office:body><office:text>{body}</office:text></office:body></office:document>"#
    )
}

/// Opens `body` with recording on; its path comes back.
fn opened(cx: &mut TestAppContext, docs: &Entity<Docs>, dir: &std::path::Path, body: &str) -> String {
    let path = dir.join("body.fodt");
    std::fs::write(&path, flat(body)).expect("the file writes");
    with_window(cx, docs, |docs, _, cx| docs.open_file(path.clone(), cx));
    settle(cx, |app| docs.read(app).core.tabs.len() == 1);
    settled(cx, docs);
    with_window(cx, docs, |docs, window, cx| docs.on_track(&TrackChanges, window, cx));
    let view = cx.update(|app| docs.read(app).core.tabs[0].view.clone());
    settle(cx, |app| view.read(app).state(".uno:TrackChanges") == Some("true"));
    path.display().to_string()
}

/// Each listed paragraph's style and text.
fn listed(json: &str) -> Vec<(String, String)> {
    let list: Vec<serde_json::Value> = serde_json::from_str(json).expect("a JSON list");
    list.iter().map(|it| (it["style"].as_str().expect("a style").into(), it["text"].as_str().expect("a text").into())).collect()
}

/// What the flat ODF copy reads; it moves the cursor.
fn exported(cx: &mut TestAppContext, docs: &Entity<Docs>, dir: &std::path::Path) -> (Vec<(String, String)>, Duration) {
    cx.update(|app| {
        let docs = docs.read(app);
        let mut worker = docs.core.worker.lock().expect("the worker");
        let start = Instant::now();
        let flat = ely_office::remote::copy::<ely_office::Lok>(&mut worker, &docs.core.tabs[0].doc, &dir.join("old.fodt"), "fodt").expect("a flat copy");
        let took = start.elapsed();
        (ely_office::remote::body_paragraphs(&flat).expect("paragraphs"), took)
    })
}

#[test]
fn the_body_reads_as_the_flat_copy_does_and_the_cursor_stays() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let file = opened(
        &mut cx,
        &docs,
        dir.path(),
        r#"<text:h text:style-name="Heading_20_1" text:outline-level="1">Head<text:note text:note-class="footnote"><text:note-citation>1</text:note-citation><text:note-body><text:p>aside</text:p></text:note-body></text:note></text:h><text:p>keep gone this, page <text:page-number text:select-page="current">1</text:page-number></text:p><text:p>join</text:p><text:p>here soft&#xAD;ly<office:annotation><dc:creator>A</dc:creator><text:p>remark</text:p></office:annotation> said</text:p><text:p>drop</text:p><text:p>after</text:p><table:table table:name="T"><table:table-column table:number-columns-repeated="2"/><table:table-row><table:table-cell><text:p>left</text:p></table:table-cell><table:table-cell><text:p>right</text:p></table:table-cell></table:table-row></table:table><text:p>end</text:p>"#,
    );
    assert_eq!(ask(&mut cx, &docs, &["replace", &file, "gone ", ""]), Ok(String::new()));
    assert_eq!(ask(&mut cx, &docs, &["replace", &file, "end", "the end"]), Ok(String::new()));
    let view = cx.update(|app| docs.read(app).core.tabs[0].view.clone());
    // Each key waits for the kernel to move the cursor.
    let press = |cx: &mut TestAppContext, key: i32| {
        let before = cx.update(|app| view.read(app).cursor());
        cx.update(|app| {
            let docs = docs.read(app);
            let mut worker = docs.core.worker.lock().expect("the worker");
            worker.post_key(&docs.core.tabs[0].doc, 0, key).expect("the key goes");
        });
        settle(cx, |app| view.read(app).cursor().is_some_and(|at| Some(at) != before));
    };
    // From the top: the break after "join", then "drop".
    for key in [DOWN, DOWN, END, DELETE, HOME | MOD1, DOWN, DOWN, DOWN, DOWN] {
        press(&mut cx, key);
    }
    for _ in 0.."drop\n".len() {
        press(&mut cx, DELETE);
    }
    // Into the deleted "gone".
    press(&mut cx, HOME | MOD1);
    press(&mut cx, DOWN);
    for _ in 0.."keep go".len() {
        press(&mut cx, RIGHT);
    }
    let inside = cx.update(|app| view.read(app).cursor());
    let start = Instant::now();
    let read = ask(&mut cx, &docs, &["get", &file, "--json"]).expect("the paragraphs");
    let took = start.elapsed();
    let plain = ask(&mut cx, &docs, &["get", &file]).expect("the text");
    settled(&cx, &docs);
    assert_eq!(cx.update(|app| view.read(app).cursor()), inside, "a read leaves the cursor in deleted text");
    let read = listed(&read);
    let texts: Vec<_> = read.iter().map(|(_, text)| text.as_str()).collect();
    assert_eq!(plain, texts.join("\n"), "the plain read is the same body");
    assert_eq!(read[0], ("Heading 1".into(), "Head".into()), "the footnote's number stays out");
    assert!(texts.contains(&"keep this, page 1"), "{texts:?}");
    assert!(texts.contains(&"joinhere soft\u{ad}ly said"), "a deleted break joins; {texts:?}");
    assert!(!plain.contains("drop") && !plain.contains("gone"), "{plain:?}");
    let (old, old_took) = exported(&mut cx, &docs, dir.path());
    assert_eq!(read, old, "the flat copy reads the same");
    eprintln!("read: {took:?} from the body, {old_took:?} through a flat copy");
}

#[test]
fn thousands_of_tracked_deletions_read_quickly() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let paragraph = format!("<text:p>{}</text:p>", "word gone ".repeat(10));
    let file = opened(&mut cx, &docs, dir.path(), &paragraph.repeat(400));
    assert_eq!(ask(&mut cx, &docs, &["replace", &file, "gone ", ""]), Ok(String::new()));
    let start = Instant::now();
    let read = ask(&mut cx, &docs, &["get", &file]).expect("the text");
    let took = start.elapsed();
    assert_eq!(read, vec!["word ".repeat(10); 400].join("\n"));
    let (_, old_took) = exported(&mut cx, &docs, dir.path());
    eprintln!("4,000 deletions: {took:?} from the body, {old_took:?} through a flat copy");
    assert!(took * 5 < old_took, "the body reads in a fifth of the export's time");
}
