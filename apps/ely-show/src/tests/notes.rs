//! The notes field under the slide (SHOW-08).

use ely_office::prelude::*;
use gpui::TestAppContext;

use super::objects::blank;
use super::slides::{in_window, type_in};
use super::{settle, update};

#[test]
fn typed_notes_reach_the_slide() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let notes = |app: &gpui::App| {
        deck.read(app)
            .active_tab()
            .and_then(|tab| tab.extra.notes.clone())
    };
    settle(&cx, |app| notes(app).is_some());
    let field = in_window(&mut cx, &deck, |deck, window, cx| {
        deck.notes_input(window, cx)
    });
    let field = field.expect("the field");
    type_in(&mut cx, &field, "Start with the totals");
    update(&mut cx, &deck, |deck, cx| deck.commit_notes(cx));
    settle(&cx, |app| {
        notes(app).is_some_and(|notes| notes.text == "Start with the totals")
    });
}

#[test]
fn notes_refused_as_stale_stay_typed_and_go_next_time() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let notes = |app: &gpui::App| deck.read(app).active_tab().and_then(|tab| tab.extra.notes.clone());
    let text = |app: &gpui::App| notes(app).map(|notes| notes.text);
    settle(&cx, |app| notes(app).is_some());
    let field = in_window(&mut cx, &deck, |deck, window, cx| deck.notes_input(window, cx)).expect("the field");
    let stale = cx.update(|app| deck.read(app).notes.at.clone());
    type_in(&mut cx, &field, "first");
    update(&mut cx, &deck, |deck, cx| deck.commit_notes(cx));
    settle(&cx, |app| text(app).as_deref() == Some("first"));
    in_window(&mut cx, &deck, |deck, window, cx| deck.notes_input(window, cx));
    // Typing began before the edit above was read back.
    type_in(&mut cx, &field, "second");
    update(&mut cx, &deck, |deck, cx| {
        deck.notes.at = stale;
        deck.commit_notes(cx);
    });
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.view.read(app).settled()));
    cx.run_until_parked();
    in_window(&mut cx, &deck, |deck, window, cx| deck.notes_input(window, cx));
    assert_eq!(cx.update(|app| field.read(app).text().to_string()), "second", "the typing stays");
    assert_eq!(cx.update(|app| text(app)).as_deref(), Some("first"), "the stale send was refused");
    update(&mut cx, &deck, |deck, cx| deck.commit_notes(cx));
    settle(&cx, |app| text(app).as_deref() == Some("second"));
}

#[test]
fn a_read_while_typing_keeps_the_new_draft() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let text = |app: &gpui::App| deck.read(app).active_tab().and_then(|tab| tab.extra.notes.clone()).map(|notes| notes.text);
    settle(&cx, |app| text(app).is_some());
    let field = in_window(&mut cx, &deck, |deck, window, cx| deck.notes_input(window, cx)).expect("the field");
    type_in(&mut cx, &field, "first");
    update(&mut cx, &deck, |deck, cx| deck.commit_notes(cx));
    // Back in the field before the send is read back.
    in_window(&mut cx, &deck, |_, window, cx| window.focus(&gpui::Focusable::focus_handle(field.read(cx), cx), cx));
    type_in(&mut cx, &field, "second");
    settle(&cx, |app| text(app).as_deref() == Some("first"));
    in_window(&mut cx, &deck, |deck, window, cx| deck.notes_input(window, cx));
    // The blur redraws before its commit runs.
    in_window(&mut cx, &deck, |deck, window, cx| {
        window.blur(cx);
        deck.notes_input(window, cx)
    });
    assert_eq!(cx.update(|app| field.read(app).text().to_string()), "second", "the draft stays");
    update(&mut cx, &deck, |deck, cx| deck.commit_notes(cx));
    settle(&cx, |app| text(app).as_deref() == Some("second"));
}

#[test]
fn comments_open_a_folded_right_panel() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(|app| ely_ui::settings::panels::show(false, false, app));
    cx.update(|app| deck.update(app, |deck, cx| deck.toggle_remarks(cx)));
    let (remarks, shown) = cx.update(|app| (deck.read(app).remarks_open, ely_ui::settings::panels::shown(false, app)));
    assert!(remarks && shown, "View › Comments shows them");
}
