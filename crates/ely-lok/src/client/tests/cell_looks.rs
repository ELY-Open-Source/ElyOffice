//! Cell looks the format panel and header edges set (#15).

use super::sheet::open_book;
use super::*;

/// A long text in A1 over a short column.
const BOOK: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S">
 <table:table-row><table:table-cell><text:p>a much longer line than one column holds</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell><text:p>b</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell><text:p>c</text:p></table:table-cell></table:table-row>
 </table:table></office:spreadsheet></office:body>
</office:document>"#;

fn skip() -> bool {
    let missing = install().is_none();
    if missing {
        eprintln!("SKIP: no pinned LOK build");
    }
    missing
}

/// The cells of `range`, as the kernel copies them.
fn cells(worker: &mut Worker, doc: &Doc, range: &str) -> String {
    worker.run(doc, "cell.goto", &[range]).expect("the range");
    doc.selection_text(worker).expect("the text").replace("\r\n", "\n")
}

/// The sheet's sizes, in print twips.
pub(super) fn geometry(worker: &mut Worker, doc: &Doc) -> crate::sheet::Geometry {
    let json = worker.values(doc, crate::sheet::GEOMETRY).expect("geometry");
    crate::sheet::geometry(&json).expect("parsed")
}

/// The automatic styles' text properties, as saved.
fn text_looks(dir: &std::path::Path, worker: &mut Worker, doc: &Doc) -> String {
    let out = dir.join("text.fods");
    doc.save_as(worker, &crate::file_url(&out), "fods").expect("the save");
    let xml = std::fs::read_to_string(&out).expect("the saved book");
    let start = xml.find("<office:automatic-styles>").expect("cell formats");
    let looks = xml[start..].split("<style:text-properties").skip(1);
    looks.map(|text| text.split("/>").next().expect("a closed element")).collect()
}

#[test]
fn a_cell_takes_a_text_color_and_goes_back_to_automatic() {
    if skip() {
        return;
    }
    let (dir, mut worker, doc) = open_book(BOOK);
    worker.run(&doc, "cell.goto", &["A2"]).expect("A2");
    worker.run(&doc, "cells.text_color", &["16711680"]).expect("red text");
    assert!(text_looks(dir.path(), &mut worker, &doc).contains("fo:color=\"#ff0000\""));
    worker.run(&doc, "cells.text_color", &["-1"]).expect("automatic");
    // Automatic: the window's font color, the RGB kept beside.
    let cleared = text_looks(dir.path(), &mut worker, &doc);
    assert!(cleared.contains("style:use-window-font-color=\"true\""), "{cleared}");
}

#[test]
fn a_text_cell_keeps_its_digits_through_xlsx() {
    if skip() {
        return;
    }
    let (dir, mut worker, doc) = open_book(BOOK);
    worker.run(&doc, "cell.goto", &["H1:H2"]).expect("H1:H2");
    worker.run(&doc, "number.code", &["@"]).expect("text");
    worker.run(&doc, "cell.goto", &["H1"]).expect("H1");
    worker.run(&doc, "cell.enter", &["007"]).expect("007");
    worker.run(&doc, "cell.goto", &["H2"]).expect("H2");
    worker.run(&doc, "cell.enter", &["110105199001011234"]).expect("an ID");
    assert_eq!(cells(&mut worker, &doc, "H1:H2").trim(), "007\n110105199001011234");
    let out = dir.path().join("ids.xlsx");
    doc.save_as(&mut worker, &crate::file_url(&out), "xlsx").expect("the save");
    let again = worker.load(&crate::file_url(&out)).expect("it reopens");
    assert_eq!(cells(&mut worker, &again, "H1:H2").trim(), "007\n110105199001011234");
}

#[test]
fn an_edge_fits_its_own_column_and_row_and_keeps_the_cursor() {
    if skip() {
        return;
    }
    let (_dir, mut worker, doc) = open_book(BOOK);
    let before = geometry(&mut worker, &doc);
    worker.run(&doc, "rows.height", &["2000", "3"]).expect("row 3 tall");
    worker.run(&doc, "cell.goto", &["C5"]).expect("C5");
    worker.run(&doc, "columns.fit", &["0", "1"]).expect("A fits");
    worker.run(&doc, "rows.fit", &["0", "3"]).expect("row 3 fits");
    let after = geometry(&mut worker, &doc);
    assert!(after.columns.size(0) > before.columns.size(0) * 2, "A widens to its text");
    assert_eq!(after.columns.size(1), before.columns.size(1), "B stays");
    assert_eq!(after.rows.size(2), before.rows.size(2), "row 3 back to its text");
    // An entry lands where the cursor is.
    worker.run(&doc, "cell.enter", &["here"]).expect("an entry");
    assert_eq!(cells(&mut worker, &doc, "C5").trim(), "here", "the cursor stays");
}
