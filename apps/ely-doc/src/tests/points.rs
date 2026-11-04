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

#[test]
fn an_autosave_keeps_the_round_going() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, kept)) = kept_docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let saved = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "saved");
    let path = dir.path().join("saved.docx");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(saved, path.clone(), None, window, cx));
    settle(&cx, |app| docs.read(app).tab(saved).is_some_and(|tab| !tab.exporting && !tab.dirty(app)));
    type_into(&mut cx, &docs, "more");
    blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "unsaved");
    cx.update(|app| ely_ui::settings::switches::set(ely_ui::settings::switches::AUTOSAVE, true, app));
    cx.update(|app| docs.update(app, |docs, cx| docs.keep_points(cx)));
    beat(&mut cx, &docs);
    beat(&mut cx, &docs);
    let exporting = cx.update(|app| docs.read(app).tab(saved).is_some_and(|tab| tab.exporting));
    assert!(exporting, "the saved tab autosaves in the pause");
    // No input since: the unsaved tab follows without a beat.
    settle(&cx, |app| points(&kept) == 2 && writing(&docs, app) == 0);
}

/// A flat document of about two megabytes of text.
fn big_fodt(cx: &mut TestAppContext, dir: &std::path::Path) -> std::path::PathBuf {
    let rows: String = (0..20_000).map(|n| format!("<text:p>Row {n} holds text that makes the document large.</text:p>")).collect();
    let fodt = cx.update(|app| crate::files::Template::Blank.fodt(app));
    let path = dir.join("big.fodt");
    std::fs::write(&path, fodt.replace("</office:text>", &format!("{rows}</office:text>"))).expect("the file writes");
    path
}

fn refused(docs: &Entity<Docs>, app: &gpui::App) -> usize {
    docs.read(app).core.keeping.as_ref().expect("recovery").refused()
}

/// An owed round, then a pause of two beats.
fn round(cx: &mut TestAppContext, docs: &Entity<Docs>) {
    cx.update(|app| docs.update(app, |docs, cx| docs.keep_points(cx)));
    beat(cx, docs);
    beat(cx, docs);
}

fn limits(megabytes: u64) -> serde_json::Value {
    serde_json::json!({ "keep": 20, "megabytes": megabytes })
}

fn open_big(cx: &mut TestAppContext, docs: &Entity<Docs>, dir: &std::path::Path) {
    let file = big_fodt(cx, dir);
    with_window(cx, docs, |docs, _, cx| docs.open_file(file, cx));
    settle(cx, |app| docs.read(app).active_tab().is_some_and(|tab| tab.view.read(app).settled()));
}

#[test]
fn a_file_opened_over_its_space_takes_no_snapshot() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, kept)) = kept_docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let root = super::words::rooted(&mut cx, &docs);
    ely_platform::settings::Store::of(&kept).set("recovery", limits(1)).expect("the space is set");
    open_big(&mut cx, &docs, dir.path());
    type_into(&mut cx, &docs, "big");
    round(&mut cx, &docs);
    // Its size on disk tells before any snapshot.
    assert_eq!(cx.update(|app| writing(&docs, app)), 0, "no snapshot of a file over the space");
    settle(&cx, |app| refused(&docs, app) == 1);
    assert_eq!(points(&kept), 0);
    let never = cx.update(|app| ely_ui::i18n::text(app, "office.stopped.never", &[]).to_string());
    let args = [("name", "big.fodt"), ("limit", "1"), ("at", never.as_str())];
    let said = cx.update(|app| ely_ui::i18n::text(app, "office.recovery.too_big", &args));
    let toasts = cx.update(|app| root.read(app).toasts(app));
    assert!(toasts.iter().any(|(text, _)| *text == said), "{toasts:?}");
}

#[test]
fn a_reopened_copy_names_the_point_it_came_from() {
    use ely_office::Closing as _;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, kept)) = kept_docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let root = super::words::rooted(&mut cx, &docs);
    open_big(&mut cx, &docs, dir.path());
    type_into(&mut cx, &docs, "big");
    round(&mut cx, &docs);
    settle(&cx, |app| points(&kept) == 1 && writing(&docs, app) == 0);
    // The engine's restart: the copy takes over the point.
    cx.update(|app| {
        docs.update(app, |docs, cx| {
            let id = docs.core.active().expect("a tab");
            let taken = docs.take_slot(id, "big.fodt".into()).expect("its slot");
            docs.release_tab(id, cx);
            docs.reopen_taken(vec![taken], Vec::new(), cx);
        })
    });
    settle(&cx, |app| docs.read(app).active_tab().is_some_and(|tab| tab.view.read(app).settled()));
    let name = cx.update(|app| docs.read(app).active_tab().expect("the copy").name.to_string());
    ely_platform::settings::Store::of(&kept).set("recovery", limits(1)).expect("the space is set");
    type_into(&mut cx, &docs, "more");
    round(&mut cx, &docs);
    settle(&cx, |app| refused(&docs, app) == 1 && writing(&docs, app) == 0);
    // The time between the message's two halves.
    let marked = cx.update(|app| ely_ui::i18n::text(app, "office.recovery.too_big", &[("name", &name), ("limit", "1"), ("at", "\u{1}")]));
    let (before, after) = marked.split_once('\u{1}').expect("the time's place");
    let toasts = cx.update(|app| root.read(app).toasts(app));
    let at = toasts.iter().find_map(|(text, _)| text.strip_prefix(before)?.strip_suffix(after)).expect("the too-big toast");
    let never = cx.update(|app| ely_ui::i18n::text(app, "office.stopped.never", &[]).to_string());
    assert_ne!(at, never, "the copy has a point: {toasts:?}");
    let back = cx.update(|app| ely_ui::i18n::text(app, "office.recovery.back", &[("name", "big.fodt"), ("at", at)]));
    assert!(toasts.iter().any(|(text, _)| *text == back), "the point it came back from: {toasts:?}");
}

#[test]
fn a_document_over_its_space_is_not_snapshotted_again() {
    for locale in ["en-US", "zh-CN"] {
        over_its_space(locale);
    }
}

fn over_its_space(locale: &'static str) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, kept)) = kept_docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(|app| ely_ui::i18n::install_as(locale, app));
    let root = super::words::rooted(&mut cx, &docs);
    let store = ely_platform::settings::Store::of(&kept);
    open_big(&mut cx, &docs, dir.path());
    type_into(&mut cx, &docs, "big");
    // Shrunk after the open check: the snapshot itself is refused.
    store.set("recovery", limits(1)).expect("the space is set");
    round(&mut cx, &docs);
    settle(&cx, |app| writing(&docs, app) == 1 || refused(&docs, app) == 1);
    assert_eq!(cx.update(|app| writing(&docs, app)), 1, "the snapshot is taken, then refused");
    settle(&cx, |app| refused(&docs, app) == 1 && writing(&docs, app) == 0);
    assert_eq!(points(&kept), 0, "over the space: no point");
    // Says why and how, never that the file is kept.
    let never = cx.update(|app| ely_ui::i18n::text(app, "office.stopped.never", &[]).to_string());
    let args = [("name", "big.fodt"), ("limit", "1"), ("at", never.as_str())];
    let said = cx.update(|app| ely_ui::i18n::text(app, "office.recovery.too_big", &args));
    let toasts = cx.update(|app| root.read(app).toasts(app));
    assert!(toasts.iter().any(|(text, _)| *text == said), "{locale}: {toasts:?}");
    // Each snapshot of it would hold the engine for nothing.
    round(&mut cx, &docs);
    assert_eq!(cx.update(|app| writing(&docs, app)), 0, "no snapshot while it stays too big");
    // No key since: the refused edit itself is kept.
    store.set("recovery", limits(64)).expect("the space is raised");
    cx.update(|app| docs.update(app, |docs, cx| docs.keep_points(cx)));
    settle(&cx, |app| refused(&docs, app) == 0);
    beat(&mut cx, &docs);
    beat(&mut cx, &docs);
    settle(&cx, |app| points(&kept) == 1 && writing(&docs, app) == 0);
}
