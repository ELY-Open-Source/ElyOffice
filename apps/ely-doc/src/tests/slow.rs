//! A load that hangs can be stopped (GitHub #49).

use std::{path::Path, time::Duration};

use gpui::{Entity, IntoElement, Modifiers, Render, TestAppContext, VisualTestContext, Window};

use ely_office::prelude::*;

use super::{docs, settle, with_window};
use crate::docs::Docs;
use crate::files::Template;

/// Draws the window body ElyDoc shows.
struct Body(Entity<Docs>);

impl Render for Body {
    fn render(&mut self, _: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        self.0.update(cx, |docs, cx| docs.body(cx))
    }
}

const STOP: &str = "banner-action Stop the engine";
const REOPEN: &str = "banner-action Reopen documents";

/// Redraws the body; whether the banner action `selector` shows.
fn drawn(shown: &mut VisualTestContext, selector: &'static str) -> bool {
    shown.update(|window, _| window.refresh());
    shown.run_until_parked();
    shown.debug_bounds(selector).is_some()
}

/// Presses the banner action `selector` in the drawn body.
fn press(shown: &mut VisualTestContext, selector: &'static str) {
    assert!(drawn(shown, selector), "{selector} draws");
    let at = shown.debug_bounds(selector).expect("drawn above");
    shown.simulate_click(at.center(), Modifiers::none());
    shown.run_until_parked();
}

#[test]
fn a_slow_load_offers_the_stop_and_the_stop_ends_the_engine() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    assert!(cx.update(|app| docs.read(app).core.stop.is_some()), "the engine can be stopped");
    // A restart signs as the app's author, set at start.
    cx.update(|app| docs.read(app).core.worker.lock().expect("the worker").set_author("Ada").expect("signs"));
    let file = dir.path().join("open.fodt");
    let fodt = cx.update(|app| Template::Report.fodt(app));
    std::fs::write(&file, fodt).expect("the file writes");
    let window = cx.add_window(|_, _| Body(docs.clone()));
    let shown = VisualTestContext::from_window(window.into(), &cx).into_mut();
    // The first document hangs: no tab, only the loading page.
    with_window(&mut cx, &docs, |docs, _, cx| {
        docs.core.loading.push((7, "stuck.docx".into()));
        docs.watch_slow(7, cx);
    });
    assert!(!drawn(shown, STOP), "not slow yet");
    cx.executor().advance_clock(Duration::from_secs(31));
    cx.run_until_parked();
    assert_eq!(cx.update(|app| docs.read(app).core.slow), Some(7), "the timer marks it slow");
    press(shown, STOP);
    assert_eq!(cx.update(|app| docs.read(app).core.slow), None, "the offer is gone");
    // The press ends the engine.
    settle(&cx, |app| docs.read(app).core.worker.lock().expect("the worker").ended().is_some());
    with_window(&mut cx, &docs, |docs, _, _| docs.core.loading.clear());
    fail_a_load(&mut cx, &docs, &file);
    press(shown, REOPEN);
    settle(&cx, |app| docs.read(app).core.stopped.is_none());
    // A load that answers in time never offers it.
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(file.clone(), cx));
    settle(&cx, |app| docs.read(app).core.tabs.len() == 1);
    cx.executor().advance_clock(Duration::from_secs(31));
    cx.run_until_parked();
    assert_eq!(cx.update(|app| docs.read(app).core.slow), None, "an answered load is not slow");
    // The restarted engine can be stopped too.
    with_window(&mut cx, &docs, |docs, _, cx| docs.stop_engine(cx));
    fail_a_load(&mut cx, &docs, &file);
}

/// A load on the stopped engine fails, reporting the end.
fn fail_a_load(cx: &mut TestAppContext, docs: &Entity<Docs>, file: &Path) {
    let copy = file.with_file_name(format!("again-{}.fodt", cx.update(|app| docs.read(app).core.next)));
    std::fs::copy(file, &copy).expect("a fresh file");
    with_window(cx, docs, |docs, _, cx| docs.open_file(copy, cx));
    settle(cx, |app| docs.read(app).core.stopped.is_some());
}

#[test]
fn a_docx_that_is_plain_text_fails_at_once_and_the_engine_stays() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let fake = dir.path().join("fake.docx");
    std::fs::write(&fake, "examplefile.com - Sample Files\r\n".repeat(1000)).expect("the file writes");
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(fake, cx));
    settle(&cx, |app| docs.read(app).core.loading.is_empty());
    let (tabs, stopped) = cx.update(|app| (docs.read(app).core.tabs.len(), docs.read(app).core.stopped.is_some()));
    assert_eq!((tabs, stopped), (0, false), "no tab, and the engine never saw it");
    let empty = dir.path().join("empty.docx");
    std::fs::write(&empty, "").expect("the file writes");
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(empty, cx));
    settle(&cx, |app| docs.read(app).core.loading.is_empty());
    assert_eq!(cx.update(|app| docs.read(app).core.tabs.len()), 1, "an empty file opens as new");
}
