//! Sections put a PPTX in risk mode; the original stays.

use std::path::Path;

use ely_office::prelude::*;
use gpui::{AppContext as _, TestAppContext};

use super::{quiet, settle, update};

#[test]
fn sections_keep_the_original_and_save_only_as_a_copy() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = super::deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/show/sections.pptx");
    let path = dir.path().join("deck.pptx");
    std::fs::copy(&fixture, &path).expect("copies");
    update(&mut cx, &deck, |deck, cx| deck.open_file(path.clone(), cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.list.len() == 9));
    quiet(&cx, &deck);
    cx.update(|app| {
        let tab = deck.read(app).active_tab().expect("a tab");
        assert_eq!(tab.risk, Some(ely_office::Risk::Sections));
        assert_eq!(tab.state_key(app), "office.state.sections");
    });
    let (ask, answered) = ely_ui::Ask::for_tests(&serde_json::json!(["save", path.display().to_string()]).to_string());
    update(&mut cx, &deck, |deck, cx| deck.answer_request(ask, cx));
    let refused = answered.try_recv().expect("answered at once");
    assert!(refused.as_ref().is_err_and(|why| why.contains("another name")), "{refused:?}");
    let copy = dir.path().join("copy.pptx");
    let id = cx.update(|app| deck.read(app).core.active).expect("a tab");
    let window = cx.update(|app| deck.read(app).core.window).expect("a window");
    cx.update_window(window, |_, window, app| deck.update(app, |deck, cx| deck.write_as(id, copy.clone(), None, window, cx)))
        .expect("the window lives");
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.name == "copy.pptx"));
    quiet(&cx, &deck);
    assert!(ely_office::files::sections(&path).expect("the original"), "the original keeps its sections");
    assert!(!ely_office::files::sections(&copy).expect("the copy"), "the kernel drops sections");
    let risk = cx.update(|app| deck.read(app).active_tab().expect("a tab").risk);
    assert_eq!(risk, None, "the copy has none to lose");
}
