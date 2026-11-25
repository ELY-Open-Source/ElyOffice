//! Another program changes sheets, rows, columns and merges (#107).

use ely_office::prelude::*;
use gpui::{AppContext as _, TestAppContext};

use super::{blank, book, remote::ask, settle, update};

#[test]
fn structure_set_from_outside_shows_in_the_tabs_and_undoes() {
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
    let file = path.display().to_string();
    let sheets = |cx: &mut TestAppContext| -> serde_json::Value {
        serde_json::from_str(&ask(cx, &book, &["sheet.list", &file]).expect("listed")).expect("JSON")
    };
    let tabs = |app: &gpui::App| {
        let sheets = &book.read(app).core.tabs[0].extra.sheets;
        (sheets.list.iter().map(|sheet| sheet.name.clone()).collect::<Vec<_>>(), sheets.current)
    };
    assert_eq!(sheets(&mut cx), serde_json::json!([{ "name": sheet, "used": null, "hidden": false }]));

    assert_eq!(ask(&mut cx, &book, &["sheet.add", &file, "First", "1"]), Ok(String::new()));
    settle(&cx, |app| tabs(app).0.len() == 2);
    let (names, current) = cx.update(|app| tabs(app));
    assert_eq!(names, ["First".to_string(), sheet.clone()], "the tabs show the new sheet");
    assert_eq!(names[current], sheet, "the shown sheet stays");
    update(&mut cx, &book, |book, cx| book.run_active("edit.undo", &[], cx));
    settle(&cx, |app| tabs(app).0.len() == 1);

    let at = |cell: &str| format!("{sheet}!{cell}");
    let cells = serde_json::json!([["a", "b"], ["c", "d"]]).to_string();
    assert_eq!(ask(&mut cx, &book, &["range.set", &file, &at("A1"), &cells]), Ok(String::new()));
    assert_eq!(ask(&mut cx, &book, &["rows.insert", &file, &at("1"), ""]), Ok(String::new()));
    let read = |cx: &mut TestAppContext, address: &str| -> String {
        let json: serde_json::Value = serde_json::from_str(&ask(cx, &book, &["range.get", &file, address]).expect("read")).expect("JSON");
        json["rows"][0][0]["input"].as_str().expect("input").to_owned()
    };
    assert_eq!(read(&mut cx, &at("A2")), "a", "one row above, written Sheet!1");
    assert_eq!(ask(&mut cx, &book, &["columns.width", &file, &at("B"), "2.54cm"]), Ok(String::new()));
    assert_eq!(ask(&mut cx, &book, &["range.merge", &file, &at("A2:B2"), ""]), Ok(String::new()));
    let over = ask(&mut cx, &book, &["range.merge", &file, &at("B2:C2"), ""]);
    assert_eq!(over, Err(format!("{}: already contains or overlaps merged cells; unmerge first", at("B2:C2"))));
    assert_eq!(ask(&mut cx, &book, &["range.unmerge", &file, &at("A2:B2"), ""]), Ok(String::new()));
    assert_eq!(read(&mut cx, &at("B2")), "b", "the hidden cell kept its text");
    assert_eq!(ask(&mut cx, &book, &["sheet.freeze", &file, &at("B2"), ""]), Ok(String::new()));

    let (row, block, one, empty) = (at("1:1"), at("A1:B2"), at("C3"), at("C3:D4"));
    let refusals = [
        (vec!["sheet.remove", &file, &sheet, ""], format!("{sheet}: cannot remove the last visible sheet")),
        (vec!["sheet.add", &file, &sheet, ""], format!("{sheet}: a sheet with this name already exists")),
        (vec!["sheet.add", &file, "X", "9"], "position 9 is out of range (must be 1 to 2)".to_string()),
        (vec!["sheet.rename", &file, "Nope", "Y"], "Nope: no such sheet".to_string()),
        (vec!["rows.delete", &file, "Nope!1", ""], "Nope: no such sheet".to_string()),
        (vec!["columns.insert", &file, &row, ""], format!("{row}: invalid column address (use e.g. Sheet1!C or Sheet1!C:E)")),
        (vec!["sheet.freeze", &file, &block, ""], format!("{block}: freeze requires a single cell (e.g. Sheet1!B2)")),
        (vec!["range.merge", &file, &one, ""], format!("{one}: single cell; nothing to merge")),
        (vec!["range.unmerge", &file, &empty, ""], format!("{empty}: no merged cells in this range")),
    ];
    for (words, want) in refusals {
        assert_eq!(ask(&mut cx, &book, &words), Err(want), "{words:?}");
    }
    let listed = sheets(&mut cx);
    assert_eq!(listed[0]["used"].as_str().map(|used| used.ends_with("$B$3")), Some(true), "A1 to B3: {listed}");
}
