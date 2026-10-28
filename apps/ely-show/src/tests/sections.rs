//! Sections put a PPTX in risk mode; the original stays.

use std::path::Path;

use ely_office::{Risk, prelude::*};
use gpui::{AppContext as _, TestAppContext};

use super::{quiet, settle, update};

/// Opens `fixture`, expects `risk`, then saves only a copy.
fn keeps_the_original(fixture: &str, password: &str, risk: Risk) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = super::deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/show").join(fixture);
    let path = dir.path().join("deck.pptx");
    std::fs::copy(&source, &path).expect("copies");
    let original = std::fs::read(&path).expect("reads");
    let password = password.to_string();
    update(&mut cx, &deck, |deck, cx| deck.open_with(path.clone(), password, cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.list.len() == 9));
    quiet(&cx, &deck);
    assert_eq!(cx.update(|app| deck.read(app).active_tab().expect("a tab").risk), Some(risk), "{fixture}");
    let (ask, answered) = ely_ui::Ask::for_tests(&serde_json::json!(["save", path.display().to_string()]).to_string());
    update(&mut cx, &deck, |deck, cx| deck.answer_request(ask, cx));
    let refused = answered.try_recv().expect("answered at once");
    assert!(refused.as_ref().is_err_and(|why| why.contains("another name")), "{fixture}: {refused:?}");
    let copy = dir.path().join("copy.pptx");
    let id = cx.update(|app| deck.read(app).core.active).expect("a tab");
    let window = cx.update(|app| deck.read(app).core.window).expect("a window");
    cx.update_window(window, |_, window, app| deck.update(app, |deck, cx| deck.write_as(id, copy.clone(), None, window, cx)))
        .expect("the window lives");
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.name == "copy.pptx"));
    quiet(&cx, &deck);
    assert_eq!(std::fs::read(&path).expect("reads"), original, "{fixture}: the original stays");
    assert!(!ely_office::files::sections(&copy).expect("the copy"), "{fixture}: the kernel drops sections");
    assert_eq!(cx.update(|app| deck.read(app).active_tab().expect("a tab").risk), None, "{fixture}: the copy is safe");
}

#[test]
fn sections_keep_the_original_and_save_only_as_a_copy() {
    keeps_the_original("sections.pptx", "", Risk::Sections);
}

#[test]
fn utf16_and_encrypted_decks_are_guarded_too() {
    keeps_the_original("sections-utf16.pptx", "", Risk::Sections);
    keeps_the_original("sections-locked.pptx", "ely", Risk::Sealed);
    keeps_the_original("sections-locked-tail.pptx", "ely", Risk::Sealed);
}
