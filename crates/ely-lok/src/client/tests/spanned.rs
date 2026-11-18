//! §9: an external 3D range reads every sheet it spans.

use super::analysis::{cells, skip};
use super::linked::{book, update};
use super::sheet::open_book;
use super::*;
use crate::links;

/// A source with these sheets; 0 leaves A1 blank.
fn spanned(sheets: &[(&str, u32)]) -> String {
    let cell = |v: u32| format!(r#"<table:table-row><table:table-cell office:value-type="float" office:value="{v}"/></table:table-row>"#);
    let tables: String = (sheets.iter())
        .map(|&(name, v)| format!(r#"<table:table table:name="{name}">{}</table:table>"#, if v == 0 { String::new() } else { cell(v) }))
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet>{tables}</office:spreadsheet></office:body>
</office:document>"#
    )
}

const DIRECT: &str = "'{}/[source.fods]S1:S3'!A1";
const INDIRECT: &str = "INDIRECT(\"'{}/[source.fods]S1:S3'!A1\")";
/// S1 = 1, S2 = 2, S3 blank.
const BLANK_LAST: &[(&str, u32)] = &[("S1", 1), ("S2", 2), ("S3", 0)];

/// A reader whose H1 holds `formula` over `sheets`.
fn linked(formula: &str, sheets: &[(&str, u32)], warm: bool) -> (tempfile::TempDir, Worker, Doc, std::path::PathBuf) {
    let (dir, mut worker, reader) = open_book(&book(0));
    let source = dir.path().join("source.fods");
    std::fs::write(&source, spanned(sheets)).expect("writes the source");
    let at = dir.path().display().to_string();
    // A single cell first: the source is cached.
    if warm {
        enter(&mut worker, &reader, "H2", &format!("='{at}/[source.fods]S1'!A1"));
    }
    enter(&mut worker, &reader, "H1", &formula.replace("{}", &at));
    (dir, worker, reader, source)
}

fn enter(worker: &mut Worker, doc: &Doc, cell: &str, formula: &str) {
    worker.run(doc, "cell.goto", &[cell]).expect("the cell");
    worker.run(doc, "cell.enter", &[formula]).expect("the formula");
}

fn formula(worker: &mut Worker, doc: &Doc) -> String {
    worker.run(doc, "cell.goto", &["H1"]).expect("H1");
    poll_until(worker, doc, &Events::default(), HANG, |seen| seen.cell.address == "H1" && !seen.cell.formula.is_empty()).cell.formula
}

/// The reader saved as XLSX and opened again.
fn reopened(dir: &tempfile::TempDir, worker: &mut Worker, reader: &Doc) -> Doc {
    let saved = dir.path().join("reader.xlsx");
    reader.save_as(worker, &crate::file_url(&saved), "xlsx").expect("saves");
    worker.load(&crate::file_url(&saved)).expect("reloads")
}

#[test]
fn an_external_3d_range_reads_every_sheet() {
    if skip() {
        return;
    }
    for warm in [false, true] {
        for (cell, value) in [(format!("=SUM({DIRECT})"), "3"), (format!("=SUM({INDIRECT})"), "3"), (format!("=COUNT({DIRECT})"), "2"), (format!("=AVERAGE({DIRECT})"), "1.5")] {
            let (_dir, mut worker, reader, source) = linked(&cell, BLANK_LAST, warm);
            assert_eq!(cells(&mut worker, &reader, "H1"), value, "{cell} warm {warm}");
            let written = formula(&mut worker, &reader);
            assert!(written.contains("S1':'S3'") || written.contains("S1:S3"), "{cell} warm {warm}: the last sheet stays in {written}");
            let listed = links::sources(&worker.values(&reader, links::LIST).expect("reads")).expect("parses");
            assert_eq!(listed, [crate::file_url(&source)], "{cell} warm {warm}: one link per source");
            // The shown formula enters again alike.
            enter(&mut worker, &reader, "H1", &written);
            assert_eq!(cells(&mut worker, &reader, "H1"), value, "{written} entered again");
        }
    }
    // Other functions take one sheet: they say so.
    let (_dir, mut worker, reader, _) = linked(&format!("=INDEX({DIRECT},1,1)"), BLANK_LAST, true);
    assert_eq!(cells(&mut worker, &reader, "H1"), "Err:502", "no first sheet passes for the range");
}

#[test]
fn a_last_sheet_the_source_lacks_is_no_range() {
    if skip() {
        return;
    }
    for range in [DIRECT, INDIRECT] {
        let range = range.replace("S3", "S9");
        let (_dir, mut worker, reader, _) = linked(&format!("=SUM({range})"), BLANK_LAST, false);
        assert_eq!(cells(&mut worker, &reader, "H1"), "#REF!", "{range}: S1 alone is not read");
    }
}

#[test]
fn a_range_whose_blank_last_sheet_is_gone_keeps_its_value() {
    if skip() {
        return;
    }
    for range in [DIRECT, INDIRECT] {
        let (_dir, mut worker, reader, source) = linked(&format!("=SUM({range})"), BLANK_LAST, false);
        assert_eq!(cells(&mut worker, &reader, "H1"), "3", "{range}: S1 and S2 add up");
        // The blank S3 goes; S1 changes.
        std::fs::write(&source, spanned(&[("S1", 10), ("S2", 2)])).expect("removes S3");
        assert_eq!(update(&mut worker, &reader), [crate::file_url(&source)], "{range}: the source is named");
        assert_eq!(cells(&mut worker, &reader, "H1"), "3", "{range}: the old value stays");
        // S3 back: the update reads the new values.
        std::fs::write(&source, spanned(&[("S1", 10), ("S2", 2), ("S3", 0)])).expect("restores S3");
        assert_eq!(update(&mut worker, &reader), Vec::<String>::new(), "{range}: the update succeeds");
        assert_eq!(cells(&mut worker, &reader, "H1"), "12", "{range}: the new values");
    }
}

#[test]
fn a_range_saved_as_xlsx_updates_on_request() {
    if skip() {
        return;
    }
    let (dir, mut worker, reader, source) = linked(&format!("=SUM({DIRECT})"), &[("S1", 1), ("S2", 2), ("S3", 4)], false);
    let book = reopened(&dir, &mut worker, &reader);
    assert_eq!(cells(&mut worker, &book, "H1"), "7", "the saved value");
    assert!(formula(&mut worker, &book).contains("S3"), "the range is a formula again");
    std::fs::write(&source, spanned(&[("S1", 10), ("S2", 20), ("S3", 40)])).expect("changes the source");
    assert_eq!(update(&mut worker, &book), Vec::<String>::new(), "the update succeeds");
    assert_eq!(cells(&mut worker, &book, "H1"), "70", "every sheet is read again");
    // Reopened, a gone blank sheet fails too.
    let (dir, mut worker, reader, source) = linked(&format!("=SUM({DIRECT})"), BLANK_LAST, false);
    let book = reopened(&dir, &mut worker, &reader);
    std::fs::write(&source, spanned(&[("S1", 10), ("S2", 20)])).expect("removes S3");
    assert_eq!(update(&mut worker, &book), [crate::file_url(&source)], "the source is named");
    assert_eq!(cells(&mut worker, &book, "H1"), "3", "the saved value stays");
}

#[test]
fn a_reopened_indirect_range_awaits_its_source() {
    if skip() {
        return;
    }
    let (dir, mut worker, reader, source) = linked(&format!("=SUM({INDIRECT})"), BLANK_LAST, false);
    let book = reopened(&dir, &mut worker, &reader);
    // XLSX keeps no INDIRECT result: the link awaits a check.
    assert_eq!(cells(&mut worker, &book, "H1"), "Err:540", "unread until asked");
    std::fs::write(&source, spanned(&[("S1", 10), ("S2", 20)])).expect("removes S3");
    assert_eq!(update(&mut worker, &book), [crate::file_url(&source)], "the named last sheet is gone");
    assert_eq!(cells(&mut worker, &book, "H1"), "Err:540", "nothing changes");
    std::fs::write(&source, spanned(&[("S1", 10), ("S2", 20), ("S3", 0)])).expect("restores S3");
    assert_eq!(update(&mut worker, &book), Vec::<String>::new(), "the update succeeds");
    assert_eq!(cells(&mut worker, &book, "H1"), "30", "every sheet is read");
}

#[test]
fn a_range_follows_its_named_ends_when_sheets_move() {
    if skip() {
        return;
    }
    for range in [DIRECT, INDIRECT] {
        let (_dir, mut worker, reader, source) = linked(&format!("=SUM({range})"), &[("S1", 1), ("S2", 2), ("S3", 4)], false);
        assert_eq!(cells(&mut worker, &reader, "H1"), "7", "{range}: three sheets");
        // A sheet before the last joins the range.
        std::fs::write(&source, spanned(&[("S1", 10), ("S2", 20), ("S2a", 30), ("S3", 40)])).expect("inserts S2a");
        assert_eq!(update(&mut worker, &reader), Vec::<String>::new(), "{range}: the update succeeds");
        assert_eq!(cells(&mut worker, &reader, "H1"), "100", "{range}: S1 to S3 again");
        // The ends swap places: the range runs between them.
        std::fs::write(&source, spanned(&[("S3", 1), ("S2a", 2), ("S2", 3), ("S1", 4), ("After", 50)])).expect("reverses the sheets");
        assert_eq!(update(&mut worker, &reader), Vec::<String>::new(), "{range}: the reversed update succeeds");
        assert_eq!(cells(&mut worker, &reader, "H1"), "10", "{range}: S3 to S1");
    }
}
