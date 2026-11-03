//! §9.2: a clicked link reaches Ely; an internal one jumps.

use std::path::Path;

use super::*;

const SETTLE: std::time::Duration = std::time::Duration::from_secs(3);

/// VCL's Ctrl (Cmd on macOS).
const MOD1: i32 = 0x2000;

/// Clicks `cell`'s text and waits until `done`.
fn click(worker: &mut Worker, doc: &Doc, from: &Events, cell: &str, modifiers: i32, done: impl Fn(&Events) -> bool) -> Events {
    worker.run(doc, "cell.goto", &[cell]).expect("the cell");
    let at = poll_until(worker, doc, from, HANG, |seen| seen.cell.address == cell);
    let (x, y, w, h) = at.cell.cursor.expect("the cell cursor");
    for kind in [0, 1] {
        let press = Mouse { kind, x: x + w / 8, y: y + h / 2, count: 1, buttons: 1, modifiers };
        worker.post_mouse(doc, press).expect("the click posts");
    }
    poll_until(worker, doc, &at, HANG, |seen| done(seen) || !seen.blocked.is_empty())
}

#[test]
fn a_plain_click_reports_the_link_and_a_jump_follows_it() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/table/links.xlsx");
    let doc = worker.load(&crate::file_url(&fixture)).expect("loads");
    let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    let linked = |seen: &Events| !seen.links.is_empty();
    // Issue #57: the file's location carries its own '#'.
    let own = click(&mut worker, &doc, &base, "A1", 0, linked);
    assert!(own.blocked.is_empty(), "no dialog: {:?}", own.blocked);
    assert_eq!(own.links, ["#'历史严重违法失信'.A1"]);
    let web = click(&mut worker, &doc, &own, "A3", 0, linked);
    assert_eq!(web.links, ["https://example.com/"]);
    let sheets = web.cell.sheets;
    worker.run(&doc, "link.jump", &["'历史严重违法失信'.B2"]).expect("the jump");
    let there = poll_until(&mut worker, &doc, &web, HANG, |seen| seen.cell.sheets > sheets && seen.cell.address == "B2");
    assert!(there.blocked.is_empty(), "no dialog: {:?}", there.blocked);
    assert_eq!((there.cell.address.as_str(), there.cell.sheets > sheets), ("B2", true), "the target sheet's B2");
    // Ctrl+click: the kernel follows the link itself.
    doc.show_sheet(&mut worker, 0).expect("back to the list");
    let back = poll_until(&mut worker, &doc, &there, HANG, |seen| seen.cell.address == "A1");
    let sheets = back.cell.sheets;
    let jumped = click(&mut worker, &doc, &back, "A1", MOD1, |seen| seen.cell.sheets > sheets);
    assert!(jumped.blocked.is_empty(), "no dialog: {:?}", jumped.blocked);
    assert_eq!(doc.sheets(&mut worker).expect("the sheets").current, 1, "the target sheet shows");
    // The link's own text may hold Calc's marker.
    let mail = "mailto:someone@example.com?body=The coordinates: 1, 2";
    let formula = format!("=HYPERLINK(\"{mail}\",\"mail\")");
    worker.run(&doc, "cell.set", &["'历史严重违法失信'!C1", &formula]).expect("the formula");
    let written = poll_until(&mut worker, &doc, &jumped, SETTLE, |_| false);
    let clicked = click(&mut worker, &doc, &written, "C1", 0, linked);
    assert_eq!(clicked.links, [mail]);
}

#[test]
fn a_closed_message_box_reports_what_it_said() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/table/links.xlsx");
    let doc = worker.load(&crate::file_url(&fixture)).expect("loads");
    let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    // The kernel reports the miss only in its message box.
    worker.run(&doc, "link.jump", &["'没有这张表'.A1"]).expect("the jump runs");
    let seen = poll_until(&mut worker, &doc, &base, HANG, |seen| !seen.blocked.is_empty());
    let (title, body) = seen.blocked.first().expect("the message box opened and closed");
    assert_eq!((title.as_str(), body.as_str()), ("Information", "Invalid range"));
}

#[test]
fn a_writer_link_keeps_its_whole_target() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    // Only Calc appends a click position.
    let mail = "mailto:someone@example.com?body=The coordinates: 1, 2";
    let body = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:xlink="http://www.w3.org/1999/xlink" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
<office:body><office:text><text:p><text:a xlink:type="simple" xlink:href="{mail}">Mail</text:a></text:p></office:text></office:body></office:document>"#);
    let source = dir.path().join("mail.fodt");
    write(&source, &body);
    let doc = worker.load(&crate::file_url(&source)).expect("loads");
    let loaded = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    // Loading encodes spaces; an inserted link keeps them.
    worker.run(&doc, "edit.select_all", &[]).expect("select all");
    worker.run(&doc, "link.insert", &["Mail", mail]).expect("the link");
    let base = poll_until(&mut worker, &doc, &loaded, SETTLE, |_| false);
    worker.run(&doc, "find.run", &["Mail", "false", "0", "0", "0", "1"]).expect("the find");
    let found = poll_until(&mut worker, &doc, &base, HANG, |seen| !seen.selection.is_empty());
    let (x, y, w, h) = *found.selection.first().expect("the link text");
    worker.post_key(&doc, 0, 1027).expect("Right drops the selection");
    let plain = poll_until(&mut worker, &doc, &found, HANG, |seen| seen.selection.is_empty());
    for kind in [0, 1] {
        let press = Mouse { kind, x: x + w / 2, y: y + h / 2, count: 1, buttons: 1, modifiers: MOD1 };
        worker.post_mouse(&doc, press).expect("the click posts");
    }
    let clicked = poll_until(&mut worker, &doc, &plain, HANG, |seen| !seen.links.is_empty());
    assert_eq!(clicked.links, [mail]);
}
