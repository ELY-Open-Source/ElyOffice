use super::cell_looks::geometry;
use super::sheet::{SHEETS, open_book};
use super::*;
use crate::sheet::{self, range, structure::{self, Refused}};

/// How long a command may take to report.
const SETTLE: std::time::Duration = std::time::Duration::from_secs(3);

/// The structure check's answer for `verb`.
fn check(worker: &mut Worker, doc: &Doc, verb: &str, address: &str, value: &str) -> Result<String, Refused> {
    structure::checked(&worker.values(doc, &structure::check_query(verb, address, value)).expect("checked")).expect("readable")
}

/// Checks `verb`, then runs it as ElyTable does.
fn change(worker: &mut Worker, doc: &Doc, verb: &str, address: &str, value: &str) {
    check(worker, doc, verb, address, value).unwrap_or_else(|why| panic!("{verb} {address}: {why:?}"));
    let command = if verb == "sheet.freeze" { "structure.freeze" } else { "structure" };
    worker.run(doc, command, &[verb, address, value]).unwrap_or_else(|error| panic!("{verb} {address}: {error}"));
}

fn names(worker: &mut Worker, doc: &Doc) -> Vec<String> {
    let sheets = structure::sheets(&worker.values(doc, structure::SHEETS).expect("the sheets")).expect("readable");
    sheets.into_iter().map(|sheet| sheet.name).collect()
}

fn input(worker: &mut Worker, doc: &Doc, address: &str) -> String {
    let read = range::read(&worker.values(doc, &range::read_query(address)).expect("read")).expect("readable");
    read.expect("a range").rows[0][0].input.clone()
}

fn undo(worker: &mut Worker, doc: &Doc) {
    worker.run(doc, "edit.undo", &[]).expect("undone");
}

#[test]
fn sheets_rows_columns_and_merges_change_by_address_one_undo_step_each() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_book(SHEETS);
    let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    worker.post_key(&doc, 0, 1027).expect("Right to B1");
    let b1 = poll_until(&mut worker, &doc, &base, HANG, |seen| seen.cell.address == "B1");
    let shown = |worker: &mut Worker| {
        let read = worker.values(&doc, sheet::SELECTED).expect("the selection");
        sheet::selected_range(&read).expect("readable").expect("one range").range
    };
    let (selected, first) = (shown(&mut worker), names(&mut worker, &doc));
    let listed = structure::sheets(&worker.values(&doc, structure::SHEETS).expect("listed")).expect("readable");
    assert!(listed[0].used.is_some(), "S holds text: {listed:?}");

    change(&mut worker, &doc, "rows.insert", "S!1:2", "");
    assert_eq!(input(&mut worker, &doc, "S!A3"), "Hello 世界", "two rows above");
    change(&mut worker, &doc, "rows.delete", "S!1:2", "");
    assert_eq!(input(&mut worker, &doc, "S!A1"), "Hello 世界");
    change(&mut worker, &doc, "columns.insert", "S!A:A", "");
    assert_eq!(input(&mut worker, &doc, "S!B1"), "Hello 世界", "a column before A");
    undo(&mut worker, &doc);
    assert_eq!(input(&mut worker, &doc, "S!A1"), "Hello 世界", "the insert is one undo step");

    change(&mut worker, &doc, "columns.width", "S!D:E", "1440");
    change(&mut worker, &doc, "rows.height", "S!3:3", "567");
    let sizes = geometry(&mut worker, &doc);
    assert_eq!((sizes.columns.size(3), sizes.columns.size(4)), (1440, 1440), "D and E are 1 inch");
    assert_eq!(sizes.rows.size(2), 567, "row 3 is 1 cm");
    change(&mut worker, &doc, "columns.width", "S!A:A", "auto");

    worker.run(&doc, "range.set", &["S!B2:C2", &range::rows_json(&[vec!["b".into(), "c".into()]])]).expect("two cells");
    change(&mut worker, &doc, "range.merge", "S!B2:C2", "");
    assert_eq!(check(&mut worker, &doc, "range.merge", "S!C2:D2", ""), Err(Refused::Merged), "overlaps the merge");
    change(&mut worker, &doc, "range.unmerge", "S!B2:C2", "");
    assert_eq!(input(&mut worker, &doc, "S!C2"), "c", "the hidden cell kept its contents");

    change(&mut worker, &doc, "sheet.freeze", "S!B2", "");
    // Row and column report apart; both must arrive.
    let both = |seen: &Events| {
        [".uno:FreezePanesRow", ".uno:FreezePanesColumn"]
            .iter()
            .all(|axis| seen.states.iter().rev().find(|(name, _)| name == axis).is_some_and(|(_, state)| state == "1 / 0"))
    };
    let frozen = poll_until(&mut worker, &doc, &b1, HANG, both);
    assert!(both(&frozen), "{:?}", frozen.states);
    change(&mut worker, &doc, "sheet.freeze", "U!A2", "");
    change(&mut worker, &doc, "sheet.freeze", "S!A1", "");

    let after = poll_until(&mut worker, &doc, &frozen, SETTLE, |_| false);
    assert_eq!(after.cell.address, "B1", "the cursor stays");
    assert_eq!(shown(&mut worker), selected, "the selection and shown sheet stay");

    let refusals = [
        ("sheet.remove", "Nope", "", Refused::Sheet("Nope".into())),
        ("rows.insert", "Nope!1:1", "", Refused::Sheet("Nope".into())),
        ("sheet.add", "S", "", Refused::Taken),
        ("sheet.add", "a/b", "", Refused::Name),
        ("sheet.rename", "S", "", Refused::Name),
        ("sheet.add", "X", "99", Refused::Position(first.len() as u64 + 1)),
        ("rows.delete", "S!A1:B2", "", Refused::Address),
        ("columns.insert", "S!1:1", "", Refused::Address),
        ("sheet.freeze", "S!A1:B2", "", Refused::Address),
        ("rows.height", "S!1:1", "0", Refused::Size),
        ("range.merge", "S!B2", "", Refused::Single),
        ("range.unmerge", "S!B2:C2", "", Refused::Unmerged),
    ];
    for (verb, address, value, want) in refusals {
        assert_eq!(check(&mut worker, &doc, verb, address, value), Err(want), "{verb} {address} {value}");
    }
    worker.run(&doc, "sheet.protect", &[&sheet::protect_json("pw", &[])]).expect("protects S");
    let locked = Err(Refused::Protected("S".into()));
    assert_eq!(check(&mut worker, &doc, "rows.insert", "S!1:1", ""), locked);
    assert_eq!(check(&mut worker, &doc, "range.merge", "S!A1:B1", ""), locked);
    worker.run(&doc, "sheet.protect", &[&sheet::unprotect_json("pw")]).expect("unprotects S");
    worker.run(&doc, "range.set", &["S!A1048576", &range::rows_json(&[vec!["bottom".into()]])]).expect("the last row");
    assert_eq!(check(&mut worker, &doc, "rows.insert", "S!1:1", ""), Err(Refused::Edge));
    assert!(worker.run(&doc, "structure", &["rows.insert", "S!1:1", ""]).is_err(), "the kernel checks it too");
    assert_eq!(input(&mut worker, &doc, "S!A1048576"), "bottom", "nothing pushed off the sheet");

    // Undoing a removal shows that sheet, as Calc does.
    change(&mut worker, &doc, "sheet.add", "New", "1");
    assert_eq!(names(&mut worker, &doc)[0], "New", "at position 1");
    assert_eq!(shown(&mut worker), selected, "an added sheet is not shown");
    undo(&mut worker, &doc);
    assert_eq!(names(&mut worker, &doc), first, "the add is one undo step");
    change(&mut worker, &doc, "sheet.add", "Last", "");
    change(&mut worker, &doc, "sheet.rename", "Last", "Renamed");
    assert_eq!(names(&mut worker, &doc).last().map(String::as_str), Some("Renamed"));
    change(&mut worker, &doc, "sheet.remove", "Renamed", "");
    assert_eq!(names(&mut worker, &doc), first);
    undo(&mut worker, &doc);
    assert!(names(&mut worker, &doc).contains(&"Renamed".to_string()), "the remove is undone");

    let others: Vec<String> = names(&mut worker, &doc).into_iter().filter(|name| name != "S").collect();
    for other in &others {
        change(&mut worker, &doc, "sheet.remove", other, "");
    }
    assert_eq!(check(&mut worker, &doc, "sheet.remove", "S", ""), Err(Refused::Last("S".into())));
    for _ in &others {
        undo(&mut worker, &doc);
    }
}
