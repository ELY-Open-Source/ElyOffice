//! §19.2: Ely edits, saves, reopens; another reader checks.

use std::{path::Path, process::Command};

use super::input::{FODS, open};
use super::roundtrip::type_in;
use super::show::open_deck;
use super::*;

/// Saves as `format`, closes, reopens for a second edit.
fn reopened(worker: &mut Worker, doc: Doc, dir: &Path, name: &str, format: &str) -> Doc {
    let first = dir.join(format!("{name}.{format}"));
    doc.save_as(worker, &crate::file_url(&first), format).expect("the first save");
    worker.close_doc(doc).expect("closes");
    worker.load(&crate::file_url(&first)).expect("reopens")
}

/// The second save, read by `tests/scripts/ooxml.py`.
fn checked(worker: &mut Worker, doc: Doc, dir: &Path, case: &str, format: &str) {
    let last = dir.join(format!("{case}-again.{format}"));
    doc.save_as(worker, &crate::file_url(&last), format).expect("the second save");
    worker.close_doc(doc).expect("closes");
    read_back(&last, case);
}

/// `tests/scripts/ooxml.py` checks `file` as `case`.
pub(super) fn read_back(file: &Path, case: &str) {
    let reader = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/scripts/ooxml.py");
    // The file passes, and each known damage to it fails.
    for args in [vec![case], vec!["selftest", case]] {
        let out = match Command::new("python3").arg(&reader).args(&args).arg(file).output() {
            Ok(out) => out,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return eprintln!("SKIP: no python3 to read {case}");
            }
            Err(error) => panic!("the reader did not run: {error}"),
        };
        let said = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        assert!(out.status.success(), "{args:?}: {said}");
    }
}

#[test]
fn a_docx_keeps_text_comments_and_tracking_across_sessions() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    type_in(&mut worker, &doc, "Alpha 甲 ");
    worker.run(&doc, "comment.insert", &["Check alpha"]).expect("comments");
    worker.run(&doc, "track.toggle", &[]).expect("tracks");
    type_in(&mut worker, &doc, "Beta ");
    let doc = reopened(&mut worker, doc, dir.path(), "contract", "docx");
    type_in(&mut worker, &doc, "Gamma ");
    checked(&mut worker, doc, dir.path(), "contract", "docx");
}

#[test]
fn an_xlsx_keeps_formulas_and_results_across_sessions() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODS);
    let enter = |worker: &mut Worker, doc: &Doc, cells: &[(&str, &str)]| {
        for (cell, entry) in cells {
            worker.run(doc, "cell.goto", &[cell]).expect("the cell");
            worker.run(doc, "cell.enter", &[entry]).expect("the entry");
        }
    };
    enter(&mut worker, &doc, &[
        ("H1", "7"),
        ("H2", "5"),
        ("H3", "=H1*H2"),
        ("H4", "=H1/3"),
        ("H5", "=DATE(2024,2,29)"),
        ("H6", "=1/0"),
        ("H7", "=\"Ely\"&\"-\"&TEXT(H1,\"0\")"),
        ("H8", "=SUM(H1:H2)"),
    ]);
    let doc = reopened(&mut worker, doc, dir.path(), "sheet", "xlsx");
    poll_until(&mut worker, &doc, &Events::default(), std::time::Duration::from_secs(3), |seen| {
        seen.cell.cursor.is_some()
    });
    enter(&mut worker, &doc, &[("H9", "=H3+1")]);
    checked(&mut worker, doc, dir.path(), "sheet", "xlsx");
}

#[test]
fn a_pptx_keeps_slides_hiding_and_notes_across_sessions() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_deck();
    let at = worker.now(&doc).expect("the state");
    worker.run(&at, "notes.set", &["Speaker 甲", "0"]).expect("notes");
    doc.show_sheet(&mut worker, 1).expect("slide two");
    worker.run(&doc, "slide.hide", &[]).expect("hides");
    let doc = reopened(&mut worker, doc, dir.path(), "deck", "pptx");
    worker.run(&doc, "slide.insert", &[]).expect("a third slide");
    checked(&mut worker, doc, dir.path(), "deck", "pptx");
}
