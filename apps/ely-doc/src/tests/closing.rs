//! Closing never waits for the engine (SF-18).

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

use ely_office::{After, prelude::*};
use gpui::{Entity, TestAppContext};

use super::{docs, settle, type_into, with_window};
use crate::docs::Docs;
use crate::files::Template;

/// A document file in `dir`.
fn file(cx: &mut TestAppContext, dir: &Path) -> PathBuf {
    let file = dir.join("open.fodt");
    let fodt = cx.update(|app| Template::Report.fodt(app));
    std::fs::write(&file, fodt).expect("the file writes");
    file
}

/// Holds the engine's turn until the sender drops.
fn hold(worker: &Arc<Mutex<ely_lok::Worker>>) -> mpsc::Sender<()> {
    let (release, wait) = mpsc::channel::<()>();
    let (held, locked) = mpsc::channel();
    let worker = Arc::clone(worker);
    std::thread::spawn(move || {
        futures::executor::block_on(ely_lok::behind(&worker, move |_| {
            held.send(()).expect("the test waits");
            let _ = wait.recv();
        }));
    });
    locked.recv().expect("the turn is held");
    release
}

fn worker(cx: &mut TestAppContext, docs: &Entity<Docs>) -> Arc<Mutex<ely_lok::Worker>> {
    cx.update(|app| Arc::clone(&docs.read(app).core.worker))
}

#[test]
fn a_clean_tab_closes_while_the_engine_is_busy() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let file = file(&mut cx, dir.path());
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(file.clone(), cx));
    settle(&cx, |app| docs.read(app).core.tabs.len() == 1);
    let id = cx.update(|app| docs.read(app).core.tabs[0].id);
    let release = hold(&worker(&mut cx, &docs));
    with_window(&mut cx, &docs, |docs, window, cx| docs.request_close(id, After::Tab, window, cx));
    let (open, asked) = cx.update(|app| (docs.read(app).core.tabs.len(), docs.read(app).core.dialog.is_some()));
    assert_eq!((open, asked), (0, false), "the tab left with the engine held");
    drop(release);
    cx.run_until_parked();
}

#[test]
fn a_write_behind_an_undo_stays_unsaved_and_counts() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let file = file(&mut cx, dir.path());
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(file.clone(), cx));
    settle(&cx, |app| docs.read(app).core.tabs.len() == 1);
    type_into(&mut cx, &docs, "x");
    let (id, view) = cx.update(|app| (docs.read(app).core.tabs[0].id, docs.read(app).core.tabs[0].view.clone()));
    let before = cx.update(|app| view.read(app).edited());
    let release = hold(&worker(&mut cx, &docs));
    // Undo makes Writer clean; the write follows it.
    cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.undo", &[], cx)));
    let write = |worker: &mut ely_lok::Worker, doc: &ely_lok::Doc| worker.run(doc, "text.replace", &["a", "false", "3", "0", "0", "1", "b", "true"]);
    let written = Arc::new(Mutex::new(None));
    let landed = Arc::clone(&written);
    with_window(&mut cx, &docs, |docs, _, cx| docs.write_in(id, write, move |_, done, _, _| *landed.lock().expect("the slot") = Some(done.is_ok()), cx));
    with_window(&mut cx, &docs, |docs, window, cx| docs.request_close(id, After::Tab, window, cx));
    let asked = cx.update(|app| docs.read(app).core.dialog.is_some());
    assert!(asked, "the close asks while the write is out");
    // The user keeps the tab.
    cx.update(|app| docs.update(app, |docs, _| docs.core.dialog = None));
    drop(release);
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    while cx.update(|app| !view.read(app).settled() || written.lock().expect("the slot").is_none()) {
        assert!(cx.update(|app| docs.read(app).core.tabs[0].dirty(app)), "dirty at every step");
        assert!(std::time::Instant::now() < deadline, "the write never landed");
        if !cx.executor().tick() {
            cx.executor().advance_clock(Duration::from_millis(20));
        }
    }
    assert_eq!(*written.lock().expect("the slot"), Some(true), "the write landed");
    settle(&cx, |app| docs.read(app).core.tabs[0].modified);
    assert_eq!(cx.update(|app| view.read(app).edited()), before + 2, "undo and write both count");
}

#[test]
fn a_write_alone_counts_and_stays_unsaved() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let file = file(&mut cx, dir.path());
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(file.clone(), cx));
    settle(&cx, |app| docs.read(app).core.tabs.len() == 1);
    let (id, view) = cx.update(|app| (docs.read(app).core.tabs[0].id, docs.read(app).core.tabs[0].view.clone()));
    let before = cx.update(|app| view.read(app).edited());
    let write = |worker: &mut ely_lok::Worker, doc: &ely_lok::Doc| worker.run(doc, "text.replace", &["a", "false", "3", "0", "0", "1", "b", "true"]);
    with_window(&mut cx, &docs, |docs, _, cx| docs.write_in(id, write, |_, done, _, _| done.expect("the write lands"), cx));
    // Unsaved while queued; counted once run.
    assert!(cx.update(|app| docs.read(app).core.tabs[0].dirty(app)), "unsaved once queued");
    settle(&cx, |app| view.read(app).settled() && docs.read(app).core.tabs[0].modified);
    assert_eq!(cx.update(|app| view.read(app).edited()), before + 1, "the write counts");
}

#[test]
fn a_closed_load_lets_go_and_keeps_the_engine() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let file = file(&mut cx, dir.path());
    let release = hold(&worker(&mut cx, &docs));
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(file.clone(), cx));
    let id = cx.update(|app| docs.read(app).core.loading[0].0);
    with_window(&mut cx, &docs, |docs, window, cx| docs.request_close(id, After::Tab, window, cx));
    assert!(cx.update(|app| docs.read(app).core.loading.is_empty()), "the loading tab left");
    // The load never took the engine: no swap.
    cx.executor().advance_clock(Duration::from_secs(3));
    cx.run_until_parked();
    assert!(cx.update(|app| docs.read(app).core.shared.get().stopped.is_none()), "the engine stays");
    drop(release);
    cx.run_until_parked();
    let tabs = cx.update(|app| docs.read(app).core.tabs.len());
    assert_eq!(tabs, 0, "the closed load opens nothing");
    // The same engine opens the next file.
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(file.clone(), cx));
    settle(&cx, |app| docs.read(app).core.tabs.len() == 1);
}

#[test]
fn a_launch_tab_closed_before_its_turn_closes_the_early_load() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let file = file(&mut cx, dir.path());
    let early = ely_office::lok::test_load_early(&worker(&mut cx, &docs), &file);
    let release = hold(&worker(&mut cx, &docs));
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(file.clone(), cx));
    let id = cx.update(|app| docs.read(app).core.loading[0].0);
    with_window(&mut cx, &docs, |docs, window, cx| docs.request_close(id, After::Tab, window, cx));
    drop(release);
    cx.run_until_parked();
    let worker = worker(&mut cx, &docs);
    assert!(early.size(&mut worker.lock().expect("the worker")).is_err(), "the engine let it go");
}

#[test]
fn a_swapped_engine_reopens_the_other_tabs() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let file = file(&mut cx, dir.path());
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(file.clone(), cx));
    settle(&cx, |app| docs.read(app).core.tabs.len() == 1);
    let id = cx.update(|app| docs.read(app).core.tabs[0].id);
    // A restart signs as the app's author, set at start.
    worker(&mut cx, &docs).lock().expect("the worker").set_author("Ada").expect("signs");
    with_window(&mut cx, &docs, |docs, _, cx| docs.swap_engine(99, cx));
    assert!(cx.update(|app| docs.read(app).core.shared.get().stopped.is_some()), "the swap shows");
    settle(&cx, |app| docs.read(app).core.shared.get().stopped.is_none() && docs.read(app).core.tabs.len() == 1);
    let reopened = cx.update(|app| docs.read(app).core.tabs[0].id);
    assert_ne!(reopened, id, "the tab came back on the new engine");
}
