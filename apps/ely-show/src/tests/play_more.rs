//! Show starts, copies, windows and timing (C10.5 R1).

use ely_office::prelude::*;
use gpui::{Entity, TestAppContext};

use super::objects::{act, blank};
use super::play::{place, three_slides, typed};
use super::{settle, update};
use crate::deck::Deck;
use crate::preview::timeline::{Look, object_look};
use crate::slideshow::{Show, start::show_window, steps::Place};

pub(super) fn started(cx: &mut TestAppContext, deck: &Entity<Deck>, here: bool) -> Entity<Show> {
    update(cx, deck, |deck, cx| deck.start_show(here, cx));
    settle(cx, |app| deck.read(app).show.is_some());
    cx.update(|app| deck.read(app).show.clone().expect("playing"))
}

fn bytes(cx: &mut TestAppContext, show: &Entity<Show>, part: usize) -> Vec<u8> {
    cx.update(|app| {
        let stack = show.read(app).stack(part).expect("read").clone();
        stack.iter().flat_map(|(_, image)| image.as_bytes(0).expect("a frame").to_vec()).collect()
    })
}

#[test]
fn both_windows_take_keys_and_a_second_start_waits() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    update(&mut cx, &deck, |deck, cx| {
        deck.start_show(false, cx);
        deck.start_show(false, cx);
    });
    settle(&cx, |app| deck.read(app).show.is_some());
    let show = cx.update(|app| deck.read(app).show.clone().expect("playing"));
    let before = cx.update(|app| app.windows().len());
    for _ in 0..50 {
        cx.run_until_parked();
    }
    assert_eq!(cx.update(|app| app.windows().len()), before, "one start, one window");
    // A presenter window beside the audience, as on two screens.
    let presenter = cx.update(|app| {
        let display = app.primary_display().expect("a screen");
        show_window(&show, &display, true, app).expect("opens")
    });
    cx.update(|app| show.update(app, |show, _| show.windows.push(presenter)));
    cx.simulate_keystrokes(presenter, "right");
    assert_eq!(place(&mut cx, &show), Place::Slide { part: 1, clicks: 0 });
    typed(&mut cx, &show, "right");
    assert_eq!(place(&mut cx, &show), Place::Slide { part: 1, clicks: 1 });
}

#[test]
fn from_here_follows_a_slide_picked_just_now() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    update(&mut cx, &deck, |deck, cx| {
        let view = deck.active_tab().expect("a tab").view.clone();
        view.update(cx, |view, _| view.show_part(1));
        deck.start_show(true, cx);
    });
    settle(&cx, |app| deck.read(app).show.is_some());
    let show = cx.update(|app| deck.read(app).show.clone().expect("playing"));
    assert_eq!(place(&mut cx, &show), Place::Slide { part: 1, clicks: 0 });
}

#[test]
fn edits_during_a_show_stay_out_of_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    let show = started(&mut cx, &deck, true);
    settle(&cx, |app| show.read(app).stack(2).is_some());
    let first = bytes(&mut cx, &show, 2);
    act(&mut cx, &deck, ("draw.shape", &["ellipse", "true"]), |o| !o.objects.is_empty());
    typed(&mut cx, &show, "1 enter");
    settle(&cx, |app| show.read(app).stack(2).is_none());
    typed(&mut cx, &show, "3 enter");
    settle(&cx, |app| show.read(app).stack(2).is_some());
    assert_eq!(bytes(&mut cx, &show, 2), first, "the show plays its copy");
}

#[test]
fn late_layers_and_jumps_leave_no_trace() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    let fade = r#"{"preset":"fade-smoothly","duration":10,"advance":"click","time":0,"all":false}"#;
    update(&mut cx, &deck, |deck, cx| {
        let view = deck.active_tab().expect("a tab").view.clone();
        view.update(cx, |view, _| view.show_part(1));
        deck.design_command("slide.transition", &[fade], cx);
    });
    settle(&cx, |app| {
        let tab = deck.read(app).active_tab();
        tab.and_then(|t| t.extra.motion.as_ref()).is_some_and(|(t, _)| t.duration == 10.0)
    });
    let show = started(&mut cx, &deck, false);
    settle(&cx, |app| {
        let show = show.read(app);
        show.stack(0).is_some() && show.stack(1).is_some()
    });
    typed(&mut cx, &show, "right");
    assert!(cx.update(|app| show.read(app).leaving.is_some()), "it fades in");
    typed(&mut cx, &show, "2 enter");
    assert!(cx.update(|app| show.read(app).leaving.is_none()), "a jump has no transition");
    typed(&mut cx, &show, "3 enter 1 enter");
    settle(&cx, |app| {
        let show = show.read(app);
        show.stack(0).is_some() && show.stack(1).is_some()
    });
    for _ in 0..100 {
        cx.run_until_parked();
    }
    assert!(cx.update(|app| show.read(app).stack(2).is_none()), "slide three came late");
}

#[test]
fn a_finished_exit_stays_gone_as_the_slide_leaves() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    let cover = r#"{"preset":"cover-left","duration":5,"advance":"click","time":0,"all":true}"#;
    update(&mut cx, &deck, |deck, cx| deck.design_command("slide.transition", &[cover], cx));
    update(&mut cx, &deck, |deck, cx| {
        let view = deck.active_tab().expect("a tab").view.clone();
        view.update(cx, |view, _| view.show_part(0));
    });
    settle(&cx, |app| {
        let tab = deck.read(app).active_tab();
        tab.is_some_and(|t| t.extra.slides.current == 0 && t.extra.objects.is_some())
    });
    act(&mut cx, &deck, ("draw.shape", &["rectangle", "true"]), |o| o.objects.len() == 3);
    act(&mut cx, &deck, ("object.select", &["2", "false"]), |o| {
        o.selection.is_some()
    });
    let exit = serde_json::json!({ "preset": "ooo-exit-disappear" }).to_string();
    update(&mut cx, &deck, |deck, cx| deck.design_command("animation.add", &[&exit], cx));
    settle(&cx, |app| {
        let tab = deck.read(app).active_tab();
        tab.and_then(|t| t.extra.motion.as_ref()).is_some_and(|(_, e)| e.len() == 1)
    });
    let show = started(&mut cx, &deck, false);
    settle(&cx, |app| show.read(app).stack(0).is_some());
    typed(&mut cx, &show, "right");
    std::thread::sleep(std::time::Duration::from_millis(300));
    typed(&mut cx, &show, "right");
    let (look, hash) = cx.update(|app| {
        let show = show.read(app);
        let hash = show.slides[0].effects[0].hash.clone();
        let leaving = show.leaving.as_ref().expect("covered");
        (object_look(&leaving.runs, &hash, true, leaving.at, 0.5), hash)
    });
    assert!(!hash.is_empty());
    assert_eq!(look, Look::HIDDEN, "the exit holds through the transition");
}

#[test]
fn media_that_cannot_play_says_why_and_the_show_goes_on() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let clip = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/media/clip.mp4");
    let url = ely_lok::file_url(&clip);
    update(&mut cx, &deck, |deck, cx| {
        deck.design_command("media.insert", &[&url, "false", "8000", "4500"], cx)
    });
    settle(&cx, |app| {
        deck.read(app).active_tab().is_some_and(|tab| tab.extra.media.len() == 1)
    });
    let show = started(&mut cx, &deck, true);
    // The show's own media file, gone, fails on every platform.
    let file = cx.update(|app| show.read(app).slides[0].media[0].file.clone());
    std::fs::remove_file(ely_lok::file_path(&file).expect("a file URL")).expect("removes the media");
    cx.update(|app| show.update(app, |show, cx| show.toggle_media(0, cx)));
    assert!(cx.update(|app| show.read(app).notice.is_some()));
    assert!(cx.update(|app| deck.read(app).show.is_some()), "the show goes on");
    // A later failure says why the same way.
    cx.update(|app| show.update(app, |show, _| show.notice = None));
    let seen = cx.update(|app| show.update(app, |show, cx| show.media_seen(Ok(true), cx)));
    assert!(seen && cx.update(|app| show.read(app).notice.is_none()));
    let failed = Err("Cannot Open".to_string());
    let seen = cx.update(|app| show.update(app, |show, cx| show.media_seen(failed, cx)));
    let notice = cx.update(|app| show.read(app).notice.clone());
    assert!(!seen && notice.is_some_and(|n| n.contains("Cannot Open")));
    assert!(cx.update(|app| deck.read(app).show.is_some()), "the show goes on");
}
