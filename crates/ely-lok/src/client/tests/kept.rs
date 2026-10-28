//! §19 (D-02): what Calc cannot compute keeps the file's result.

use std::path::Path;

use super::independent::read_back;
use super::*;
use crate::sheet::{KEPT, cell_at, cell_at_query, kept};

/// Shown text and input of each cell on Sheet1.
fn cells(worker: &mut Worker, doc: &Doc, wanted: &[(&str, &str, &str)]) {
    cells_on(worker, doc, "Sheet1", wanted);
}

fn cells_on(worker: &mut Worker, doc: &Doc, sheet: &str, wanted: &[(&str, &str, &str)]) {
    for &(cell, text, input) in wanted {
        let read = worker.values(doc, &cell_at_query(&format!("{sheet}!{cell}"))).expect("reads");
        let at = cell_at(&read).expect("a cell");
        assert_eq!((at.text.as_str(), at.input.as_str()), (text, input), "{cell}");
    }
}

fn kept_cells(worker: &mut Worker, doc: &Doc) -> Vec<String> {
    kept(&worker.values(doc, KEPT).expect("reads")).expect("parses")
}

const CUBE: &str = "CUBEVALUE(\"Sales\",\"[Measures].[Total]\")";

#[test]
fn unknown_functions_keep_the_files_results_until_an_input_changes() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/table/kept.xlsx");
    let doc = worker.load(&crate::file_url(&source)).expect("loads");
    let (cube, times) = (format!("={CUBE}"), format!("=A1*{CUBE}"));
    let loaded = [
        ("B1", "41", cube.as_str()),
        ("C1", "42", "=B1+1"),
        ("D1", "82", times.as_str()),
        ("E1", "hallo", "=_xlfn.TRANSLATE(\"hello\",\"en\",\"de\")"),
        ("F1", "3", "=SUM(A1,1)"),
        ("B2", "100", "=CUBEVALUE(\"Sales\",A2)"),
        ("B3", "200", "=CUBEVALUE(\"Sales\",A3)"),
    ];
    cells(&mut worker, &doc, &loaded);
    let all = ["Sheet1!B1", "Sheet1!B2", "Sheet1!B3", "Sheet1!D1", "Sheet1!E1"];
    assert_eq!(kept_cells(&mut worker, &doc), all, "the banner's cells");
    let (xlsx, ods) = (dir.path().join("kept.xlsx"), dir.path().join("kept.ods"));
    doc.save_as(&mut worker, &crate::file_url(&xlsx), "xlsx").expect("saves");
    read_back(&xlsx, "kept");
    doc.save_as(&mut worker, &crate::file_url(&ods), "ods").expect("saves");
    worker.run(&doc, "cell.set", &["Sheet1!A1", "3"]).expect("A1");
    worker.run(&doc, "cell.set", &["Sheet1!A2", "11"]).expect("A2");
    cells(&mut worker, &doc, &[("D1", "#NAME?", &times), ("B2", "#NAME?", "=CUBEVALUE(\"Sales\",A2)"), ("F1", "4", "=SUM(A1,1)")]);
    cells(&mut worker, &doc, &loaded[..2]);
    cells(&mut worker, &doc, &[loaded[3], loaded[6]]);
    let left = ["Sheet1!B1", "Sheet1!B3", "Sheet1!E1"];
    assert_eq!(kept_cells(&mut worker, &doc), left, "changed inputs end theirs");
    let edited = dir.path().join("edited.xlsx");
    doc.save_as(&mut worker, &crate::file_url(&edited), "xlsx").expect("saves");
    read_back(&edited, "kept-edited");
    // A row above moves them; their inputs stay.
    worker.run(&doc, "cell.goto", &["Sheet1!A1"]).expect("A1");
    worker.run(&doc, "rows.insert_above", &[]).expect("a row");
    cells(&mut worker, &doc, &[("B2", "41", &cube), ("B4", "200", "=CUBEVALUE(\"Sales\",A4)")]);
    assert_eq!(kept_cells(&mut worker, &doc), ["Sheet1!B2", "Sheet1!B4", "Sheet1!E2"], "moved");
    // Without its input B4 cannot stand.
    worker.run(&doc, "cell.goto", &["Sheet1!A1"]).expect("A1");
    worker.run(&doc, "columns.delete", &[]).expect("column A");
    cells(&mut worker, &doc, &[("A2", "41", &cube), ("A4", "#NAME?", "=CUBEVALUE(\"Sales\",#REF!)")]);
    assert_eq!(kept_cells(&mut worker, &doc), ["Sheet1!A2", "Sheet1!D2"], "an input gone");
    worker.close_doc(doc).expect("closes");
    // Ely's own copies keep them too.
    for copy in [&xlsx, &ods] {
        let again = worker.load(&crate::file_url(copy)).expect("reopens");
        cells(&mut worker, &again, &loaded);
        assert_eq!(kept_cells(&mut worker, &again), all, "{}", copy.display());
        worker.close_doc(again).expect("closes");
    }
}


fn open_fixture(worker: &mut Worker, name: &str) -> Doc {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/table").join(name);
    worker.load(&crate::file_url(&source)).expect("loads")
}

#[test]
fn a_copied_sheet_and_an_undone_insert_keep_them() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let doc = open_fixture(&mut worker, "kept.xlsx");
    worker.run(&doc, "sheet.move", &["2", "true", "true"]).expect("copies");
    let (cube, b2) = (format!("={CUBE}"), "=CUBEVALUE(\"Sales\",A2)");
    cells_on(&mut worker, &doc, "Sheet1_2", &[("B1", "41", &cube), ("B2", "100", b2)]);
    let both = [
        "Sheet1!B1", "Sheet1!B2", "Sheet1!B3", "Sheet1!D1", "Sheet1!E1",
        "Sheet1_2!B1", "Sheet1_2!B2", "Sheet1_2!B3", "Sheet1_2!D1", "Sheet1_2!E1",
    ];
    assert_eq!(kept_cells(&mut worker, &doc), both, "the copy's too");
    worker.run(&doc, "cell.goto", &["Sheet1!A1"]).expect("A1");
    worker.run(&doc, "rows.insert_above", &[]).expect("a row");
    worker.run(&doc, "edit.undo", &[]).expect("undone");
    cells(&mut worker, &doc, &[("B2", "100", b2), ("B3", "200", "=CUBEVALUE(\"Sales\",A3)")]);
    assert_eq!(kept_cells(&mut worker, &doc), both, "undo brings them back");
    // The copy's inputs are its own.
    worker.run(&doc, "cell.set", &["Sheet1_2!A2", "11"]).expect("A2");
    cells_on(&mut worker, &doc, "Sheet1_2", &[("B2", "#NAME?", b2)]);
    cells(&mut worker, &doc, &[("B2", "100", b2)]);
}

#[test]
fn booleans_empty_strings_arrays_and_volatile_inputs_keep_them_too() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let doc = open_fixture(&mut worker, "kept-more.xlsx");
    let (array, cube) = ("{=FUTUREFN(1)}", format!("{{={CUBE}}}"));
    let loaded = [
        ("A1", "TRUE", "=FUTUREFN(1)"),
        ("B1", "", "=FUTUREFN(2)"),
        // No result in the file, then a bad number.
        ("C1", "#NAME?", "=FUTUREFN(3)"),
        ("D1", "#NAME?", "=FUTUREFN(4)"),
        ("E1", "41", cube.as_str()),
        ("A2", "TRUE", "=FUTUREFN(I2)"),
        ("A3", "FALSE", "=FUTUREFN(I3)"),
        ("B2", "", "=FUTUREFN(I2)"),
        ("B3", "ok", "=FUTUREFN(I3)"),
        ("G5", "1", array),
        ("H5", "2", array),
        ("G6", "3", array),
        ("H6", "four", array),
        ("K8", "42", "=FUTUREFN(INDIRECT(\"J8\"))"),
        ("L8", "43", "=FUTUREFN(OFFSET(J8,0,0))"),
    ];
    cells(&mut worker, &doc, &loaded);
    let all = [
        "Sheet1!A1", "Sheet1!A2", "Sheet1!A3", "Sheet1!B1", "Sheet1!B2",
        "Sheet1!B3", "Sheet1!E1", "Sheet1!G5", "Sheet1!K8", "Sheet1!L8",
    ];
    assert_eq!(kept_cells(&mut worker, &doc), all, "the banner's cells");
    let (xlsx, ods) = (dir.path().join("more.xlsx"), dir.path().join("more.ods"));
    doc.save_as(&mut worker, &crate::file_url(&xlsx), "xlsx").expect("saves");
    read_back(&xlsx, "kept-more");
    doc.save_as(&mut worker, &crate::file_url(&ods), "ods").expect("saves");
    // Volatile cells take any input; formatting is none.
    worker.run(&doc, "cell.set", &["Sheet1!J8", "5"]).expect("J8");
    cells(&mut worker, &doc, &[("K8", "#NAME?", loaded[13].2), ("L8", "#NAME?", loaded[14].2)]);
    worker.run(&doc, "cell.goto", &["Sheet1!A5"]).expect("A5");
    worker.run(&doc, "format.bold", &[]).expect("bold");
    assert_eq!(kept_cells(&mut worker, &doc), all[..8], "formatting keeps them");
    worker.run(&doc, "cell.set", &["Sheet1!I2", "5"]).expect("I2");
    cells(&mut worker, &doc, &[("A2", "#NAME?", "=FUTUREFN(I2)"), ("B2", "#NAME?", "=FUTUREFN(I2)")]);
    cells(&mut worker, &doc, &[loaded[2], loaded[6], loaded[8], loaded[12]]);
    worker.close_doc(doc).expect("closes");
    for copy in [&xlsx, &ods] {
        let again = worker.load(&crate::file_url(copy)).expect("reopens");
        cells(&mut worker, &again, &loaded);
        assert_eq!(kept_cells(&mut worker, &again), all, "{}", copy.display());
        worker.close_doc(again).expect("closes");
    }
}
