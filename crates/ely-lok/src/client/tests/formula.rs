use super::input::FODS;
use super::*;

/// The FODS fixture, loaded and its cursor read.
fn open() -> (tempfile::TempDir, Worker, Doc, Events) {
    let install = install().expect("the pinned LOK build");
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("in.fods");
    write(&source, FODS);
    let doc = worker
        .load(&crate::file_url(&source))
        .expect("the sheet loads");
    let seen = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        seen.cell.cursor.is_some()
    });
    (dir, worker, doc, seen)
}

fn type_text(worker: &mut Worker, doc: &Doc, text: &str) {
    for c in text.chars() {
        worker.post_key(doc, c as u32, 0).expect("a key posts");
    }
}

/// The cells of `range`, as the kernel copies them.
fn values(worker: &mut Worker, doc: &Doc, range: &str) -> String {
    worker.run(doc, "cell.goto", &[range]).expect("the range");
    // Windows ends copied lines with CRLF.
    doc.selection_text(worker).expect("the text").replace("\r\n", "\n")
}

#[test]
fn typing_a_formula_offers_functions_usage_and_marks() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc, seen) = open();
    worker.post_key(&doc, 0, 1024).expect("Down to A2");
    type_text(&mut worker, &doc, "=SU");
    let offered = poll_until(&mut worker, &doc, &seen, HANG, |seen| {
        seen.cell.functions.iter().any(|f| f.name == "SUMIF")
    });
    let sum = offered.cell.functions.iter().find(|f| f.name == "SUM");
    assert!(
        sum.is_some_and(|f| f.signature.contains(", ")),
        "the signature takes the Excel separator: {sum:?}"
    );
    let taken = doc.complete_function(&mut worker, Some("SUMIF"), None);
    assert!(taken.expect("the completion"), "SUMIF is on offer");
    let done = poll_until(&mut worker, &doc, &offered, HANG, |seen| {
        seen.cell.formula == "=SUMIF()"
    });
    assert_eq!(done.cell.formula, "=SUMIF()", "the name completes");
    type_text(&mut worker, &doc, "A1,");
    let usage = poll_until(&mut worker, &doc, &done, HANG, |seen| {
        seen.cell.formula == "=SUMIF(A1,)" && seen.cell.usage.contains('►')
    });
    assert_eq!(usage.cell.formula, "=SUMIF(A1,)", "typing goes on inside");
    assert!(
        usage.cell.usage.contains("► Criteria"),
        "{}",
        usage.cell.usage
    );
    assert!(usage.cell.functions.is_empty(), "the list gives way");
    let b3 = usage.cell.cursor.expect("the cell cursor");
    let (x, y) = (b3.0 + b3.2 * 3 / 2, b3.1 + b3.3 * 3 / 2);
    for kind in [0, 1] {
        let click = Mouse {
            kind,
            x,
            y,
            count: 1,
            buttons: 1,
            modifiers: 0,
        };
        worker.post_mouse(&doc, click).expect("the click posts");
    }
    let picked = poll_until(&mut worker, &doc, &usage, HANG, |seen| {
        seen.cell.formula == "=SUMIF(A1,B3)" && !seen.cell.marks.is_empty()
    });
    assert_eq!(
        picked.cell.formula, "=SUMIF(A1,B3)",
        "a click picks the cell"
    );
    assert_eq!(picked.cell.marks.len(), 1, "the picked cell is marked");
    assert_eq!(picked.cell.marks[0].part, 0);
    type_text(&mut worker, &doc, "*2");
    let typed = poll_until(&mut worker, &doc, &picked, HANG, |seen| {
        seen.cell.formula == "=SUMIF(A1,B3*2)"
    });
    assert_eq!(
        typed.cell.formula, "=SUMIF(A1,B3*2)",
        "typing goes on after a pick"
    );
    let picked = typed;
    // The first Escape may only close the kernel's hint.
    worker.post_key(&doc, 0, 1281).expect("Escape");
    worker.post_key(&doc, 0, 1281).expect("Escape cancels");
    let done = poll_until(&mut worker, &doc, &picked, HANG, |seen| {
        seen.cell.marks.is_empty() && seen.cell.formula.is_empty()
    });
    assert!(done.cell.marks.is_empty(), "{:?}", done.cell);
    worker
        .run(&doc, "cell.enter", &["=A1+B3"])
        .expect("a formula");
    worker.run(&doc, "cell.goto", &["A2"]).expect("back to it");
    worker.post_key(&doc, 0, 769).expect("F2 edits it");
    let edited = poll_until(&mut worker, &doc, &done, HANG, |seen| {
        seen.cell.marks.len() == 2
    });
    assert_eq!(edited.cell.marks.len(), 2, "each reference is marked");
    worker.post_key(&doc, 0, 1280).expect("Enter commits");
    let entered = poll_until(&mut worker, &doc, &edited, HANG, |seen| {
        seen.cell.marks.is_empty()
    });
    assert!(entered.cell.marks.is_empty(), "the commit clears the marks");
}

#[test]
fn a_broken_formula_enters_as_typed_without_a_dialog() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc, seen) = open();
    worker.run(&doc, "cell.goto", &["C1"]).expect("C1");
    type_text(&mut worker, &doc, "=1+");
    worker.post_key(&doc, 0, 1280).expect("Enter");
    let entered = poll_until(&mut worker, &doc, &seen, HANG, |seen| {
        seen.cell.address == "C2"
    });
    assert!(
        entered.blocked.is_empty(),
        "no dialog: {:?}",
        entered.blocked
    );
    assert!(values(&mut worker, &doc, "C1").starts_with("Err:"));
}

#[test]
fn formulas_take_excel_syntax() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc, _) = open();
    let sheet = doc.sheets(&mut worker).expect("the sheets").list[0]
        .name
        .clone();
    for (cell, formula) in [
        ("D1", "=SUM(1,2)"),
        ("D2", &format!("='{sheet}'!D1*2")),
        ("D3", "=SUM($D$1:D2,1)"),
    ] {
        worker.run(&doc, "cell.goto", &[cell]).expect("the cell");
        worker
            .run(&doc, "cell.enter", &[formula])
            .expect("the entry");
    }
    assert_eq!(values(&mut worker, &doc, "D1:D3"), "3\n6\n10\n");
}

/// Enters each formula at its cell.
fn enter(worker: &mut Worker, doc: &Doc, cells: &[(&str, &str)]) {
    for (cell, formula) in cells {
        worker.run(doc, "cell.goto", &[cell]).expect("the cell");
        worker
            .run(doc, "cell.enter", &[formula])
            .expect("the entry");
    }
}

#[test]
fn the_kernel_computes_each_family_of_functions() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc, _) = open();
    enter(
        &mut worker,
        &doc,
        &[
            ("F1", "3"),
            ("F2", "1"),
            ("F3", "2"),
            ("F4", "3"),
            ("G1", "=F1^2+F2*4-F3/2"),
            ("G2", "=IF(AND(F1>2,NOT(F2=2)),\"yes\",\"no\")"),
            ("G3", "=COUNTIF(F1:F4,3)+SUMIFS(F1:F4,F1:F4,\">1\")"),
            (
                "G4",
                "=CONCAT(LEFT(\"Ely Office\",3),\"-\",TEXT(0.25,\"0%\"))",
            ),
            (
                "G5",
                "=DATEDIF(DATE(2026,1,31),EDATE(DATE(2026,1,31),1),\"d\")",
            ),
            (
                "G6",
                "=XLOOKUP(2,F2:F4,F2:F4*10)+INDEX(F1:F4,MATCH(1,F1:F4,0))",
            ),
            ("G7", "=SUM(SORT(UNIQUE(F1:F4),1,-1))"),
            ("G8", "=ROWS(FILTER(F1:F4,F1:F4>1))*SUM(SEQUENCE(3))"),
        ],
    );
    assert_eq!(
        values(&mut worker, &doc, "G1:G8"),
        "12\nyes\n10\nEly-25%\n28\n21\n6\n18\n"
    );
}

#[test]
fn manual_calculation_waits_and_survives_a_save() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc, seen) = open();
    enter(&mut worker, &doc, &[("F1", "1"), ("F2", "=F1*2")]);
    assert_eq!(values(&mut worker, &doc, "F2"), "2");
    worker.run(&doc, "calc.auto", &["false"]).expect("manual");
    let manual = poll_until(&mut worker, &doc, &seen, HANG, |seen| {
        seen.states
            .contains(&(".uno:AutomaticCalculation".into(), "false".into()))
    });
    assert!(
        manual
            .states
            .contains(&(".uno:AutomaticCalculation".into(), "false".into())),
        "the state follows"
    );
    enter(&mut worker, &doc, &[("F1", "5")]);
    assert_eq!(values(&mut worker, &doc, "F2"), "2", "the formula waits");
    worker.run(&doc, "calc.now", &[]).expect("a recalc");
    assert_eq!(values(&mut worker, &doc, "F2"), "10", "a recalc updates");
    for format in ["ods", "xlsx", "xls", "fods"] {
        let path = dir.path().join(format!("manual.{format}"));
        doc.save_as(&mut worker, &crate::file_url(&path), format)
            .expect("the save");
        let again = worker
            .load(&crate::file_url(&path))
            .expect("the copy loads");
        let state = poll_until(&mut worker, &again, &Events::default(), HANG, |seen| {
            seen.states
                .iter()
                .any(|(name, _)| name == ".uno:AutomaticCalculation")
        });
        let auto = state
            .states
            .iter()
            .rfind(|(name, _)| name == ".uno:AutomaticCalculation")
            .map(|(_, value)| value.as_str());
        assert_eq!(auto, Some("false"), "{format} keeps manual calculation");
        worker.close_doc(again).expect("the copy closes");
    }
}

#[test]
fn a_circular_reference_reports_its_error() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc, seen) = open();
    enter(&mut worker, &doc, &[("F1", "=F2+1"), ("F2", "=F1")]);
    worker.run(&doc, "cell.goto", &["F1"]).expect("F1");
    let state = poll_until(&mut worker, &doc, &seen, HANG, |seen| {
        seen.states
            .iter()
            .any(|(name, value)| name == ".uno:StateTableCell" && value.contains("Circular"))
    });
    assert!(
        state
            .states
            .iter()
            .any(|(_, value)| value == "Error: Circular reference"),
        "{:?}",
        state.states
    );
    assert_eq!(values(&mut worker, &doc, "F1"), "Err:522");
}

#[test]
fn names_are_defined_changed_removed_and_flag_breaks() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc, _) = open();
    let sheet = doc.sheets(&mut worker).expect("the sheets").list[0]
        .name
        .clone();
    let read = |worker: &mut Worker| {
        crate::sheet::names(&worker.values(&doc, crate::sheet::NAMES).expect("names"))
            .expect("the names parse")
    };
    enter(&mut worker, &doc, &[("F1", "2"), ("F2", "5")]);
    let range = format!("'{sheet}'!$F$1");
    worker
        .run(&doc, "names.define", &["Rate", &range, ""])
        .expect("a workbook name");
    worker
        .run(&doc, "names.define", &["Local", "$F$2", &sheet])
        .expect("a sheet name");
    enter(&mut worker, &doc, &[("G1", "=Rate*10+Local")]);
    assert_eq!(values(&mut worker, &doc, "G1"), "25");
    let names = read(&mut worker);
    let rate = names.iter().find(|name| name.name == "Rate").expect("Rate");
    assert_eq!((rate.scope.as_str(), rate.broken), ("", false));
    assert!(
        names
            .iter()
            .any(|name| name.name == "Local" && name.scope == sheet)
    );
    let moved = format!("'{sheet}'!$F$2");
    worker
        .run(&doc, "names.define", &["Rate", &moved, ""])
        .expect("a new range");
    assert_eq!(values(&mut worker, &doc, "G1"), "55", "the range moves");
    worker.run(&doc, "edit.undo", &[]).expect("an undo");
    assert_eq!(values(&mut worker, &doc, "G1"), "25", "the change undoes");
    assert!(
        worker
            .run(&doc, "names.define", &["A1", "$F$1", ""])
            .is_err(),
        "a cell address is no name"
    );
    let check = |worker: &mut Worker, name: &str| {
        let query = crate::sheet::name_check_query(name);
        crate::sheet::name_check(&worker.values(&doc, &query).expect("a check"))
            .expect("the check parses")
    };
    assert_eq!(check(&mut worker, "Tax_2026"), "valid");
    assert_eq!(check(&mut worker, "B2"), "address");
    assert_eq!(check(&mut worker, "a b&c"), "characters");
    worker.run(&doc, "cell.goto", &["Rate"]).expect("a jump");
    assert_eq!(
        poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
            seen.cell.address == "Rate"
        })
        .cell
        .address,
        "Rate",
        "the name box jumps to a name and shows it"
    );
    worker.run(&doc, "cell.goto", &["F1"]).expect("F1");
    worker.run(&doc, "rows.delete", &[]).expect("row 1 goes");
    let names = read(&mut worker);
    assert!(
        names.iter().any(|name| name.name == "Rate" && name.broken),
        "a deleted target breaks the name: {names:?}"
    );
    worker
        .run(&doc, "names.remove", &["Local", &sheet])
        .expect("a removal");
    assert!(!read(&mut worker).iter().any(|name| name.name == "Local"));
}

#[test]
fn selection_figures_follow_the_cells_number_format() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc, seen) = open();
    enter(&mut worker, &doc, &[("H1", "8128.33333333333"), ("H2", "8128.33333333333")]);
    worker.run(&doc, "cell.goto", &["H1:H2"]).expect("both");
    worker.run(&doc, "number.code", &["0.00"]).expect("two decimals");
    let figures = |seen: &Events| {
        seen.states.iter().rfind(|(name, _)| name == ".uno:StateTableCell").map(|(_, value)| value.clone())
    };
    let state = poll_until(&mut worker, &doc, &seen, HANG, |seen| figures(seen).is_some_and(|v| v.contains("8128.33;")));
    assert!(figures(&state).is_some_and(|v| v.contains("Average: 8128.33;")), "{:?}", figures(&state));
}
