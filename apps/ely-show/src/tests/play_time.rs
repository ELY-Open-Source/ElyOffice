//! The show's clock: back, late layers, released slides (C10.5 R2).

use std::time::Duration;

use ely_office::prelude::*;
use gpui::{Entity, TestAppContext};

use super::objects::{act, blank};
use super::play::{place, three_slides, typed};
use super::{settle, update};
use crate::deck::Deck;
use crate::preview::timeline::{Look, object_look};
use crate::slideshow::{Show, steps};

fn started(cx: &mut TestAppContext, deck: &Entity<Deck>) -> Entity<Show> {
    update(cx, deck, |deck, cx| deck.start_show(false, cx));
    settle(cx, |app| deck.read(app).show.is_some());
    let show = cx.update(|app| deck.read(app).show.clone().expect("playing"));
    settle(cx, |app| {
        let show = show.read(app);
        show.stack(0).is_some() && show.stack(1).is_some()
    });
    show
}

/// Slide one: a rectangle fading in for ten seconds.
fn fading_first(cx: &mut TestAppContext, deck: &Entity<Deck>) {
    update(cx, deck, |deck, cx| {
        let view = deck.active_tab().expect("a tab").view.clone();
        view.update(cx, |view, _| view.show_part(0));
    });
    settle(cx, |app| {
        let tab = deck.read(app).active_tab();
        tab.is_some_and(|t| t.extra.slides.current == 0 && t.extra.objects.is_some())
    });
    act(cx, deck, ("draw.shape", &["rectangle", "true"]), |o| o.objects.len() == 3);
    act(cx, deck, ("object.select", &["2", "false"]), |o| o.selection.is_some());
    let add = serde_json::json!({ "preset": "ooo-entrance-fade-in" }).to_string();
    update(cx, deck, |deck, cx| deck.design_command("animation.add", &[&add], cx));
    let effects = |app: &gpui::App| {
        let tab = deck.read(app).active_tab();
        tab.and_then(|t| t.extra.motion.clone()).map(|(_, e)| e).unwrap_or_default()
    };
    settle(cx, |app| effects(app).len() == 1);
    update(cx, deck, |deck, cx| {
        deck.send_effect(0, serde_json::json!({ "start": "with", "duration": 10.0 }), cx)
    });
    settle(cx, |app| effects(app).first().is_some_and(|e| e.start == "with" && e.duration == 10.0));
}

#[test]
fn back_shows_a_slide_fully_built_even_without_clicks() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    fading_first(&mut cx, &deck);
    let show = started(&mut cx, &deck);
    typed(&mut cx, &show, "right left");
    assert_eq!(place(&mut cx, &show), steps::Place::Slide { part: 0, clicks: 0 });
    let (look, moving) = cx.update(|app| {
        let show = show.read(app);
        let effect = &show.slides[0].effects[0];
        let runs = steps::runs(&show.slides[0].effects, &show.clicks);
        let t = show.entered.elapsed().as_secs_f64();
        (object_look(&runs, &effect.hash, false, t, 0.5), show.moving())
    });
    assert_eq!((look, moving), (Look::SHOWN, false), "nothing replays");
}

#[test]
fn a_slide_starts_its_clock_when_its_layers_come() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    let show = started(&mut cx, &deck);
    // Another tab's read holds the worker a while.
    let worker = cx.update(|app| deck.read(app).core.worker.clone());
    let (held, hold) = std::sync::mpsc::channel();
    let busy = std::thread::spawn(move || {
        let _lock = worker.lock().expect("the worker lock");
        held.send(()).expect("told");
        // The other read holds the engine this long.
        std::thread::sleep(Duration::from_millis(400));
    });
    hold.recv().expect("held");
    // The read for slide three waits on the lock.
    typed(&mut cx, &show, "3 enter");
    settle(&cx, |app| show.read(app).stack(2).is_some());
    busy.join().expect("let go");
    let (waiting, since) = cx.update(|app| {
        let show = show.read(app);
        (show.waiting, show.entered.elapsed())
    });
    assert!(!waiting);
    assert!(since < Duration::from_millis(300), "the clock began late: {since:?}");
}

#[test]
fn a_finished_transition_lets_the_left_slide_go() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    let fade = r#"{"preset":"fade-smoothly","duration":0.1,"advance":"click","time":0,"all":true}"#;
    update(&mut cx, &deck, |deck, cx| deck.design_command("slide.transition", &[fade], cx));
    settle(&cx, |app| {
        let tab = deck.read(app).active_tab();
        tab.and_then(|t| t.extra.motion.as_ref()).is_some_and(|(t, _)| t.duration == 0.1)
    });
    let show = started(&mut cx, &deck);
    typed(&mut cx, &show, "right");
    assert!(cx.update(|app| show.read(app).leaving.is_some()));
    settle(&cx, |app| show.read(app).leaving.is_none());
    let held = cx.update(|app| {
        let show = show.read(app);
        (0..3).filter(|at| show.stack(*at).is_some()).count()
    });
    assert!(held <= 2, "only the shown and next slides stay");
}
