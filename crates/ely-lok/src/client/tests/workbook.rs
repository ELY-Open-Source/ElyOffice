use super::sheet::{SHEETS, open_book};
use super::*;
use crate::sheet;

fn names(worker: &mut Worker, doc: &Doc) -> Vec<(String, bool)> {
    doc.sheets(worker)
        .expect("the sheets")
        .list
        .into_iter()
        .map(|sheet| (sheet.name, sheet.visible))
        .collect()
}

/// Runs `id` as an app does, at a fresh read.
fn run(worker: &mut Worker, doc: &Doc, id: &str, values: &[&str]) {
    let at = worker.now(doc).expect("the state");
    worker
        .run(&at, id, values)
        .unwrap_or_else(|error| panic!("{id} {values:?}: {error}"));
}

fn geometry(worker: &mut Worker, doc: &Doc) -> sheet::Geometry {
    sheet::geometry(&worker.values(doc, sheet::GEOMETRY).expect("the geometry"))
        .expect("the geometry parses")
}

#[test]
fn sheets_insert_rename_move_hide_and_delete() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_book(SHEETS);
    let owned = |list: &[(&str, bool)]| -> Vec<(String, bool)> {
        list.iter()
            .map(|&(name, shown)| (name.to_string(), shown))
            .collect()
    };
    let refs = sheet::references(
        &worker
            .values(&doc, &sheet::references_query(0))
            .expect("the references"),
    )
    .expect("they parse");
    assert_eq!((refs.formulas, refs.names), (1, 0), "U refers to S");
    assert_eq!(refs.dynamic, 1, "INDIRECT on U is counted apart");
    assert!(
        refs.first.contains("U"),
        "the first is on U: {}",
        refs.first
    );
    run(&mut worker, &doc, "sheet.insert", &["New", "2"]);
    run(&mut worker, &doc, "sheet.rename", &["Data", "2"]);
    assert_eq!(
        names(&mut worker, &doc),
        owned(&[("S", true), ("Data", true), ("T", false), ("U", true)])
    );
    doc.show_sheet(&mut worker, 1).expect("Data shows");
    run(&mut worker, &doc, "sheet.move", &["5", "false", "true"]);
    run(&mut worker, &doc, "sheet.move", &["5", "true", "true"]);
    let list = names(&mut worker, &doc);
    assert_eq!(list[..3], owned(&[("S", true), ("T", false), ("U", true)]));
    assert_eq!(list[3].0, "Data");
    assert!(list[4].0.starts_with("Data"), "a copy: {}", list[4].0);
    doc.show_sheet(&mut worker, 0).expect("S shows");
    run(&mut worker, &doc, "sheet.move", &["2", "false", "true"]);
    let order = |worker: &mut Worker| -> Vec<String> {
        names(worker, &doc)
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    };
    assert_eq!(
        order(&mut worker)[..3],
        ["T", "S", "U"],
        "Index i+2 moves right"
    );
    run(&mut worker, &doc, "sheet.move", &["1", "false", "true"]);
    assert_eq!(
        order(&mut worker)[..3],
        ["S", "T", "U"],
        "Index i moves left"
    );
    run(&mut worker, &doc, "sheet.hide", &["2"]);
    run(&mut worker, &doc, "sheet.show", &["T"]);
    let list = names(&mut worker, &doc);
    assert_eq!(list[1], ("T".to_string(), true), "T shows");
    assert_eq!(list[2], ("U".to_string(), false), "U hides");
    run(&mut worker, &doc, "sheet.delete", &["1"]);
    assert_eq!(names(&mut worker, &doc)[0].0, "T", "S is gone");
}

/// A 1, 2 column to fill, and cells to merge.
const GRID: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="G">
  <table:table-row><table:table-cell office:value-type="float" office:value="1"><text:p>1</text:p></table:table-cell><table:table-cell><text:p>b</text:p></table:table-cell></table:table-row>
  <table:table-row><table:table-cell office:value-type="float" office:value="2"><text:p>2</text:p></table:table-cell></table:table-row>
 </table:table></office:spreadsheet></office:body>
</office:document>"#;

#[test]
fn ranges_fill_size_hide_merge_and_take_formulas() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_book(GRID);
    run(&mut worker, &doc, "cell.goto", &["A1:A2"]);
    run(&mut worker, &doc, "fill.auto", &["A5"]);
    run(&mut worker, &doc, "cell.goto", &["C1"]);
    run(&mut worker, &doc, "cell.enter", &["=SUM(A1:A5)"]);
    run(&mut worker, &doc, "cell.goto", &["C1"]);
    let summed = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        seen.cell.address == "C1" && seen.cell.formula == "=SUM(A1:A5)"
    });
    assert_eq!(summed.cell.formula, "=SUM(A1:A5)");
    run(&mut worker, &doc, "cell.goto", &["F1:H1"]);
    run(&mut worker, &doc, "columns.width_selected", &["3000"]);
    run(&mut worker, &doc, "columns.width", &["2540", "4"]);
    run(&mut worker, &doc, "cell.goto", &["E1"]);
    run(&mut worker, &doc, "columns.hide", &[]);
    let sizes = geometry(&mut worker, &doc);
    assert_eq!(sizes.columns.size(3), 1440, "D is 1 inch wide");
    assert_eq!(sizes.columns.size(4), 0, "E hides");
    assert_eq!(
        (5..8)
            .map(|column| sizes.columns.size(column))
            .collect::<Vec<_>>(),
        [1701, 1701, 1701],
        "F to H all take 3 cm"
    );
    run(&mut worker, &doc, "select.column", &["1", "0"]);
    run(&mut worker, &doc, "select.row", &["2", "4096"]);
    run(&mut worker, &doc, "cell.goto", &["B2"]);
    run(&mut worker, &doc, "view.freeze", &[]);
    let frozen = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        let state = |uno: &str| {
            seen.states
                .iter()
                .rev()
                .find(|(name, _)| name == uno)
                .map(|(_, state)| state.clone())
        };
        state(".uno:FreezePanesColumn").as_deref() == Some("1 / 0")
            && state(".uno:FreezePanesRow").as_deref() == Some("1 / 0")
    });
    assert!(
        frozen
            .states
            .iter()
            .any(|(name, state)| name == ".uno:FreezePanesRow" && state == "1 / 0"),
        "B2 freezes one row and one column: {:?}",
        frozen.states
    );
    run(&mut worker, &doc, "cell.goto", &["A1"]);
    run(&mut worker, &doc, "rows.insert_above", &[]);
    run(&mut worker, &doc, "cell.goto", &["A2:B2"]);
    assert_eq!(
        doc.selection_text(&mut worker)
            .expect("the text")
            .trim_end(),
        "1\tb",
        "two cells hold data"
    );
    run(&mut worker, &doc, "cells.merge", &["true"]);
    let out = dir.path().join("out.fods");
    doc.save_as(&mut worker, &crate::file_url(&out), "fods")
        .expect("the save");
    let xml = std::fs::read_to_string(&out).expect("the save reads");
    assert!(
        xml.contains("table:number-columns-spanned=\"2\""),
        "A2:B2 merges"
    );
    assert!(
        xml.contains("of:=SUM([.A2:.A6])"),
        "the formula follows the inserted row"
    );
    assert!(
        xml.contains("office:value=\"14\""),
        "the fill reaches 5; the merged text drops out of the sum"
    );
    assert!(xml.contains("<text:p>1 b</text:p>"), "the contents move");
}

#[test]
fn two_workbooks_keep_their_own_sheets() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, first) = open_book(SHEETS);
    first.show_sheet(&mut worker, 2).expect("U shows");
    let grid = dir.path().join("grid.fods");
    std::fs::write(&grid, GRID).expect("the grid writes");
    let second = worker
        .load(&crate::file_url(&grid))
        .expect("the second book loads");
    first.show_sheet(&mut worker, 0).expect("S shows");
    assert_eq!(first.sheets(&mut worker).expect("sheets").current, 0);
    assert_eq!(second.sheets(&mut worker).expect("sheets").current, 0);
    run(&mut worker, &first, "cell.goto", &["C1"]);
    run(&mut worker, &first, "cell.enter", &["123456"]);
    let out = dir.path().join("first.fods");
    first
        .save_as(&mut worker, &crate::file_url(&out), "fods")
        .expect("the save");
    let xml = std::fs::read_to_string(&out).expect("the save reads");
    let on_s = &xml[..xml.find("table:name=\"T\"").expect("sheet T")];
    assert!(
        on_s.contains("office:value=\"123456\""),
        "the entry lands on S"
    );
}

#[test]
fn a_sheet_by_number_needs_the_state_it_was_read_in() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_book(SHEETS);
    let read = worker.now(&doc).expect("the state");
    worker.run(&read, "sheet.delete", &["1"]).expect("deletes S");
    // Sheet 2 was T when read; it is U now.
    let stale = worker.run(&read, "sheet.rename", &["Renamed", "2"]).expect_err("an older state");
    assert!(crate::is_stale(&stale), "{stale}");
    let only = |worker: &mut Worker| names(worker, &doc).into_iter().map(|(name, _)| name).collect::<Vec<_>>();
    assert_eq!(only(&mut worker), ["T", "U"], "nothing was renamed");
    run(&mut worker, &doc, "sheet.rename", &["Renamed", "2"]);
    assert_eq!(only(&mut worker), ["T", "Renamed"]);
    // A loaded handle never names a sheet after edits.
    assert!(worker.run(&doc, "sheet.delete", &["1"]).is_err());
}

#[test]
fn commands_from_one_read_run_together_and_hovering_changes_nothing() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_book(SHEETS);
    let read = worker.now(&doc).expect("the state");
    let hover = crate::Mouse { kind: 2, x: 500, y: 500, count: 0, buttons: 0, modifiers: 0 };
    worker.post_mouse(&read, hover).expect("hovers");
    assert_eq!(worker.now(&doc).expect("the state"), read, "a hover is no change");
    // Both numbers come from one read.
    let renames: [(&str, &[&str]); 2] = [("sheet.rename", &["One", "1"]), ("sheet.rename", &["Two", "2"])];
    worker.run_all(&read, &renames).expect("renames both");
    let only = |worker: &mut Worker| names(worker, &doc).into_iter().map(|(name, _)| name).collect::<Vec<_>>();
    assert_eq!(only(&mut worker), ["One", "Two", "U"]);
    let stale = worker.run(&read, "sheet.rename", &["Three", "3"]).expect_err("an older state");
    assert!(crate::is_stale(&stale), "{stale}");
    assert_eq!(only(&mut worker), ["One", "Two", "U"], "nothing else was renamed");
}

#[test]
fn a_header_act_after_a_click_is_not_stale() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_book(SHEETS);
    let read = worker.now(&doc).expect("the state");
    // A click moves the revision; rows and columns stay.
    for kind in [0, 1] {
        let click = crate::Mouse { kind, x: 500, y: 500, count: 1, buttons: 1, modifiers: 0 };
        worker.post_mouse(&read, click).expect("clicks");
    }
    assert_ne!(worker.now(&doc).expect("the state"), read, "the click counts as a change");
    worker.run(&read, "select.column", &["2", "0"]).expect("the column header still selects");
    worker.run(&read, "columns.width", &["800", "3"]).expect("the column's width still sets");
}


#[test]
fn the_selection_counts_its_filled_cells() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = super::sheet::open_book(super::input::FODS);
    let filled = |worker: &mut Worker, range: &str| {
        worker.run(&doc, "cell.goto", &[range]).expect("the range");
        let read = worker.values(&doc, sheet::SELECTED).expect("the selection");
        sheet::selected_range(&read)
            .expect("parses")
            .expect("one range")
            .filled
    };
    assert_eq!(filled(&mut worker, "H8:J9"), 0, "an empty range");
    assert_eq!(filled(&mut worker, "A1:C3"), 1, "A1 alone holds text");
}
