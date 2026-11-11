//! A new chart never lands on data or drawings.

use super::analysis::skip;
use super::sheet::open_book;
use super::*;
use crate::sheet::{self, Look};

const ONE: &str = r#"<table:table-cell office:value-type="float" office:value="1"><text:p>1</text:p></table:table-cell>"#;

/// S holds A1:C5; `other` adds sheets after it.
fn book(rtl: bool, other: &str) -> String {
    let direction = if rtl { "rl-tb" } else { "lr-tb" };
    let rows = format!(r#"<table:table-row table:number-rows-repeated="5">{ONE}{ONE}{ONE}</table:table-row>"#);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
 xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:automatic-styles><style:style style:name="Direction" style:family="table"><style:table-properties style:writing-mode="{direction}"/></style:style></office:automatic-styles>
 <office:body><office:spreadsheet><table:table table:name="S" table:style-name="Direction">{rows}</table:table>{other}</office:spreadsheet></office:body>
</office:document>"#
    )
}

/// The cell's rect, once the cursor shows it.
fn cell(worker: &mut Worker, doc: &Doc, name: &str) -> Rect {
    let before = poll_until(worker, doc, &Events::default(), std::time::Duration::ZERO, |_| true);
    worker.run(doc, "cell.goto", &[name]).expect("the cell");
    let seen = poll_until(worker, doc, &before, HANG, |seen| seen.cell.cursor.is_some() && seen.cell.cursor != before.cell.cursor);
    seen.cell.cursor.expect("the cell's rect")
}

fn source(worker: &mut Worker, doc: &Doc) -> String {
    worker.run(doc, "cell.goto", &["S!A1:C5"]).expect("the source");
    let read = worker.values(doc, sheet::SELECTED).expect("the selection");
    sheet::selected_range(&read).expect("parses").expect("one range").range
}

fn column() -> String {
    Look { kind: "column".into(), ..Look::default() }.to_json()
}

/// Inserts at `stand`; the new chart's rect.
fn insert(worker: &mut Worker, doc: &Doc, data: &str, stand: &str) -> Rect {
    cell(worker, doc, stand);
    let before = poll_until(worker, doc, &Events::default(), std::time::Duration::ZERO, |_| true);
    worker.run(doc, "chart.insert", &[data, "false", &column()]).expect("inserts");
    poll_until(worker, doc, &before, HANG, |seen| seen.graphic.is_some() && seen.graphic != before.graphic).graphic.expect("the chart")
}

fn apart(a: Rect, b: Rect) -> bool {
    a.0 >= b.0 + b.2 || a.0 + a.2 <= b.0 || a.1 >= b.1 + b.3 || a.1 + a.3 <= b.1
}

#[test]
fn a_right_to_left_chart_clears_its_data() {
    if skip() {
        return;
    }
    let (_dir, mut worker, doc) = open_book(&book(true, ""));
    let (first, last) = (cell(&mut worker, &doc, "A1"), cell(&mut worker, &doc, "C5"));
    let data = source(&mut worker, &doc);
    let chart = insert(&mut worker, &doc, &data, "A1");
    let block = (first.0.min(last.0), first.1, (first.0 + first.2).max(last.0 + last.2) - first.0.min(last.0), last.1 + last.3 - first.1);
    assert!(apart(chart, block) && chart.0 >= 0, "the chart {chart:?} covers {block:?} or leaves the sheet");
}

#[test]
fn a_second_chart_clears_the_first() {
    if skip() {
        return;
    }
    let (_dir, mut worker, doc) = open_book(&book(false, ""));
    let data = source(&mut worker, &doc);
    let first = insert(&mut worker, &doc, &data, "A1");
    let second = insert(&mut worker, &doc, &data, "A1");
    assert!(apart(first, second), "{second:?} covers {first:?}");
}

/// T: full above, a 100×25 gap at CW11, full below.
const GAPPED: &str = r#"<table:table table:name="T">
<table:table-row><table:table-cell table:number-columns-spanned="16384" table:number-rows-spanned="10"><text:p>TOP</text:p></table:table-cell><table:covered-table-cell table:number-columns-repeated="16383"/></table:table-row>
<table:table-row table:number-rows-repeated="9"><table:covered-table-cell table:number-columns-repeated="16384"/></table:table-row>
<table:table-row><table:table-cell table:number-columns-spanned="100" table:number-rows-spanned="25"><text:p>LEFT</text:p></table:table-cell><table:covered-table-cell table:number-columns-repeated="99"/><table:table-cell table:number-columns-repeated="100"/><table:table-cell table:number-columns-spanned="16184" table:number-rows-spanned="25"><text:p>RIGHT</text:p></table:table-cell><table:covered-table-cell table:number-columns-repeated="16183"/></table:table-row>
<table:table-row table:number-rows-repeated="24"><table:covered-table-cell table:number-columns-repeated="100"/><table:table-cell table:number-columns-repeated="100"/><table:covered-table-cell table:number-columns-repeated="16184"/></table:table-row>
<table:table-row><table:table-cell table:number-columns-spanned="16384" table:number-rows-spanned="1048541"><text:p>BOTTOM</text:p></table:table-cell><table:covered-table-cell table:number-columns-repeated="16383"/></table:table-row>
<table:table-row table:number-rows-repeated="1048540"><table:covered-table-cell table:number-columns-repeated="16384"/></table:table-row>
</table:table>"#;

#[test]
fn a_chart_finds_a_gap_that_starts_off_its_grid() {
    if skip() {
        return;
    }
    let (_dir, mut worker, doc) = open_book(&book(false, GAPPED));
    let (first, last) = (cell(&mut worker, &doc, "T!CW11"), cell(&mut worker, &doc, "T!GR35"));
    let data = source(&mut worker, &doc);
    let started = std::time::Instant::now();
    let chart = insert(&mut worker, &doc, &data, "T!A1");
    // A cell-sized step took 27 s here.
    assert!(started.elapsed() < std::time::Duration::from_secs(5), "the search took {:?}", started.elapsed());
    let inside = chart.0 >= first.0 && chart.1 >= first.1 && chart.0 + chart.2 <= last.0 + last.2 && chart.1 + chart.3 <= last.1 + last.3;
    assert!(inside, "the chart {chart:?} is not in CW11:GR35");
}

/// T: one merge over every cell.
const FULL: &str = r#"<table:table table:name="T">
<table:table-row><table:table-cell table:number-columns-spanned="16384" table:number-rows-spanned="1048576"><text:p>ALL</text:p></table:table-cell><table:covered-table-cell table:number-columns-repeated="16383"/></table:table-row>
<table:table-row table:number-rows-repeated="1048575"><table:covered-table-cell table:number-columns-repeated="16384"/></table:table-row>
</table:table>"#;

#[test]
fn no_room_refuses_the_chart_and_leaves_the_book() {
    if skip() {
        return;
    }
    let (_dir, mut worker, doc) = open_book(&book(false, FULL));
    let data = source(&mut worker, &doc);
    cell(&mut worker, &doc, "T!A1");
    let undo = worker.values(&doc, ".uno:Undo").expect("the history");
    let started = std::time::Instant::now();
    assert!(worker.run(&doc, "chart.insert", &[&data, "false", &column()]).is_err(), "a chart went on full cells");
    // A band-by-band search took 73 s here.
    assert!(started.elapsed() < std::time::Duration::from_secs(5), "the refusal took {:?}", started.elapsed());
    assert_eq!(worker.values(&doc, ".uno:Undo").expect("the worker answers"), undo, "the history moved");
    let seen = poll_until(&mut worker, &doc, &Events::default(), std::time::Duration::ZERO, |_| true);
    assert!(seen.graphic.is_none(), "a chart is selected: {:?}", seen.graphic);
}

/// T: full but CW11:GR60, whose rows 11–21 hold `filled`.
fn gap_under(filled: &str, hide: bool) -> String {
    let span = |n: usize, rows: usize| {
        format!(r#"<table:table-cell table:number-columns-spanned="{n}" table:number-rows-spanned="{rows}"><text:p>X</text:p></table:table-cell><table:covered-table-cell table:number-columns-repeated="{}"/>"#, n - 1)
    };
    let covered = |n: usize| format!(r#"<table:covered-table-cell table:number-columns-repeated="{n}"/>"#);
    let middle: String = (0..50)
        .map(|row| {
            // Row 21 hidden, if asked.
            let hidden = if hide && row == 10 { r#" table:visibility="collapse""# } else { "" };
            let (left, right) = if row == 0 { (span(100, 50), span(16184, 50)) } else { (covered(100), covered(16184)) };
            let gap = if row <= 10 { filled.repeat(100) } else { r#"<table:table-cell table:number-columns-repeated="100"/>"#.into() };
            format!("<table:table-row{hidden}>{left}{gap}{right}</table:table-row>")
        })
        .collect();
    format!(
        r#"<table:table table:name="T"><table:table-row>{}</table:table-row><table:table-row table:number-rows-repeated="9">{}</table:table-row>{middle}<table:table-row>{}</table:table-row><table:table-row table:number-rows-repeated="1048515">{}</table:table-row></table:table>"#,
        span(16384, 10),
        covered(16384),
        span(16384, 1048516),
        covered(16384)
    )
}

#[test]
fn hidden_rows_and_lone_comments_hide_no_room() {
    if skip() {
        return;
    }
    let note = r#"<table:table-cell><office:annotation><text:p>N</text:p></office:annotation></table:table-cell>"#;
    // Data-area navigation once jumped these to the sheet's end.
    for (filled, hide) in [(ONE, true), (note, false)] {
        let (_dir, mut worker, doc) = open_book(&book(false, &gap_under(filled, hide)));
        let (first, last) = (cell(&mut worker, &doc, "T!CW22"), cell(&mut worker, &doc, "T!GR60"));
        let data = source(&mut worker, &doc);
        let chart = insert(&mut worker, &doc, &data, "T!A1");
        let inside = chart.0 >= first.0 && chart.1 >= first.1 && chart.0 + chart.2 <= last.0 + last.2 && chart.1 + chart.3 <= last.1 + last.3;
        assert!(inside, "hidden {hide}: the chart {chart:?} is not in CW22:GR60");
    }
}
