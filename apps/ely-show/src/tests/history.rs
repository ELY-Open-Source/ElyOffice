//! SHOW-02: past the undo cap, one edit, one repaint.

use std::sync::Arc;

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
