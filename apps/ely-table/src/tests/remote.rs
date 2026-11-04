//! Other programs' saves when the tab goes (X-08).

use std::cell::RefCell;

use ely_office::prelude::*;
use gpui::{AppContext as _, TestAppContext};

use super::{blank, book, settle, update};

/// The answer to `words`, once it comes.
fn ask(cx: &mut TestAppContext, book: &gpui::Entity<crate::book::Book>, words: &[&str]) -> Result<String, String> {
    let (ask, answered) = ely_ui::Ask::for_tests(&serde_json::json!(words).to_string());
    update(cx, book, |book, cx| book.answer(ask, cx));
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

#[test]
fn a_hidden_book_set_from_outside_reads_nothing_until_shown() {
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
    blank(&mut cx, &book);
    cx.run_until_parked();
    let reads = |app: &gpui::App| book.read(app).tab(id).expect("the tab").extra.read_at.clone();
    let view = |app: &gpui::App| book.read(app).tab(id).expect("the tab").view.read(app).read_at();
    let (before, seen) = (cx.update(|app| reads(app)), cx.update(|app| view(app)));
    let file = path.display().to_string();
    for (cell, value) in [("A1", "7"), ("A2", "=A1*6")] {
        assert_eq!(ask(&mut cx, &book, &["set", &file, &format!("{sheet}!{cell}"), value]), Ok(String::new()));
    }
    // The hidden view takes the kernel's report first.
    settle(&cx, |app| view(app).newer_than(&seen));
    cx.run_until_parked();
    assert_eq!(cx.update(|app| reads(app)), before, "a hidden book reads nothing");
    let got = ask(&mut cx, &book, &["get", &file, &format!("{sheet}!A2")]).expect("read");
    assert!(got.contains("42"), "{got}");
    assert!(ask(&mut cx, &book, &["save", &file]).is_ok(), "saved");
    update(&mut cx, &book, |book, cx| {
        book.core.set_active(Some(id));
        cx.notify();
    });
    // Shown, it reads the state the sets left.
    settle(&cx, |app| reads(app).is_some_and(|at| !view(app).newer_than(&at)));
}

#[test]
fn a_save_asked_for_a_tab_that_closed_mid_export_is_answered() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    let path = dir.path().join("book.xlsx");
    let id = cx.update(|app| book.read(app).core.tabs[0].id);
    cx.update_window(window.into(), |_, window, app| {
        book.update(app, |book, cx| book.write_as(id, path.clone(), None, window, cx))
    })
    .expect("the window lives");
    settle(&cx, |app| book.read(app).core.tabs[0].name == "book.xlsx");
    let request = serde_json::json!(["save", path.display().to_string()]).to_string();
    let (ask, answered) = ely_ui::Ask::for_tests(&request);
    cx.update_window(window.into(), |_, window, app| {
        book.update(app, |book, cx| {
            book.save(id, None, window, cx);
            book.answer(ask, cx);
        })
    })
    .expect("the window lives");
    // The program's save waits for the running export.
    settle(&cx, |app| book.read(app).core.queued_saves.len() == 1);
    update(&mut cx, &book, |book, cx| book.close_tab(id, cx));
    let answer = RefCell::new(None);
    settle(&cx, |_| {
        let mut got = answer.borrow_mut();
        if got.is_none() {
            *got = answered.try_recv().ok();
        }
        got.is_some()
    });
    let answer = answer.into_inner().expect("answered");
    assert!(answer.is_err(), "{answer:?}");
}

#[test]
fn a_running_save_asked_for_a_tab_that_closes_is_answered() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    let path = dir.path().join("book.xlsx");
    let id = cx.update(|app| book.read(app).core.tabs[0].id);
    cx.update_window(window.into(), |_, window, app| {
        book.update(app, |book, cx| book.write_as(id, path.clone(), None, window, cx))
    })
    .expect("the window lives");
    settle(&cx, |app| book.read(app).core.tabs[0].name == "book.xlsx");
    let request = serde_json::json!(["save", path.display().to_string()]).to_string();
    let (ask, answered) = ely_ui::Ask::for_tests(&request);
    update(&mut cx, &book, |book, cx| book.answer(ask, cx));
    // The program's own save runs now; nothing queues.
    settle(&cx, |app| book.read(app).exporting(id));
    let ended = std::rc::Rc::new(std::cell::Cell::new(false));
    let marker = ended.clone();
    update(&mut cx, &book, |book, cx| {
        assert!(book.core.queued_saves.is_empty());
        book.after_export(id, move |_, _, _| marker.set(true));
        book.close_tab(id, cx);
    });
    settle(&cx, |_| ended.get());
    let answer = answered.try_recv().expect("answered when the export ended");
    assert!(answer.is_err(), "{answer:?}");
}
