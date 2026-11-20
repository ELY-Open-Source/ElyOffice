//! SHOW-02: the undo cap, one repaint; keys undo text.

use std::sync::Arc;

use gpui::VisualTestContext;

use super::focus::{Front, frames};
use super::link::{edited_text, kernel, no_toasts, root_for};
use super::*;

#[test]
fn past_the_undo_cap_an_edit_repaints_its_slide_alone() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| deck.new_blank(cx));
    settle(&cx, |app| counts(app, &deck) == (1, 1));
    update(&mut cx, &deck, |deck, cx| deck.run_active("slide.duplicate", &[], cx));
    settle(&cx, |app| counts(app, &deck) == (2, 2));
    // Hide and show until the kernel keeps 100 steps.
    update(&mut cx, &deck, |deck, cx| {
        for at in 0..110 {
            deck.run_active(["slide.hide", "slide.show"][at % 2], &[], cx);
        }
    });
    quiet(&cx, &deck);
    settle(&cx, |app| counts(app, &deck) == (2, 2));
    let read = |app: &gpui::App| {
        let tab = deck.read(app).active_tab().expect("a tab");
        let steps = tab.extra.history.as_ref().map_or(0, |(undo, _)| undo.len());
        let other = &tab.extra.slides.list[1 - tab.extra.slides.current].name;
        (tab.extra.reads, steps, tab.extra.thumbs[other].clone())
    };
    let (reads, steps, before) = cx.update(|app| read(app));
    assert_eq!(steps, 100, "the undo list is full");
    update(&mut cx, &deck, |deck, cx| deck.run_active("slide.hide", &[], cx));
    settle(&cx, |app| read(app).0 > reads);
    quiet(&cx, &deck);
    let (_, _, after) = cx.update(|app| read(app));
    assert!(Arc::ptr_eq(&before, &after), "the other slide keeps its picture");
}

/// Presses `keys` in the window, as typing does.
fn press(shown: &mut VisualTestContext, cx: &TestAppContext, deck: &Entity<Deck>, keys: &str) {
    shown.simulate_keystrokes(keys);
    frames(shown);
    quiet(cx, deck);
}

/// One slide whose box already says Plan.
const PLAN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
 xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:body><office:presentation>
 <draw:page draw:name="One"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="10cm" svg:height="3cm"><draw:text-box><text:p>Plan</text:p></draw:text-box></draw:frame></draw:page>
 </office:presentation></office:body>
</office:document>"#;

/// Clicks into the box, then types `typed` at its end.
fn type_at_end(cx: &mut TestAppContext, deck: &Entity<Deck>, typed: &str) {
    const END: i32 = 1029;
    kernel(cx, deck, |worker, doc| {
        for kind in [0, 1] {
            let click = ely_lok::Mouse { kind, x: 2500, y: 1500, count: 1, buttons: 1, modifiers: 0 };
            worker.post_mouse(doc, click).expect("clicks");
        }
    });
    settle(cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.view.read(app).caret()));
    kernel(cx, deck, |worker, doc| {
        worker.post_key(doc, 0, END).expect("End");
        for c in typed.chars() {
            worker.post_key(doc, c as u32, 0).expect("types");
        }
    });
    quiet(cx, deck);
}

#[test]
fn the_undo_keys_step_through_typing_inside_a_text_box() {
    const ESCAPE: i32 = 1281;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("plan.fodp");
    std::fs::write(&path, PLAN).expect("writes");
    update(&mut cx, &deck, |deck, cx| deck.open_with(path.clone(), String::new(), cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| !tab.extra.slides.list.is_empty()));
    let root = root_for(&mut cx, &deck);
    // As the app binds them; else Ctrl+Z goes raw.
    cx.update(ely_office::bind_undo_keys);
    let window = cx.add_window(|_, _| Front(deck.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    shown.update(|window, _| window.activate_window());
    shown.update(|window, app| deck.update(app, |deck, cx| {
        deck.own_window(window, cx);
        let focus = deck.active_tab().expect("a tab").view.read(cx).focus_handle().clone();
        window.focus(&focus, cx);
    }));
    frames(&mut shown);
    quiet(&cx, &deck);
    // An empty deck history first, then one step.
    for (typed, before) in [(" B", "Plan"), (" C", "Plan B")] {
        type_at_end(&mut cx, &deck, typed);
        let after = format!("{before}{typed}");
        assert_eq!(edited_text(&mut cx, &deck), after);
        press(&mut shown, &cx, &deck, "secondary-z");
        assert_eq!(edited_text(&mut cx, &deck), before, "Ctrl+Z took {typed:?} back");
        no_toasts(&mut cx, &root);
        press(&mut shown, &cx, &deck, "secondary-shift-z");
        assert_eq!(edited_text(&mut cx, &deck), after, "the redo key typed {typed:?} again");
        no_toasts(&mut cx, &root);
        kernel(&mut cx, &deck, |worker, doc| worker.post_key(doc, 0, ESCAPE).expect("leaves the text"));
        settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| !tab.view.read(app).caret()));
    }
    assert_eq!(undo_steps(&mut cx, &deck), 2, "each edit is one deck step");
}
