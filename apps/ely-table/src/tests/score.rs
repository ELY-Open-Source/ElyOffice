//! The scoreboard lists and reads a book never saved.

use std::cell::RefCell;

use ely_office::prelude::*;
use gpui::TestAppContext;

use super::{blank, book, settle, update};

/// The scoreboard's answer, through its own route.
fn score(cx: &mut TestAppContext, words: &[&str]) -> Result<String, String> {
    let (ask, answered) = ely_ui::Ask::for_tests(&serde_json::json!(words).to_string());
    assert!(cx.update(|app| crate::score::route(ask, app)).is_none(), "taken");
    let answer = RefCell::new(None);
    settle(cx, |_| {
        let mut got = answer.borrow_mut();
        if got.is_none() {
            *got = answered.try_recv().ok();
        }
        got.is_some()
    });
    answer.into_inner().expect("answered")
}

/// The book's mark and the sheet's rows.
fn read(cx: &mut TestAppContext, key: &str, sheet: &str) -> (String, serde_json::Value) {
    let answer = score(cx, &["score.read", key, sheet]).expect("rows");
    let value: serde_json::Value = serde_json::from_str(&answer).expect("JSON");
    (value["mark"].as_str().expect("a mark").to_owned(), value["rows"].clone())
}

#[test]
fn an_unsaved_book_is_listed_and_read_by_its_key() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    blank(&mut cx, &book);
    let listed: serde_json::Value = serde_json::from_str(&score(&mut cx, &["score.list"]).expect("a list")).expect("JSON");
    assert_eq!(listed.as_array().map(Vec::len), Some(1), "{listed}");
    assert!(listed[0]["path"].is_null(), "never saved: {listed}");
    let key = listed[0]["book"].as_str().expect("a key").to_owned();
    let sheet = listed[0]["sheets"][0].as_str().expect("a sheet").to_owned();
    assert_eq!(read(&mut cx, &key, &sheet).1, serde_json::json!([]), "an empty sheet");
    for (cell, value) in [("A1", "Lions"), ("B1", "=1+2")] {
        update(&mut cx, &book, |book, cx| book.run_active("cell.set", &[&format!("{sheet}!{cell}"), value], cx));
    }
    cx.run_until_parked();
    assert_eq!(read(&mut cx, &key, &sheet).1, serde_json::json!([["Lions", "3"]]));
    assert!(score(&mut cx, &["score.read", "0:999", &sheet]).is_err(), "no such book");
    assert!(score(&mut cx, &["score.read", &key, "Nowhere"]).is_err(), "no such sheet");
    // A read at the current mark waits for a change.
    let mut mark = read(&mut cx, &key, &sheet).0;
    loop {
        let now = read(&mut cx, &key, &sheet).0;
        if now == mark {
            break;
        }
        mark = now;
    }
    let (held, answered) = ely_ui::Ask::for_tests(&serde_json::json!(["score.read", &key, &sheet, &mark]).to_string());
    assert!(cx.update(|app| crate::score::route(held, app)).is_none(), "taken");
    cx.run_until_parked();
    assert!(answered.try_recv().is_err(), "held while nothing changes");
    update(&mut cx, &book, |book, cx| book.run_active("cell.set", &[&format!("{sheet}!A1"), "Tigers"], cx));
    let got = RefCell::new(None);
    settle(&cx, |_| {
        let mut got = got.borrow_mut();
        if got.is_none() {
            *got = answered.try_recv().ok();
        }
        got.is_some()
    });
    let changed: serde_json::Value =
        serde_json::from_str(&got.into_inner().expect("answered").expect("rows")).expect("JSON");
    assert_ne!(changed["mark"], serde_json::json!(mark));
    assert_eq!(changed["rows"], serde_json::json!([["Tigers", "3"]]));
    // Other asks pass on untouched.
    let (other, _answered) = ely_ui::Ask::for_tests(r#"["docs"]"#);
    assert!(cx.update(|app| crate::score::route(other, app)).is_some());
}
