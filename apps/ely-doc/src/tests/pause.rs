//! A save starts at the next typing pause (R15.37).

use std::time::Duration;

use ely_office::prelude::*;
use gpui::{Entity, TestAppContext};

use super::{blank, commit, docs, settle, text_of, type_into, with_window, written};
use crate::docs::Docs;

/// A tab saved at `own.docx`, typed into since.
fn typed_file(cx: &mut TestAppContext, docs: &Entity<Docs>, dir: &std::path::Path) -> (u64, std::path::PathBuf, Vec<u8>) {
    let id = blank(cx, docs);
    let own = dir.join("own.docx");
    with_window(cx, docs, |docs, window, cx| docs.write_as(id, own.clone(), None, window, cx));
    written(cx, &own, None);
    type_into(cx, docs, "one");
    (id, own.clone(), std::fs::read(&own).expect("the saved file"))
}

fn exporting(docs: &Entity<Docs>, id: u64, app: &gpui::App) -> bool {
    docs.read(app).tab(id).expect("the tab").exporting
}

/// Runs what is ready; the clock stays.
fn run(cx: &TestAppContext) {
    for _ in 0..10_000 {
        if !cx.executor().tick() {
            return;
        }
    }
    panic!("the window never went idle");
}

/// Steps tasks until `done`; the clock never moves.
#[track_caller]
fn without_time(cx: &TestAppContext, done: impl Fn(&gpui::App) -> bool) {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while std::time::Instant::now() < deadline {
        if !cx.executor().tick() {
            std::thread::sleep(Duration::from_millis(5));
        }
        if cx.update(|app| done(app)) {
            return;
        }
    }
    panic!("nothing happened without the clock moving");
}

#[test]
fn a_save_that_asks_where_never_waits_for_a_pause() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "abc");
    commit(&mut cx, &docs, "d");
    without_time(&cx, |app| docs.read(app).tab(id).is_some_and(|tab| tab.view.read(app).settled()));
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_save(&ely_ui::commands::Save, window, cx));
    let state = cx.update(|app| docs.read(app).tab(id).expect("the tab").state_key(app));
    assert_ne!(state, "office.state.saving", "nothing is saved before a place is chosen");
    without_time(&cx, |_| cx.did_prompt_for_new_path());
}

#[test]
fn a_program_save_waiting_for_a_pause_hears_its_tab_close() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let (id, own, _) = typed_file(&mut cx, &docs, dir.path());
    commit(&mut cx, &docs, "two");
    run(&cx);
    let (ask, answered) = ely_ui::Ask::for_tests(&serde_json::json!(["save", own.display().to_string()]).to_string());
    cx.update(|app| docs.update(app, |docs, cx| docs.answer_request(ask, cx)));
    run(&cx);
    assert!(cx.update(|app| docs.read(app).tab(id).is_some_and(|tab| tab.pausing == 1)), "the save waits for a pause");
    cx.update(|app| docs.update(app, |docs, cx| docs.close_tab(id, cx)));
    cx.executor().advance_clock(Duration::from_secs(1));
    run(&cx);
    let answer = answered.try_recv().expect("answered once its pause ends");
    assert!(answer.is_err_and(|why| why.contains("closed")), "the program hears the tab went");
}

/// A program saves `own` after a key; it waits.
fn program_save(cx: &mut TestAppContext, docs: &Entity<Docs>, own: &std::path::Path) -> std::sync::mpsc::Receiver<Result<String, String>> {
    commit(cx, docs, "two");
    run(cx);
    let (ask, answered) = ely_ui::Ask::for_tests(&serde_json::json!(["save", own.display().to_string()]).to_string());
    cx.update(|app| docs.update(app, |docs, cx| docs.answer_request(ask, cx)));
    run(cx);
    answered
}

#[test]
fn a_save_as_ending_in_the_pause_never_answers_a_program() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let (id, own, before) = typed_file(&mut cx, &docs, dir.path());
    let answered = program_save(&mut cx, &docs, &own);
    let other = dir.path().join("other.docx");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, other.clone(), None, window, cx));
    without_time(&cx, |app| other.exists() && docs.read(app).tab(id).is_some_and(|tab| !tab.exporting && tab.name == "other.docx"));
    run(&cx);
    assert!(answered.try_recv().is_err(), "the Save As answered the program");
    assert_eq!(std::fs::read(&own).expect("own.docx"), before, "own.docx is not written");
    cx.executor().advance_clock(Duration::from_secs(1));
    let answer = std::cell::RefCell::new(None);
    settle(&cx, |_| {
        if answer.borrow().is_none() {
            *answer.borrow_mut() = answered.try_recv().ok();
        }
        answer.borrow().is_some()
    });
    assert!(answer.into_inner().expect("answered").is_ok(), "the program's own save answers");
    let saved = format!("office: saved {}", other.display());
    let saves = ely_ui::logs::kept().iter().filter(|line| line.contains(&saved)).count();
    assert_eq!(saves, 2, "the Save As, then the program's own write");
}

#[test]
fn a_person_and_a_program_saving_a_gone_file_both_hear_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let (_, own, _) = typed_file(&mut cx, &docs, dir.path());
    commit(&mut cx, &docs, "k");
    run(&cx);
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_save(&ely_ui::commands::Save, window, cx));
    let answered = program_save(&mut cx, &docs, &own);
    std::fs::remove_file(&own).expect("deleted outside");
    cx.executor().advance_clock(Duration::from_secs(1));
    let (asked, answer) = (std::cell::Cell::new(false), std::cell::RefCell::new(None));
    settle(&cx, |_| {
        asked.set(asked.get() || cx.did_prompt_for_new_path());
        if answer.borrow().is_none() {
            *answer.borrow_mut() = answered.try_recv().ok();
        }
        asked.get() && answer.borrow().is_some()
    });
    assert!(answer.into_inner().expect("answered").is_err(), "the program hears the file went");
}

#[test]
fn a_key_after_a_save_goes_first_and_into_the_file() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let (id, own, before) = typed_file(&mut cx, &docs, dir.path());
    // Typing goes on as another program asks.
    commit(&mut cx, &docs, "two");
    run(&cx);
    let (ask, answered) = ely_ui::Ask::for_tests(&serde_json::json!(["save", own.display().to_string()]).to_string());
    cx.update(|app| docs.update(app, |docs, cx| docs.answer_request(ask, cx)));
    run(&cx);
    let held = cx.update(|app| exporting(&docs, id, app)) || std::fs::read(&own).expect("the file") != before;
    assert!(!held, "the save went ahead of the typing");
    commit(&mut cx, &docs, "three");
    // A second ask in the pause joins the first.
    let (again, answered_again) = ely_ui::Ask::for_tests(&serde_json::json!(["save", own.display().to_string()]).to_string());
    cx.update(|app| docs.update(app, |docs, cx| docs.answer_request(again, cx)));
    settle(&cx, |app| {
        assert!(!exporting(&docs, id, app), "the save went ahead of the key");
        docs.read(app).tab(id).expect("the tab").view.read(app).settled()
    });
    written(&cx, &own, Some(&before));
    let answers = std::cell::RefCell::new(Vec::new());
    settle(&cx, |_| {
        let mut got = answers.borrow_mut();
        got.extend(answered.try_recv().ok());
        got.extend(answered_again.try_recv().ok());
        got.len() == 2
    });
    assert!(answers.into_inner().iter().all(Result::is_ok), "both programs hear the save");
    for _ in 0..150 {
        cx.executor().advance_clock(Duration::from_millis(20));
        run(&cx);
    }
    let saved = format!("office: saved {}", own.display());
    let saves = ely_ui::logs::kept().iter().filter(|line| line.contains(&saved)).count();
    assert_eq!(saves, 2, "the first Save As and one save for both programs");
    assert!(text_of(&mut cx, &docs, &own).contains("onetwothree"), "the key after the request is saved");
}

#[test]
fn typing_on_starts_the_save_at_the_cap() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let (id, own, before) = typed_file(&mut cx, &docs, dir.path());
    commit(&mut cx, &docs, "k");
    run(&cx);
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_save(&ely_ui::commands::Save, window, cx));
    let state = cx.update(|app| docs.read(app).tab(id).expect("the tab").state_key(app));
    assert_eq!(state, "office.state.saving", "a waiting save shows");
    assert!(!cx.update(|app| exporting(&docs, id, app)), "the save went ahead of the typing");
    // A key every 200 ms, never a pause.
    let mut keys = 1;
    let begun = |cx: &mut TestAppContext| cx.update(|app| exporting(&docs, id, app)) || std::fs::read(&own).expect("the file") != before;
    while !begun(&mut cx) {
        assert!(keys < 15, "no save after {keys} keys");
        cx.executor().advance_clock(Duration::from_millis(200));
        commit(&mut cx, &docs, "k");
        run(&cx);
        keys += 1;
    }
    assert_eq!(keys, 11, "the save began 2 s after it was asked");
    written(&cx, &own, Some(&before));
    run(&cx);
    let saved = ely_office::remote::body_text(text_of(&mut cx, &docs, &own).as_bytes()).expect("a body");
    // The key at the cap follows the save, unsaved.
    assert_eq!(saved.trim(), format!("one{}", "k".repeat(keys - 1)), "the keys before the cap are saved");
    settle(&cx, |app| docs.read(app).tab(id).is_some_and(|tab| tab.dirty(app) && !tab.exporting));
}
