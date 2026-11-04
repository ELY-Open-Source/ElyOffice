use super::input::open;
use super::menus::{DOC, find};
use super::*;

/// The first inner column line, once it differs from `from`.
fn column(worker: &mut Worker, doc: &Doc, seen: &Events, from: i32) -> Events {
    poll_until(worker, doc, seen, HANG, |seen| {
        seen.table.as_ref().is_some_and(|table| table.columns.inner[0].position != from)
    })
}

#[test]
fn a_border_reports_and_moves_in_one_step() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open(DOC);
    click(&mut worker, &doc, 150, 150);
    let mut seen = Events::default();
    let (x, y) = find(&mut worker, &doc, &mut seen, "left");
    click(&mut worker, &doc, x, y);
    seen = poll_until(&mut worker, &doc, &seen, HANG, |seen| seen.table.is_some());
    let table = seen.table.clone().expect("the cursor's table");
    let (columns, rows) = (&table.columns, &table.rows);
    assert_eq!(columns.inner.len(), 1, "two columns, one inner line");
    assert_eq!(rows.inner.len(), 1, "two rows, one inner line");
    let caret = seen.cursor.expect("a caret");
    let across = (columns.offset + columns.left, columns.offset + columns.inner[0].position);
    let down = (rows.offset + rows.left, rows.offset + rows.inner[0].position);
    assert!(across.0 < caret.0 && caret.0 < across.1, "the caret is in column one: {across:?} {caret:?}");
    assert!(down.0 <= caret.1 && caret.1 < down.1, "the caret is in row one: {down:?} {caret:?}");

    let read = worker.now(&doc).expect("the revision");
    let line = columns.inner[0].position;
    worker.run(&read, "table.border", &["column-middle", "0", "500"]).expect("the line moves");
    seen = column(&mut worker, &doc, &seen, line);
    let moved = seen.table.clone().expect("still in the table");
    assert_eq!(moved.columns.inner[0].position, line + 500, "500 twips right");
    let stale = worker.run(&read, "table.border", &["column-middle", "0", "500"]);
    assert!(stale.is_err_and(|error| crate::is_stale(&error)), "an old read is refused");

    worker.run(&doc, "edit.undo", &[]).expect("undo");
    seen = column(&mut worker, &doc, &seen, line + 500);
    let undone = seen.table.clone().expect("still in the table");
    assert_eq!(undone.columns.inner[0].position, line, "one undo restores it");

    let bottom = undone.rows.right;
    let read = worker.now(&doc).expect("the revision");
    worker.run(&read, "table.border", &["row-right", "0", "400"]).expect("the bottom moves");
    seen = poll_until(&mut worker, &doc, &seen, HANG, |seen| {
        seen.table.as_ref().is_some_and(|table| table.rows.right != bottom)
    });
    let taller = seen.table.expect("still in the table");
    assert_eq!(taller.rows.right, bottom + 400, "the last row grows");
}
