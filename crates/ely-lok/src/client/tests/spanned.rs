//! §9: an external 3D range reads every sheet it spans.

use super::analysis::{cells, skip};
use super::linked::{book, update};
use super::sheet::open_book;
use super::*;
use crate::links;

/// A source with sheets `names`; the first two hold A1.
fn spanned(names: &[&str], first: u32) -> String {
    let cell = |v: u32| format!(r#"<table:table-row><table:table-cell office:value-type="float" office:value="{v}"/></table:table-row>"#);
    let tables: String = (names.iter().enumerate())
        .map(|(i, name)| format!(r#"<table:table table:name="{name}">{}</table:table>"#, [cell(first), cell(2)].get(i).cloned().unwrap_or_default()))
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

/// A reader whose H1 holds `formula` over S1 to S3.
fn linked(formula: &str, first: u32, warm: bool) -> (tempfile::TempDir, Worker, Doc, std::path::PathBuf) {
    let (dir, mut worker, reader) = open_book(&book(0));
    let source = dir.path().join("source.fods");
    std::fs::write(&source, spanned(&["S1", "S2", "S3"], first)).expect("writes the source");
    let at = dir.path().display().to_string();
    // A single cell first: the source is cached.
    if warm {
        worker.run(&reader, "cell.goto", &["H2"]).expect("H2");
        worker.run(&reader, "cell.enter", &[&format!("='{at}/[source.fods]S1'!A1")]).expect("the cell");
    }
    worker.run(&reader, "cell.goto", &["H1"]).expect("H1");
    worker.run(&reader, "cell.enter", &[&formula.replace("{}", &at)]).expect("the formula");
    (dir, worker, reader, source)
}

fn formula(worker: &mut Worker, doc: &Doc) -> String {
    worker.run(doc, "cell.goto", &["H1"]).expect("H1");
    poll_until(worker, doc, &Events::default(), HANG, |seen| seen.cell.address == "H1" && !seen.cell.formula.is_empty()).cell.formula
}

#[test]
fn an_external_3d_range_reads_every_sheet() {
    if skip() {
        return;
    }
    for warm in [false, true] {
        for (cell, value) in [(format!("=SUM({DIRECT})"), "3"), (format!("=SUM({INDIRECT})"), "3"), (format!("=COUNT({DIRECT})"), "2"), (format!("=AVERAGE({DIRECT})"), "1.5")] {
            let (_dir, mut worker, reader, source) = linked(&cell, 1, warm);
            assert_eq!(cells(&mut worker, &reader, "H1"), value, "{cell} warm {warm}");
            let written = formula(&mut worker, &reader);
            assert!(written.contains("S1':'S3'") || written.contains("S1:S3"), "{cell} warm {warm}: the last sheet stays in {written}");
            let listed = links::sources(&worker.values(&reader, links::LIST).expect("reads")).expect("parses");
            assert_eq!(listed, [crate::file_url(&source)], "{cell} warm {warm}: one link per source");
        }
    }
    // Other functions take one sheet: they say so.
    let (_dir, mut worker, reader, _) = linked(&format!("=INDEX({DIRECT},1,1)"), 1, true);
    assert_eq!(cells(&mut worker, &reader, "H1"), "Err:502", "no first sheet passes for the range");
}

#[test]
fn a_range_whose_blank_last_sheet_is_gone_keeps_its_value() {
    if skip() {
        return;
    }
    for range in [DIRECT, INDIRECT] {
        let (_dir, mut worker, reader, source) = linked(&format!("=SUM({range})"), 1, false);
        assert_eq!(cells(&mut worker, &reader, "H1"), "3", "{range}: S1 and S2 add up");
        // The blank S3 goes; S1 changes.
        std::fs::write(&source, spanned(&["S1", "S2"], 10)).expect("removes S3");
        assert_eq!(update(&mut worker, &reader), [crate::file_url(&source)], "{range}: the source is named");
        assert_eq!(cells(&mut worker, &reader, "H1"), "3", "{range}: the old value stays");
        // S3 back: the update reads the new values.
        std::fs::write(&source, spanned(&["S1", "S2", "S3"], 10)).expect("restores S3");
        assert_eq!(update(&mut worker, &reader), Vec::<String>::new(), "{range}: the update succeeds");
        assert_eq!(cells(&mut worker, &reader, "H1"), "12", "{range}: the new values");
    }
}
