use super::input::FODS;
use super::*;

/// How long a command may take to report.
const SETTLE: std::time::Duration = std::time::Duration::from_secs(3);

#[test]
fn a11y_follows_calc_cells() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_sheet();
    let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    worker.post_key(&doc, 0, 1027).expect("Right to B1");
    let b1 = poll_until(&mut worker, &doc, &base, SETTLE, |_| false);
    worker.post_key(&doc, 0, 1026).expect("Left to A1");
    let a1 = poll_until(&mut worker, &doc, &b1, HANG, |seen| {
        seen.a11y.text == "Hello 世界"
    });
    assert_eq!(
        a1.a11y.text, "Hello 世界",
        "the cell text bridges without editing"
    );
    worker.post_key(&doc, 0, 1027).expect("Right to B1 again");
    let empty = poll_until(&mut worker, &doc, &a1, HANG, |seen| {
        seen.a11y.text.is_empty()
    });
    assert_eq!(empty.a11y.text, "", "the empty B1 clears the bridge");
}

/// The FODS fixture in a fresh worker.
pub(super) fn open_sheet() -> (tempfile::TempDir, Worker, Doc) {
    let install = install().expect("the pinned LOK build");
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("in.fods");
    write(&source, FODS);
    let doc = worker
        .load(&crate::file_url(&source))
        .expect("the sheet loads");
    (dir, worker, doc)
}

#[test]
fn a_cancelled_cell_edit_restores_the_cell_text() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_sheet();
    let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    worker.post_key(&doc, 0, 769).expect("F2 edits A1");
    worker
        .post_key(&doc, 'Z' as u32, 537 | 0x1000)
        .expect("Z types");
    let typed = poll_until(&mut worker, &doc, &base, HANG, |seen| {
        seen.a11y.text.ends_with('Z')
    });
    assert_eq!(typed.a11y.text, "Hello 世界Z", "the edit bridges");
    worker.post_key(&doc, 0, 1281).expect("Escape cancels");
    let back = poll_until(&mut worker, &doc, &typed, HANG, |seen| {
        seen.a11y.text == "Hello 世界"
    });
    assert_eq!(back.a11y.text, "Hello 世界", "the cancel restores the cell");
}

#[test]
fn undo_and_redo_work_inside_a_cell_edit() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    for with_history in [false, true] {
        let (_dir, mut worker, doc) = open_sheet();
        let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
        if with_history {
            worker
                .run(&doc, "format.bold", &[])
                .expect("a document step");
        }
        worker.post_key(&doc, 0, 769).expect("F2 edits A1");
        worker
            .post_key(&doc, 'Z' as u32, 537 | 0x1000)
            .expect("Z types");
        let typed = poll_until(&mut worker, &doc, &base, HANG, |seen| {
            seen.a11y.text == "Hello 世界Z"
        });
        assert_eq!(typed.a11y.text, "Hello 世界Z");
        worker
            .run(&doc, "edit.undo", &[])
            .expect("undo in the edit");
        let undone = poll_until(&mut worker, &doc, &typed, HANG, |seen| {
            seen.a11y.text == "Hello 世界"
        });
        assert_eq!(undone.a11y.text, "Hello 世界", "history {with_history}");
        worker
            .run(&doc, "edit.redo", &[])
            .expect("redo in the edit");
        let redone = poll_until(&mut worker, &doc, &undone, HANG, |seen| {
            seen.a11y.text == "Hello 世界Z"
        });
        assert_eq!(redone.a11y.text, "Hello 世界Z", "history {with_history}");
    }
}

/// Three sheets; T hides, U refers to S.
pub(super) const SHEETS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:automatic-styles><style:style style:name="ta2" style:family="table"><style:table-properties table:display="false"/></style:style></office:automatic-styles>
 <office:body><office:spreadsheet>
  <table:table table:name="S"><table:table-row><table:table-cell><text:p>Hello 世界</text:p></table:table-cell></table:table-row></table:table>
  <table:table table:name="T" table:style-name="ta2"><table:table-row><table:table-cell/></table:table-row></table:table>
  <table:table table:name="U"><table:table-row><table:table-cell table:formula="of:=[S.A1]"/><table:table-cell table:formula="of:=INDIRECT(&quot;S.A1&quot;)"/></table:table-row></table:table>
 </office:spreadsheet></office:body>
</office:document>"#;

#[test]
fn sheets_list_switch_and_the_cell_reports() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_book(SHEETS);
    let sheets = doc.sheets(&mut worker).expect("the sheets");
    let names: Vec<_> = sheets
        .list
        .iter()
        .map(|sheet| (sheet.name.as_str(), sheet.visible))
        .collect();
    assert_eq!(names, [("S", true), ("T", false), ("U", true)]);
    assert_eq!(sheets.current, 0);
    let first = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        seen.cell.address == "A1" && seen.cell.formula == "Hello 世界" && seen.cell.cursor.is_some()
    });
    assert!(first.cell.cursor.is_some(), "the cell cursor shows on load");
    let geometry = crate::sheet::geometry(
        &worker
            .values(&doc, crate::sheet::GEOMETRY)
            .expect("the geometry"),
    )
    .expect("the geometry parses");
    worker.post_key(&doc, 0, 1027).expect("Right to B1");
    worker.post_key(&doc, 0, 1024).expect("Down to B2");
    worker.post_key(&doc, 0, 1024).expect("Down to B3");
    let b3 = poll_until(&mut worker, &doc, &first, HANG, |seen| {
        seen.cell.address == "B3"
    });
    let (x, y, _, _) = b3.cell.cursor.expect("the cell cursor");
    assert_eq!(
        (x, y),
        (geometry.columns.start(1), geometry.rows.start(2)),
        "the geometry places B3 where the kernel does"
    );
    let first = b3;
    doc.show_sheet(&mut worker, 2).expect("U shows");
    assert_eq!(doc.sheets(&mut worker).expect("the sheets").current, 2);
    let third = poll_until(&mut worker, &doc, &first, HANG, |seen| {
        seen.cell.formula.starts_with('=')
    });
    assert!(
        third.cell.formula.contains("S!A1"),
        "U's formula shows: {}",
        third.cell.formula
    );
    assert!(third.cell.sheets > first.cell.sheets, "the switch reports");
}

/// `text` loaded as a spreadsheet the view shows.
pub(super) fn open_book(text: &str) -> (tempfile::TempDir, Worker, Doc) {
    let install = install().expect("the pinned LOK build");
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("book.fods");
    write(&source, text);
    let doc = worker
        .load(&crate::file_url(&source))
        .expect("the book loads");
    doc.visible_area(&mut worker, (0, 0, 15000, 10000), (256, 3840))
        .expect("the area");
    (dir, worker, doc)
}

#[test]
fn the_scheme_is_no_edit() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_book(FODS);
    worker.run(&doc, "view.theme", &["COLOR_SCHEME_ELY_LIGHT"]).expect("the scheme runs");
    let undo = worker.values(&doc, ".uno:Undo").expect("the undo stack");
    let undo: serde_json::Value = serde_json::from_str(&undo).expect("undo JSON");
    assert_eq!(undo["actions"], "", "the scheme left undo steps");
}

#[test]
fn the_pointer_is_text_only_while_a_cell_edits() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_book(FODS);
    let hover = |worker: &mut Worker, x| {
        let at = Mouse { kind: 2, x, y: 120, count: 1, buttons: 0, modifiers: 0 };
        worker.post_mouse(&doc, at).expect("hovers");
    };
    hover(&mut worker, 400);
    let reading = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        !seen.pointer.is_empty()
    });
    assert_eq!(reading.pointer, "default", "a cell shows the arrow");
    worker.post_key(&doc, 0, 769).expect("F2 edits A1");
    hover(&mut worker, 500);
    let editing = poll_until(&mut worker, &doc, &reading, HANG, |seen| {
        seen.pointer == "text"
    });
    assert_eq!(editing.pointer, "text", "the edited cell shows the text pointer");
}

#[test]
fn pasted_lines_of_tabbed_text_fill_cells_without_a_dialog() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_sheet();
    for row in 3..=8 {
        for column in ["A", "B", "C"] {
            worker.run(&doc, "cell.set", &[&format!("S!{column}{row}"), "old"]).expect("old content");
        }
    }
    let mut seen = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    // Issue #56: Chinese lines, empty fields, a quoted line break.
    let blocks = [
        ("A3", "商品名称\t规格\t\nA4纸\t\t\n"),
        ("A6", "商品\t\"一行\n二行\"\t50\n另一件\t规格\t60\n"),
        ("A8", "x\t\"一行\n\"\t\"\n\"\n"),
    ];
    for (at, text) in blocks {
        worker.run(&doc, "cell.goto", &[at]).expect("the target");
        let pasted = worker.paste(&doc, "text/plain;charset=utf-8", text.as_bytes());
        seen = poll_until(&mut worker, &doc, &seen, SETTLE, |_| false);
        assert!(seen.blocked.is_empty(), "no dialog opens: {:?}", seen.blocked);
        pasted.expect("the text pastes");
    }
    let mut cell = |address: &str| {
        let json = worker.values(&doc, &crate::sheet::cell_at_query(&format!("S!{address}"))).expect("a cell");
        crate::sheet::cell_at(&json).expect("readable").text
    };
    let rows: Vec<Vec<String>> = (3..=8).map(|row| ["A", "B", "C"].map(|column| cell(&format!("{column}{row}"))).to_vec()).collect();
    assert_eq!(
        rows,
        [
            ["商品名称", "规格", ""],
            ["A4纸", "", ""],
            ["old", "old", "old"],
            ["商品", "一行\n二行", "50"],
            ["另一件", "规格", "60"],
            ["x", "一行\n", "\n"],
        ]
    );
}

#[test]
fn a_skipped_trailing_column_keeps_its_cell() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_sheet();
    worker.run(&doc, "cell.set", &["S!A3", "one\ttwo\t"]).expect("A3");
    worker.run(&doc, "cell.set", &["S!C3", "old"]).expect("C3");
    worker.run(&doc, "cell.goto", &["A3"]).expect("A3");
    // Text to Columns by tab, the third column skipped
    worker.run(&doc, "data.columns", &["9,34,76,1,3/9,0,false,true,false,false,false,0,false"]).expect("the split");
    let mut cell = |address: &str| {
        let json = worker.values(&doc, &crate::sheet::cell_at_query(&format!("S!{address}"))).expect("a cell");
        crate::sheet::cell_at(&json).expect("readable").text
    };
    assert_eq!(["A3", "B3", "C3"].map(&mut cell), ["one", "two", "old"]);
}
