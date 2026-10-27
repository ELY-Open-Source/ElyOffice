//! Video pictures reach the show and wait to be freed.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use ely_office::prelude::*;
use gpui::{Context, TestAppContext};

use super::objects::blank;
use super::play_more::started;
use super::{settle, update};
use crate::slideshow::Show;

/// Plays the clip; `stop` ends it after one replaced picture.
fn stopped_by(another_slide: bool, stop: impl FnOnce(&mut Show, &mut Context<Show>)) {
    if !ely_platform::media::BUILT {
        eprintln!("SKIP: no system player is built on this platform");
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let clip = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/media/clip.mp4");
    let url = ely_lok::file_url(&clip);
    update(&mut cx, &deck, |deck, cx| deck.design_command("media.insert", &[&url, "false", "8000", "4500"], cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.media.len() == 1));
    if another_slide {
        update(&mut cx, &deck, |deck, cx| deck.design_command("slide.insert", &[], cx));
        settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.list.len() == 2));
    }
    let show = started(&mut cx, &deck, false);
    cx.update(|app| show.update(app, |show, cx| show.toggle_media(0, cx)));
    let end = Instant::now() + Duration::from_secs(10);
    let mut first = None;
    // Counted in the tick that replaces it, before a paint.
    while !cx.update(|app| {
        show.update(app, |show, cx| {
            show.media_tick(cx);
            let Some((now, _)) = show.media_frame() else {
                return false;
            };
            let first = first.get_or_insert_with(|| now.clone());
            if Arc::ptr_eq(first, &now) {
                return false;
            }
            assert_eq!(first.size(0), gpui::size(160.into(), 90.into()), "the clip's own size");
            assert_eq!(Arc::strong_count(first), 2, "the replaced picture waits in retired");
            true
        })
    }) {
        assert!(Instant::now() < end, "the video never moved on");
        std::thread::sleep(Duration::from_millis(20));
    }
    cx.update(|app| {
        show.update(app, |show, cx| {
            let last = show.media_frame().expect("a picture").0;
            stop(show, cx);
            assert!(show.media_frame().is_none(), "playback stopped");
            assert_eq!(Arc::strong_count(&last), 2, "the last picture waits in retired");
        })
    });
}

#[test]
fn a_failure_hands_the_last_picture_on() {
    stopped_by(false, |show, cx| {
        show.media_seen(Err("Cannot Open".into()), cx);
    });
}

#[test]
fn the_next_slide_hands_the_last_picture_on() {
    stopped_by(true, |show, cx| show.next(cx));
}

#[test]
fn the_end_screen_hands_the_last_picture_on() {
    stopped_by(false, |show, cx| show.next(cx));
}
