//! A stopped engine, and files that never reach it.

use std::path::Path;

use gpui::{Entity, IntoElement, Render, TestAppContext, VisualTestContext, Window};

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

#[test]
fn a_stopped_engine_shows_its_page_and_reopens() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    // A restart signs as the app's author, set at start.
    cx.update(|app| docs.read(app).core.worker.lock().expect("the worker").set_author("Ada").expect("signs"));
    let file = dir.path().join("open.fodt");
    let fodt = cx.update(|app| Template::Report.fodt(app));
    std::fs::write(&file, fodt).expect("the file writes");
    let window = cx.add_window(|_, _| Body(docs.clone()));
    let shown = VisualTestContext::from_window(window.into(), &cx).into_mut();
    let stop = cx.update(|app| docs.read(app).core.shared.get().stop.clone()).expect("the engine can be stopped");
    stop().expect("the engine stops");
    fail_a_load(&mut cx, &docs, &file);
    shown.update(|window, _| window.refresh());
    shown.run_until_parked();
    with_window(&mut cx, &docs, |docs, _, cx| docs.restart_engine(cx));
    settle(&cx, |app| docs.read(app).core.shared.get().stopped.is_none());
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(file.clone(), cx));
    settle(&cx, |app| docs.read(app).core.tabs.len() == 1);
}

/// A load on the stopped engine fails, reporting the end.
fn fail_a_load(cx: &mut TestAppContext, docs: &Entity<Docs>, file: &Path) {
    let copy = file.with_file_name(format!("again-{}.fodt", cx.update(|app| docs.read(app).core.next_id())));
    std::fs::copy(file, &copy).expect("a fresh file");
    with_window(cx, docs, |docs, _, cx| docs.open_file(copy, cx));
    settle(cx, |app| docs.read(app).core.shared.get().stopped.is_some());
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
    let (tabs, stopped) = cx.update(|app| (docs.read(app).core.tabs.len(), docs.read(app).core.shared.get().stopped.is_some()));
    assert_eq!((tabs, stopped), (0, false), "no tab, and the engine never saw it");
    let empty = dir.path().join("empty.docx");
    std::fs::write(&empty, "").expect("the file writes");
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(empty, cx));
    settle(&cx, |app| docs.read(app).core.loading.is_empty());
    assert_eq!(cx.update(|app| docs.read(app).core.tabs.len()), 1, "an empty file opens as new");
}

#[test]
fn a_broken_package_says_it_is_damaged_and_the_engine_stays() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let window = cx.add_window(|_, _| Body(docs.clone()));
    let shown = VisualTestContext::from_window(window.into(), &cx).into_mut();
    // The window's toasts live on its root.
    let root = shown.update(|window, app| {
        let root = ely_ui::document_root(ely_platform::AppId::Doc, docs.clone().into(), &ely_ui::commands::FILE, Vec::new(), window, app);
        docs.update(app, |docs, _| docs.core.root = root.downgrade());
        root
    });
    let broken = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office/broken.docx");
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(broken, cx));
    settle(&cx, |app| docs.read(app).core.loading.is_empty());
    let (tabs, stopped) = cx.update(|app| (docs.read(app).core.tabs.len(), docs.read(app).core.shared.get().stopped.is_some()));
    assert_eq!((tabs, stopped), (0, false), "no tab, and the engine lives");
    let said = cx.update(|app| ely_ui::i18n::text(app, "office.error.damaged", &[]));
    let toasts = cx.update(|app| root.read(app).toasts(app));
    assert!(toasts.iter().any(|(text, _)| text.contains(said.as_ref())), "{toasts:?}");
}
