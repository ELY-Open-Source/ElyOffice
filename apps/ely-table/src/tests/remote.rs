//! Other programs' saves when the tab goes (X-08).

use std::cell::RefCell;

use ely_office::prelude::*;
use gpui::{AppContext as _, TestAppContext};

use super::{blank, book, settle, update};

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
