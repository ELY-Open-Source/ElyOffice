//! Video pictures reach the show and wait to be freed.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use ely_office::prelude::*;
use gpui::{Entity, TestAppContext};

use super::objects::blank;
use super::play_more::started;
use super::{settle, update};
use crate::slideshow::Show;

/// Ticks the show's player until `done`, in real time.
fn play_until(cx: &mut TestAppContext, show: &Entity<Show>, done: impl Fn(&Show) -> bool) {
    let end = Instant::now() + Duration::from_secs(10);
    while !cx.update(|app| done(show.read(app))) {
        assert!(Instant::now() < end, "the video never got there");
        cx.update(|app| show.update(app, |show, cx| show.media_tick(cx)));
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn each_picture_a_video_leaves_waits_to_be_freed() {
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
    let show = started(&mut cx, &deck, true);
    cx.update(|app| show.update(app, |show, cx| show.toggle_media(0, cx)));
    play_until(&mut cx, &show, |show| show.media_frame().is_some());
    let first = cx.update(|app| show.read(app).media_frame().expect("a picture").0);
    assert_eq!(first.size(0), gpui::size(160.into(), 90.into()), "the clip's own size");
    let end = Instant::now() + Duration::from_secs(10);
    // Counted in the tick that replaces it, before a paint.
    while !cx.update(|app| {
        show.update(app, |show, cx| {
            show.media_tick(cx);
            let replaced = show.media_frame().is_some_and(|(now, _)| !Arc::ptr_eq(&now, &first));
            if replaced {
                assert_eq!(Arc::strong_count(&first), 2, "the replaced picture waits in retired");
            }
            replaced
        })
    }) {
        assert!(Instant::now() < end, "the video never moved on");
        std::thread::sleep(Duration::from_millis(20));
    }
    let last = cx.update(|app| show.read(app).media_frame().expect("a picture").0);
    let failed = Err("Cannot Open".to_string());
    // Checked before a window paints and frees it.
    cx.update(|app| {
        show.update(app, |show, cx| {
            show.media_seen(failed, cx);
            assert!(show.media_frame().is_none(), "playback stopped");
            assert_eq!(Arc::strong_count(&last), 2, "the last picture waits in retired");
        })
    });
}
