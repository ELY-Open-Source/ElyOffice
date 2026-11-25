use super::sheet::open_sheet;
use super::*;
use crate::sheet::range::{self, Cell, Refused};

/// How long a command may take to report.
const SETTLE: std::time::Duration = std::time::Duration::from_secs(3);

#[test]
fn a_range_reads_and_writes_by_address_in_one_undo_step() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_sheet();
    let read = |worker: &mut Worker, address: &str| range::read(&worker.values(&doc, &range::read_query(address))?);
    let target = |worker: &mut Worker, address: &str, rows, columns| {
        range::target(&worker.values(&doc, &range::target_query(address, rows, columns))?)
    };
    let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    worker.post_key(&doc, 0, 1027).expect("Right to B1");
    let b1 = poll_until(&mut worker, &doc, &base, HANG, |seen| seen.cell.address == "B1");
    let to = target(&mut worker, "S!B2", 2, 2).expect("readable").expect("one cell grows to the data");
    let rows = vec![vec!["1".to_string(), "=B2*10".into()], vec!["two words".into(), "世界 😀".into()]];
    worker.run(&doc, "range.set", &[&to, &range::rows_json(&rows)]).expect("B2:C3 set");
    let block = read(&mut worker, "S!B2:C3").expect("readable").expect("B2:C3");
    assert_eq!(block.rows[0][1], Cell { text: "10".into(), input: "=B2*10".into() }, "{block:?}");
    assert_eq!((block.rows[1][0].text.as_str(), block.rows[1][1].text.as_str()), ("two words", "世界 😀"));
    let after = poll_until(&mut worker, &doc, &b1, SETTLE, |_| false);
    assert_eq!(after.cell.address, "B1", "the cursor stays");
    let used = read(&mut worker, "S").expect("readable").expect("the used area");
    assert_eq!((used.rows.len(), used.rows[0].len()), (3, 3), "A1 to the last filled cell: {used:?}");
    assert_eq!(used.rows[0][0].text, "Hello 世界");
    let one = target(&mut worker, "S!B2", 1, 1).expect("readable").expect("B2");
    worker.run(&doc, "range.set", &[&one, &range::rows_json(&[vec![String::new()]])]).expect("B2 cleared");
    let cleared = read(&mut worker, "S!B2").expect("readable").expect("B2");
    assert_eq!(cleared.rows[0][0].input, "", "an empty string clears");
    worker.run(&doc, "edit.undo", &[]).expect("the clear undone");
    assert_eq!(read(&mut worker, "S!B2").expect("readable").expect("B2").rows[0][0].input, "1");
    worker.run(&doc, "edit.undo", &[]).expect("the block undone");
    let undone = read(&mut worker, "S!B2:C3").expect("readable").expect("B2:C3");
    assert!(undone.rows.iter().flatten().all(|cell| cell.input.is_empty()), "the block is one undo step: {undone:?}");
    assert_eq!(target(&mut worker, "S!B2:C3", 1, 2).expect("readable"), Err(Refused::Shape));
    assert_eq!(target(&mut worker, "S!XFD1", 1, 2).expect("readable"), Err(Refused::Edge));
    for bad in ["C3", "Nowhere!C3", "S!"] {
        assert_eq!(read(&mut worker, bad).expect("readable"), Err(Refused::Address), "{bad}");
    }
    assert_eq!(read(&mut worker, "S!A1:Z50000").expect("readable"), Err(Refused::Size(1_300_000)));
    let wrong = range::rows_json(&[vec!["x".into()]]);
    assert!(worker.run(&doc, "range.set", &["S!B2:C3", &wrong]).is_err(), "the kernel checks the size too");
    assert!(worker.run(&doc, "range.set", &["S!B2", "not rows"]).is_err(), "rows must be JSON");
    assert_eq!(read(&mut worker, "S!B2").expect("readable").expect("B2").rows[0][0].input, "", "nothing written");
}
