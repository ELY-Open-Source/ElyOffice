//! An unsaved tab's recovery point (X-07), written in a pause.

use gpui::{Entity, TestAppContext};

use ely_office::prelude::*;
use ely_office::{Autosaving as _, Recovering as _};

use super::{blank, commit, docs, settle, type_into, with_window};
use crate::docs::Docs;

/// Every point's file, in any slot.
fn point_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Ok(slots) = std::fs::read_dir(dir.join("recovery")) else { return Vec::new() };
    let slots = slots.filter_map(Result::ok).filter(|slot| slot.path().is_dir());
    let names = slots.filter_map(|slot| std::fs::read_dir(slot.path().join("points")).ok()).flatten();
    let names = names.filter_map(Result::ok).map(|name| name.path());
    names.filter(|path| path.extension().is_some_and(|ext| ext == "bin")).collect()
}

fn points(dir: &std::path::Path) -> usize {
    point_files(dir).len()
}

/// Each point's body text, as the kernel reads it.
fn point_texts(cx: &mut TestAppContext, docs: &Entity<Docs>, dir: &std::path::Path) -> Vec<String> {
    let worker = cx.update(|app| docs.read(app).core.worker.clone());
    let mut worker = worker.lock().expect("the worker");
    point_files(dir)
        .iter()
        .enumerate()
        .map(|(n, file)| {
            let copy = dir.join(format!("point{n}.docx"));
            std::fs::copy(file, &copy).expect("the point copies");
            let doc = worker.load(&ely_lok::file_url(&copy)).expect("the point opens");
            let fodt = dir.join(format!("point{n}.fodt"));
            let bytes = ely_office::remote::copy::<ely_office::Lok>(&mut worker, &doc, &fodt, "fodt").expect("reads");
            ely_office::remote::body_text(&bytes).expect("a body").trim().to_string()
        })
        .collect()
}

/// One tick of the autosave beat.
fn beat(cx: &mut TestAppContext, docs: &Entity<Docs>) {
    with_window(cx, docs, |docs, window, cx| docs.beat(window, cx));
}

fn writing(docs: &Entity<Docs>, app: &gpui::App) -> usize {
    docs.read(app).core.keeping.as_ref().expect("recovery").writing()
}

fn kept_docs(cx: &mut TestAppContext, dir: &std::path::Path) -> Option<(Entity<Docs>, std::path::PathBuf)> {
    let docs = docs(cx, dir)?;
    let kept = dir.join("kept");
    let keeping = ely_office::Keeping::in_folder(kept.clone()).expect("recovery starts");
    cx.update(|app| docs.update(app, |docs, _| docs.core.keeping = Some(keeping)));
    Some((docs, kept))
}

#[test]
fn an_unsaved_tab_gets_a_point() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, kept)) = kept_docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "kept");
    settle(&cx, |app| docs.read(app).tab(id).is_some_and(|tab| tab.dirty(app)));
    cx.update(|app| docs.update(app, |docs, cx| docs.keep_points(cx)));
    beat(&mut cx, &docs);
    beat(&mut cx, &docs);
    settle(&cx, |_| points(&kept) == 1);
}

#[test]
fn points_are_written_one_tab_at_a_time() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, kept)) = kept_docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    for text in ["one", "two", "three"] {
        blank(&mut cx, &docs);
        type_into(&mut cx, &docs, text);
    }
    cx.update(|app| docs.update(app, |docs, cx| docs.keep_points(cx)));
    beat(&mut cx, &docs);
    beat(&mut cx, &docs);
    // Snapshots queued at once keep a key waiting behind all.
    assert_eq!(cx.update(|app| writing(&docs, app)), 1, "one snapshot holds the engine at a time");
    settle(&cx, |app| points(&kept) == 3 && writing(&docs, app) == 0);
}

#[test]
fn typing_defers_the_point_to_the_next_pause() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, kept)) = kept_docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "one");
    blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "two");
    cx.update(|app| docs.update(app, |docs, cx| docs.keep_points(cx)));
    let none = |cx: &mut TestAppContext, when: &str| {
        assert_eq!(cx.update(|app| writing(&docs, app)), 0, "no snapshot {when}");
        assert_eq!(points(&kept), 0, "no point {when}");
    };
    beat(&mut cx, &docs);
    none(&mut cx, "before a whole beat of rest");
    type_into(&mut cx, &docs, "more");
    beat(&mut cx, &docs);
    none(&mut cx, "while typing goes on");
    beat(&mut cx, &docs);
    assert_eq!(cx.update(|app| writing(&docs, app)), 1, "the pause writes the first tab");
    // Typing comes back before the second tab's turn.
    commit(&mut cx, &docs, "x");
    settle(&cx, |app| points(&kept) == 1 && writing(&docs, app) == 0 && docs.read(app).active_tab().is_some_and(|tab| tab.view.read(app).settled()));
    beat(&mut cx, &docs);
    assert_eq!(cx.update(|app| writing(&docs, app)), 0, "the second tab waits while typed into");
    beat(&mut cx, &docs);
    settle(&cx, |app| points(&kept) == 2 && writing(&docs, app) == 0);
    let mut texts = point_texts(&mut cx, &docs, &kept);
    texts.sort();
    assert_eq!(texts, ["one", "twomorex"], "each point holds what was typed");
}

#[test]
fn a_key_never_waits_behind_a_point() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, kept)) = kept_docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "typing");
    // The tick lands mid-typing; the next key follows it.
    cx.update(|app| docs.update(app, |docs, cx| docs.keep_points(cx)));
    commit(&mut cx, &docs, "x");
    settle(&cx, |app| {
        assert_eq!(writing(&docs, app), 0, "a snapshot went ahead of the key");
        docs.read(app).active_tab().is_some_and(|tab| tab.view.read(app).settled())
    });
    assert_eq!(points(&kept), 0, "the point waits for a pause");
}
