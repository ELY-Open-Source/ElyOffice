//! Objects follow the pointer and snap as dragged (R20.32, R20.36).

use ely_gpui_component::theme::Theme;
use ely_office::prelude::*;
use gpui::{Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext};

use super::menu::{Host, frames, spot};
use super::objects::{act, objects};
use super::{settle, undo_steps};
use crate::deck::Deck;
use crate::object::twips;

/// 1/100 mm in the view's twips.
fn tw(mm100: i64) -> i32 {
    twips(mm100) as i32
}

/// Object `index`'s frame, in 1/100 mm.
fn frame(cx: &mut VisualTestContext, deck: &Entity<Deck>, index: usize) -> (i64, i64, i64, i64) {
    cx.update(|_, app| objects(app, deck).expect("objects").objects[index].rect)
}

/// The view's carried frame and line count.
fn carried(cx: &mut VisualTestContext, deck: &Entity<Deck>) -> Option<((i32, i32, i32, i32), usize)> {
    cx.update(|_, app| deck.read(app).active_tab().expect("a tab").view.read(app).carried_now())
}

/// Drags from `from` by `by`; the frame follows.
fn drag(cx: &mut VisualTestContext, deck: &Entity<Deck>, from: (i64, i64), by: (i64, i64), keys: Modifiers) {
    let start = spot(cx, deck, from);
    cx.simulate_mouse_move(start, None, keys);
    cx.simulate_mouse_down(start, MouseButton::Left, keys);
    for step in 1..=4 {
        let at = spot(cx, deck, (from.0 + by.0 * step / 4, from.1 + by.1 * step / 4));
        cx.simulate_mouse_move(at, MouseButton::Left, keys);
        frames(cx);
        assert!(carried(cx, deck).is_some(), "step {step}: the frame follows the pointer");
    }
}

fn release(cx: &mut VisualTestContext, deck: &Entity<Deck>, at: (i64, i64), keys: Modifiers) {
    let at = spot(cx, deck, at);
    cx.simulate_mouse_up(at, MouseButton::Left, keys);
    frames(cx);
}

/// Two rectangles, A low left, B high right.
const PAIR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
 xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:body><office:presentation>
 <draw:page draw:name="One"><draw:rect svg:x="2cm" svg:y="10cm" svg:width="8cm" svg:height="4cm"/><draw:rect svg:x="16cm" svg:y="3cm" svg:width="8cm" svg:height="4cm"/></draw:page>
 </office:presentation></office:body>
</office:document>"#;

#[test]
fn a_dragged_object_follows_snaps_and_lands_where_dropped() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = super::deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(|cx| Theme::update(cx, |theme| theme.reduced_motion = true));
    let path = dir.path().join("pair.fodp");
    std::fs::write(&path, PAIR).expect("writes");
    super::update(&mut cx, &deck, |deck, cx| deck.open_file(path.clone(), cx));
    settle(&cx, |app| objects(app, &deck).is_some_and(|o| o.objects.len() == 2));
    act(&mut cx, &deck, ("object.select", &["1", "false"]), |o| o.objects[1].selected);
    let window = cx.add_window(|_, _| Host(deck.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    let (a, b) = (0, 1);
    // Alt: no snap; the selected B lands at the pointer.
    let before = frame(&mut shown, &deck, b);
    let centre = (before.0 + before.2 / 2, before.1 + before.3 / 2);
    let undo = undo_steps(&mut cx, &deck);
    let alt = Modifiers { alt: true, ..Modifiers::none() };
    drag(&mut shown, &deck, centre, (2_000, 1_500), alt);
    let (held, lines) = carried(&mut shown, &deck).expect("carried");
    assert_eq!(lines, 0, "Alt shows no lines");
    assert!((held.0 - tw(before.0 + 2_000)).abs() <= 3 && (held.1 - tw(before.1 + 1_500)).abs() <= 3, "the frame is under the pointer: {held:?}");
    assert_eq!(frame(&mut shown, &deck, b), before, "the kernel waits for the drop");
    release(&mut shown, &deck, (centre.0 + 2_000, centre.1 + 1_500), alt);
    settle(&cx, |app| objects(app, &deck).is_some_and(|o| o.objects[b].rect != before));
    let after = frame(&mut shown, &deck, b);
    assert!((after.0 - before.0 - 2_000).abs() <= 5 && (after.1 - before.1 - 1_500).abs() <= 5, "{before:?} -> {after:?}");
    assert_eq!(undo_steps(&mut cx, &deck), undo + 1, "one undo step");
    // Unselected A, dropped near B's left edge, aligns to it.
    let (moved_b, start) = (frame(&mut shown, &deck, b), frame(&mut shown, &deck, a));
    let grab = (start.0 + start.2 / 2, start.1 + start.3 / 2);
    let by = (moved_b.0 - start.0 + 60, 0);
    drag(&mut shown, &deck, grab, by, Modifiers::none());
    let (held, lines) = carried(&mut shown, &deck).expect("carried");
    assert!(lines > 0 && (held.0 - tw(moved_b.0)).abs() <= 2, "it snaps to B with a line: {held:?}, {lines}");
    release(&mut shown, &deck, (grab.0 + by.0, grab.1), Modifiers::none());
    settle(&cx, |app| objects(app, &deck).is_some_and(|o| o.objects[a].rect.0 != start.0));
    let landed = cx.update(|app| objects(app, &deck).expect("objects"));
    assert!((landed.objects[a].rect.0 - moved_b.0).abs() <= 3, "A's left is B's: {:?}", landed.objects[a].rect);
    assert!(landed.objects[a].selected, "the drop selects it");
    // A press that never moves clicks B.
    let b_now = landed.objects[b].rect;
    let on_b = (b_now.0 + b_now.2 / 2, b_now.1 + b_now.3 / 2);
    let at = spot(&mut shown, &deck, on_b);
    shown.simulate_mouse_move(at, None, Modifiers::none());
    shown.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    shown.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(&cx, |app| objects(app, &deck).is_some_and(|o| o.objects[b].selected && !o.objects[a].selected));
    assert_eq!(frame(&mut shown, &deck, b), b_now, "a click moves nothing");
    assert!(carried(&mut shown, &deck).is_none());
}

#[test]
fn a_drag_whose_read_went_stale_drops_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = super::deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("pair.fodp");
    std::fs::write(&path, PAIR).expect("writes");
    super::update(&mut cx, &deck, |deck, cx| deck.open_file(path.clone(), cx));
    settle(&cx, |app| objects(app, &deck).is_some_and(|o| o.objects.len() == 2));
    act(&mut cx, &deck, ("object.select", &["1", "false"]), |o| o.objects[1].selected);
    let window = cx.add_window(|_, _| Host(deck.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    let (a, b) = (frame(&mut shown, &deck, 0), frame(&mut shown, &deck, 1));
    let centre = (b.0 + b.2 / 2, b.1 + b.3 / 2);
    let alt = Modifiers { alt: true, ..Modifiers::none() };
    drag(&mut shown, &deck, centre, (-9_000, 4_000), alt);
    // Another window picks A while B is carried.
    cx.update(|app| {
        let deck = deck.read(app);
        let doc = deck.active_tab().expect("a tab").doc.clone();
        let mut worker = deck.core.worker.lock().expect("the worker");
        let state = worker.now(&doc).expect("the state");
        worker.run(&state, "object.select", &["0", "false"]).expect("selects A");
    });
    release(&mut shown, &deck, (centre.0 - 9_000, centre.1 + 4_000), alt);
    super::quiet(&cx, &deck);
    update_objects(&mut cx, &deck);
    let o = cx.update(|app| objects(app, &deck).expect("objects"));
    assert_eq!((o.objects[0].rect, o.objects[1].rect), (a, b), "a stale drop moves nothing");
}

/// Rereads the slide's objects.
fn update_objects(cx: &mut TestAppContext, deck: &Entity<Deck>) {
    let reads = cx.update(|app| deck.read(app).active_tab().expect("a tab").extra.reads);
    super::update(cx, deck, |deck, cx| {
        let tab = deck.core.active().expect("a tab");
        deck.force_reread(tab, cx)
    });
    settle(cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.reads > reads));
}
