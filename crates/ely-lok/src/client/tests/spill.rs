//! §7: Excel's spilling arrays stay spilling through a save.

use std::path::Path;

use super::independent::read_back;
use super::*;

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/table").join(name)
}

/// Saves `doc` as XLSX; the reader checks it as `case`.
fn saved(worker: &mut Worker, doc: &Doc, dir: &Path, name: &str, case: &str) -> std::path::PathBuf {
    let out = dir.join(name);
    doc.save_as(worker, &crate::file_url(&out), "xlsx").expect("saves");
    read_back(&out, case);
    out
}

#[test]
fn dynamic_arrays_save_as_dynamic_and_plain_ones_as_plain() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let doc = worker.load(&crate::file_url(&fixture("spill.xlsx"))).expect("loads");
    let first = saved(&mut worker, &doc, dir.path(), "first.xlsx", "spill");
    // A copied sheet, put first, is the one read.
    worker.run(&doc, "sheet.move", &["1", "true", "true"]).expect("copies");
    let names = doc.sheets(&mut worker).expect("sheets").list;
    assert_eq!(names.first().map(|sheet| sheet.name.as_str()), Some("Sheet1_2"), "{names:?}");
    saved(&mut worker, &doc, dir.path(), "copied.xlsx", "spill");
    // Ely's own output reads back the same.
    let again = worker.load(&crate::file_url(&first)).expect("reloads");
    saved(&mut worker, &again, dir.path(), "again.xlsx", "spill");
}

#[test]
fn pasted_and_undone_dynamic_arrays_stay_dynamic() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let doc = worker.load(&crate::file_url(&fixture("spill.xlsx"))).expect("loads");
    worker.run(&doc, "cell.goto", &["Sheet1!K1"]).expect("K1");
    worker.copy(&doc).expect("copies");
    // Two rows: the paste builds a formula group.
    worker.run(&doc, "cell.goto", &["Sheet1!O1:O2"]).expect("O1:O2");
    worker.run(&doc, "paste.formulas", &[]).expect("pastes");
    // Deleting row 4 moves every A1:A4; undo rebuilds them.
    worker.run(&doc, "cell.goto", &["Sheet1!A4"]).expect("A4");
    worker.run(&doc, "rows.delete", &[]).expect("deletes");
    worker.run(&doc, "edit.undo", &[]).expect("undone");
    saved(&mut worker, &doc, dir.path(), "pasted.xlsx", "spill-pasted");
}

#[test]
fn unreadable_metadata_fails_the_load() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    // Truncated XML, then a zero-byte part.
    for name in ["spill-broken.xlsx", "spill-empty.xlsx"] {
        let loaded = worker.load(&crate::file_url(&fixture(name)));
        assert!(loaded.is_err(), "{name} loaded");
    }
    // The worker stays up.
    worker.load(&crate::file_url(&fixture("spill.xlsx"))).expect("loads after");
}
