//! Countdowns over a running show.

use std::time::Duration;

use ely_office::prelude::*;
use gpui::TestAppContext;

use super::objects::blank;
use super::play::{three_slides, typed};
use super::{settle, update};
use crate::slideshow::timer::{Phase, Spot};

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
    // A replaced or removed countdown's clock ends; nothing redraws.
    let redraws = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = redraws.clone();
    let _watch = cx.update(|app| app.observe(&clocks, move |_, _| counted.set(counted.get() + 1)));
    let set = |cx: &mut TestAppContext, length: Option<u64>| {
        cx.update(|app| {
            deck.update(app, |deck, cx| {
                let tab = deck.core.active().expect("a tab");
                let name = deck.tab(tab).expect("the tab").extra.slides.list[2].name.clone();
                deck.set_timer(tab, Some(name.clone()), length.map(Duration::from_secs), (Spot::Corner, None), cx);
                deck.tab(tab).expect("the tab").extra.timers.get(&name).copied()
            })
        })
    };
    let edited = set(&mut cx, Some(30)).expect("a new countdown");
    redraws.set(0);
    cx.executor().advance_clock(Duration::from_secs(5));
    assert_eq!(redraws.get(), 0, "the edited one's old clock ended");
    cx.update(|app| clocks.update(app, |clocks, cx| clocks.toggle(edited, cx)));
    cx.executor().advance_clock(Duration::from_secs(2));
    assert!(redraws.get() >= 2, "a running one redraws each second");
    assert_eq!(set(&mut cx, None), None, "removed");
    redraws.set(0);
    cx.executor().advance_clock(Duration::from_secs(5));
    assert_eq!(redraws.get(), 0, "a removed running one stops redrawing");
}
