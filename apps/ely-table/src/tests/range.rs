//! Another program reads and fills a block of cells (#107).

use ely_office::prelude::*;
use gpui::{AppContext as _, TestAppContext};

use super::{blank, book, remote::ask, settle, update};

#[test]
fn a_block_set_from_outside_reads_back_and_undoes_in_one_step() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    blank(&mut cx, &book);
    let path = dir.path().join("book.xlsx");
    let id = cx.update(|app| book.read(app).core.tabs[0].id);
    cx.update_window(window.into(), |_, window, app| {
        book.update(app, |book, cx| book.write_as(id, path.clone(), None, window, cx))
    })
    .expect("the window lives");
    settle(&cx, |app| book.read(app).core.tabs[0].name == "book.xlsx" && !book.read(app).exporting(id));
    let sheet = cx.update(|app| book.read(app).core.tabs[0].extra.sheets.list[0].name.clone());
    let (file, at) = (path.display().to_string(), |cell: &str| format!("{sheet}!{cell}"));
    let read = |cx: &mut TestAppContext, address: &str| -> serde_json::Value {
        let json = ask(cx, &book, &["range.get", &file, address]).expect("read");
        serde_json::from_str(&json).expect("JSON")
    };
    let rows = serde_json::json!([["1", "=B2*10"], ["two words", "世界 😀"]]).to_string();
    assert_eq!(ask(&mut cx, &book, &["range.set", &file, &at("B2"), &rows]), Ok(String::new()));
    let block = read(&mut cx, &at("B2:C3"));
    assert_eq!((&block["rows"][0][1]["text"], &block["rows"][0][1]["input"]), (&"10".into(), &"=B2*10".into()), "{block}");
    assert_eq!(block["rows"][1][1]["text"], "世界 😀");
    let used = read(&mut cx, &sheet);
    let shape: Vec<usize> = used["rows"].as_array().expect("rows").iter().map(|row| row.as_array().expect("a row").len()).collect();
    assert_eq!(shape, [3, 3, 3], "A1 to the last filled cell: {used}");
    update(&mut cx, &book, |book, cx| book.run_active("edit.undo", &[], cx));
    cx.run_until_parked();
    let undone = read(&mut cx, &at("B2:C3"));
    let inputs: Vec<&str> = undone["rows"].as_array().expect("rows").iter().flat_map(|row| row.as_array().expect("a row")).map(|cell| cell["input"].as_str().expect("input")).collect();
    assert_eq!(inputs, ["", "", "", ""], "one undo step: {undone}");
    let x = serde_json::json!([["x"]]).to_string();
    let shape = ask(&mut cx, &book, &["range.set", &file, &at("B2:C3"), &x]);
    assert_eq!(shape, Err(format!("{}: range does not match data size (1 rows, 1 columns)", at("B2:C3"))));
    let xy = serde_json::json!([["x", "y"]]).to_string();
    let edge = ask(&mut cx, &book, &["range.set", &file, &at("XFD1"), &xy]);
    assert_eq!(edge, Err(format!("{}: data runs past the end of the sheet (1 rows, 2 columns)", at("XFD1"))));
    assert!(ask(&mut cx, &book, &["range.get", &file, "Nowhere!A1"]).is_err(), "no such sheet");
    let big = ask(&mut cx, &book, &["range.get", &file, &at("A1:Z50000")]);
    assert_eq!(big, Err(format!("{}: 1,300,000 cells exceeds the 1,000,000-cell limit", at("A1:Z50000"))));
    assert!(ask(&mut cx, &book, &["range.set", &file, &at("B2"), "[]"]).is_err(), "no rows");
}
