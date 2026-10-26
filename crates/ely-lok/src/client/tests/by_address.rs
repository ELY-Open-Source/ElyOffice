use super::sheet::open_sheet;
use super::*;
use crate::sheet::{cell_at, cell_at_query};

/// How long a command may take to report.
const SETTLE: std::time::Duration = std::time::Duration::from_secs(3);

#[test]
fn a_cell_reads_and_writes_by_address_and_the_cursor_stays() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_sheet();
    let read = |worker: &mut Worker, address: &str| cell_at(&worker.values(&doc, &cell_at_query(address))?);
    let a1 = read(&mut worker, "S!A1").expect("A1");
    assert_eq!((a1.text.as_str(), a1.input.as_str()), ("Hello 世界", "Hello 世界"), "{a1:?}");
    let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    worker.post_key(&doc, 0, 1027).expect("Right to B1");
    let b1 = poll_until(&mut worker, &doc, &base, SETTLE, |seen| seen.cell.address == "B1");
    assert_eq!(b1.cell.address, "B1");
    worker.run(&doc, "cell.set", &["S!C3", "=1+2"]).expect("C3 set");
    let c3 = read(&mut worker, "S!C3").expect("C3");
    assert_eq!((c3.text.as_str(), c3.input.as_str()), ("3", "=1+2"), "{c3:?}");
    let after = poll_until(&mut worker, &doc, &b1, SETTLE, |_| false);
    assert_eq!(after.cell.address, "B1", "the cursor stays");
    worker.run(&doc, "cell.set", &["S!B1", "here"]).expect("the current cell set");
    let current = poll_until(&mut worker, &doc, &after, SETTLE, |seen| seen.cell.formula == "here" && seen.a11y.text == "here");
    assert_eq!((current.cell.formula.as_str(), current.a11y.text.as_str()), ("here", "here"), "the input line follows");
    for bad in ["C3", "Nowhere!C3", "S!"] {
        let refused = read(&mut worker, bad).expect_err(bad);
        assert_eq!(refused.kind(), std::io::ErrorKind::InvalidInput, "{bad}: {refused}");
        assert!(worker.run(&doc, "cell.set", &[bad, "x"]).is_err(), "{bad} written");
    }
    worker.run(&doc, "edit.undo", &[]).expect("B1 undone");
    assert_eq!(read(&mut worker, "S!B1").expect("B1").input, "", "each write is one undo step");
    assert_eq!(read(&mut worker, "S!C3").expect("C3").input, "=1+2", "C3 waits its turn");
    worker.run(&doc, "edit.undo", &[]).expect("C3 undone");
    assert_eq!(read(&mut worker, "S!C3").expect("C3").input, "");
    worker.run(&doc, "cell.set", &["S!D4", "first
second"]).expect("two lines set");
    assert_eq!(read(&mut worker, "S!D4").expect("D4").input, "first
second");
    worker.run(&doc, "edit.undo", &[]).expect("two lines undone");
    assert_eq!(read(&mut worker, "S!D4").expect("D4").input, "", "two lines are one undo step");
    let at = worker.now(&doc).expect("the state");
    worker.run(&at, "sheet.insert", &["T", "2"]).expect("T added");
    doc.show_sheet(&mut worker, 0).expect("S shows");
    worker.run(&doc, "cell.set", &["T!A1", "far"]).expect("T!A1 set");
    assert_eq!(read(&mut worker, "T!A1").expect("T!A1").text, "far");
    assert_eq!(read(&mut worker, "S!A1").expect("A1").text, "Hello 世界", "the shown sheet is untouched");
}
