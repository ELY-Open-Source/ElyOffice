//! Countdowns over a running show.

use std::time::Duration;

use ely_office::prelude::*;
use gpui::TestAppContext;

use super::objects::blank;
use super::play::{three_slides, typed};
use super::{settle, update};
use crate::slideshow::{scoreboard::Source, timer::{Phase, Spot}};

#[test]
fn all_slides_share_a_countdown_and_one_slide_may_have_its_own() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    update(&mut cx, &deck, |deck, cx| {
        let tab = deck.core.active().expect("a tab");
        let last = deck.tab(tab).expect("the tab").extra.slides.list[2].name.clone();
        deck.set_timer(tab, None, Some(Duration::from_secs(60)), (Spot::Top, None), cx);
        deck.set_timer(tab, Some(last), Some(Duration::from_secs(5)), (Spot::Corner, None), cx);
        deck.start_show(false, cx);
    });
    settle(&cx, |app| deck.read(app).show.is_some());
    let show = cx.update(|app| deck.read(app).show.clone().expect("playing"));
    cx.update(|app| show.update(app, |show, cx| show.jump(1, cx)));
    let reads = |cx: &TestAppContext, part: usize| {
        cx.update(|app| {
            let face = show.read(app).countdown(part, app).expect("a countdown");
            (face.text, face.phase)
        })
    };
    assert_eq!(reads(&cx, 0), ("1:00".into(), Phase::Ready), "waits to be started");
    typed(&mut cx, &show, "t");
    cx.executor().advance_clock(Duration::from_secs(10));
    assert_eq!(reads(&cx, 0), ("0:50".into(), Phase::Running));
    assert_eq!(reads(&cx, 1), reads(&cx, 0), "one clock for both");
    assert_eq!(reads(&cx, 2), ("0:05".into(), Phase::Ready), "its own clock");
    typed(&mut cx, &show, "t");
    cx.executor().advance_clock(Duration::from_secs(10));
    assert_eq!(reads(&cx, 0), ("0:50".into(), Phase::Paused));
    typed(&mut cx, &show, "r");
    assert_eq!(reads(&cx, 0), ("1:00".into(), Phase::Ready));
    cx.update(|app| show.update(app, |show, cx| show.jump(3, cx)));
    typed(&mut cx, &show, "t");
    cx.executor().advance_clock(Duration::from_secs(6));
    assert_eq!(reads(&cx, 2), ("0:00".into(), Phase::Over));
    typed(&mut cx, &show, "t");
    assert_eq!(reads(&cx, 2), ("0:05".into(), Phase::Running), "a done one starts over");
    typed(&mut cx, &show, "escape");
    settle(&cx, |app| deck.read(app).show.is_none());
    // The editor shares the clock; it runs on.
    let (timer, clocks) = cx.update(|app| {
        let deck = deck.read(app);
        let extra = &deck.tab(deck.core.active().expect("a tab")).expect("the tab").extra;
        let name = &extra.slides.list[2].name;
        (extra.timers[name], extra.clocks.clone().expect("clocks"))
    });
    let shown = |cx: &TestAppContext| {
        cx.update(|app| clocks.read(app).face(&timer, app.background_executor().now()))
    };
    assert_eq!(shown(&cx).phase, Phase::Running);
    cx.executor().advance_clock(Duration::from_secs(2));
    assert_eq!(shown(&cx).text, "0:03");
    update(&mut cx, &deck, |deck, cx| {
        let tab = deck.core.active().expect("a tab");
        let name = deck.tab(tab).expect("the tab").extra.slides.list[2].name.clone();
        deck.drag_timer(tab, &name, (0.25, 0.75), cx);
    });
    let moved = cx.update(|app| {
        let deck = deck.read(app);
        let extra = &deck.tab(deck.core.active().expect("a tab")).expect("the tab").extra;
        extra.timers[&extra.slides.list[2].name]
    });
    assert_eq!((moved.id, moved.at), (timer.id, Some((0.25, 0.75))));
    assert_eq!(shown(&cx).text, "0:03", "moving keeps the clock");
}

/// A stand-in ElyTable with one score.
fn table(_: &[&str]) -> std::io::Result<String> {
    Ok(r#"[["Lions","3"]]"#.into())
}

#[test]
fn the_focus_swaps_in_the_editor_for_good_and_in_a_show_for_itself() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| !tab.extra.slides.list.is_empty()));
    let kept = |cx: &TestAppContext| {
        cx.update(|app| {
            let tab = deck.read(app).active_tab().expect("a tab");
            tab.extra.timers[&tab.extra.slides.list[0].name].ahead
        })
    };
    update(&mut cx, &deck, |deck, cx| {
        let tab = deck.core.active().expect("a tab");
        let name = deck.tab(tab).expect("the tab").extra.slides.list[0].name.clone();
        let minute = Some(Duration::from_secs(60));
        deck.set_timer(tab, Some(name.clone()), minute, (Spot::Top, None), cx);
        deck.swap_focus(tab, &name, cx);
        // Editing the countdown keeps its focus.
        deck.set_timer(tab, Some(name), minute, (Spot::Corner, None), cx);
        deck.start_show(false, cx);
    });
    assert!(kept(&cx));
    settle(&cx, |app| deck.read(app).show.is_some());
    let show = cx.update(|app| deck.read(app).show.clone().expect("playing"));
    let shown = |cx: &TestAppContext| cx.update(|app| show.read(app).countdown(0, app).expect("a countdown").ahead);
    typed(&mut cx, &show, "f");
    assert!(shown(&cx), "alone, there is nothing to swap with");
    let score = Source { book: "7:0".into(), path: None, name: "Game".into(), sheet: "A".into() };
    cx.update(|app| show.update(app, |show, cx| show.follow_scores(vec![Some(score)], table, cx)));
    typed(&mut cx, &show, "f");
    assert!(!shown(&cx), "the scores take the focus");
    typed(&mut cx, &show, "f");
    assert!(shown(&cx));
    typed(&mut cx, &show, "f");
    typed(&mut cx, &show, "escape");
    settle(&cx, |app| deck.read(app).show.is_none());
    assert!(kept(&cx), "the show's swaps end with it");
}
