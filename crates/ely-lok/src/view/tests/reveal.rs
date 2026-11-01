//! §20: a turned page shows whole, the next one ready.

use super::*;

/// Prepares frames until one asks for nothing.
fn settle(cx: &gpui::TestAppContext, view: &Entity<LokView>, bounds: Bounds<Pixels>) {
    for _ in 0..200 {
        cx.update(|app| view.update(app, |view, _| _ = view.prepare(bounds)));
        if cx.update(|app| view.read(app).fetching.is_empty()) {
            return;
        }
        cx.run_until_parked();
    }
    panic!("the lane never rested");
}

/// A long document laid out at 800 by 600.
fn long_view(cx: &gpui::TestAppContext, install: &std::path::Path) -> (Entity<LokView>, Bounds<Pixels>) {
    let view = cx.update(|app| live_view(app, install).0);
    cx.update(|app| {
        view.update(app, |view, cx| {
            {
                let mut worker = view.worker.lock().expect("the worker lock");
                for _ in 0..200 {
                    worker.post_key(&view.doc, 0, 1280).expect("Return posts");
                }
            }
            let measured = super::super::measure(&view.worker, &view.doc);
            view.apply_batch(crate::Events { generation: 1, all: true, ..crate::Events::default() }, Some(measured), cx);
            view.viewport = (800.0, 600.0);
        })
    });
    let bounds = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(800.0), px(600.0)) };
    settle(cx, &view, bounds);
    (view, bounds)
}

/// Each drawn tile's place and image address.
fn drawn(painted: &super::super::render::Painted) -> Vec<(Bounds<Pixels>, usize)> {
    painted.tiles.iter().map(|(place, image, _)| (*place, Arc::as_ptr(image) as usize)).collect()
}

#[test]
fn a_jump_keeps_the_whole_old_screen_until_the_new_one_is_whole() {
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let cx = gpui::TestAppContext::single();
    let (view, bounds) = long_view(&cx, &install);
    let before = cx.update(|app| view.update(app, |view, _| view.prepare(bounds)));
    assert!(!before.tiles.is_empty(), "a whole first screen");
    let spot = point(px(100.0), px(100.0));
    let under = |cx: &gpui::TestAppContext| cx.update(|app| view.read(app).doc_point(spot));
    let old = under(&cx);
    let caret = cx.update(|app| view.read(app).cursor_bounds());
    assert!(caret.is_some(), "the caret shows on the first screen");
    cx.update(|app| view.update(app, |view, _| view.scroll.1 = view.extent().1 / 2));
    let waiting = cx.update(|app| view.update(app, |view, _| view.prepare(bounds)));
    assert_eq!(drawn(&waiting), drawn(&before), "the old screen stays, every tile in place");
    assert_eq!(waiting.pages, before.pages, "its pages stay with it");
    assert_eq!(under(&cx), old, "a click lands on the screen it sees");
    assert_eq!(cx.update(|app| view.read(app).cursor_bounds()), caret, "the caret stays with it");
    let after = (0..200)
        .find_map(|_| {
            cx.run_until_parked();
            let frame = cx.update(|app| view.update(app, |view, _| view.prepare(bounds)));
            (drawn(&frame) != drawn(&before)).then_some(frame)
        })
        .expect("the new screen comes");
    let next = cx.update(|app| view.update(app, |view, _| view.prepare(bounds)));
    assert_eq!((&after.pages, drawn(&after)), (&next.pages, drawn(&next)), "the first new frame is already the new screen");
    let shown = cx.update(|app| view.read(app).on_screen.len());
    assert_eq!(after.tiles.len(), shown, "the new screen comes whole");
    assert_ne!(under(&cx), old, "then clicks land on the new one");
    let old: HashSet<usize> = drawn(&before).into_iter().map(|(_, at)| at).collect();
    assert!(drawn(&after).iter().all(|(_, at)| !old.contains(at)), "and none of the old screen");
}

#[test]
fn the_heading_follows_up_and_down_moves_only() {
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let cx = gpui::TestAppContext::single();
    let (view, bounds) = long_view(&cx, &install);
    let at = |y: i32, width: f32| {
        cx.update(|app| {
            view.update(app, |view, _| {
                view.scroll.1 = y;
                let bounds = Bounds { size: size(px(width), bounds.size.height), ..bounds };
                _ = view.prepare(bounds);
                view.heading
            })
        })
    };
    at(0, 800.0);
    assert_eq!(at(7680, 800.0), 1, "down");
    assert_eq!(at(3840, 800.0), -1, "up");
    assert_eq!(at(3840, 700.0), -1, "a narrower window keeps the heading");
    assert_eq!(at(9000, 700.0), 1, "down again");
}

#[test]
fn the_next_screen_paints_ahead_and_old_jobs_are_skipped() {
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let cx = gpui::TestAppContext::single();
    let (view, bounds) = long_view(&cx, &install);
    cx.update(|app| {
        let view = view.read(app);
        let bottom = view.on_screen.iter().map(|key| key.1).max().expect("a screen");
        assert!(view.tiles.contains_key(&(0, bottom + 1)), "the next screen is painted ahead");
    });
    cx.update(|app| view.update(app, |view, _| _ = view.prepare(bounds)));
    assert!(cx.update(|app| view.read(app).fetching.is_empty()), "a whole screen with its next one asks nothing");
    let far = (0, 400);
    cx.update(|app| view.update(app, |view, _| view.request_tile(far)));
    cx.run_until_parked();
    cx.update(|app| {
        let view = view.read(app);
        assert!(!view.tiles.contains_key(&far) && view.fetching.is_empty(), "a job nobody waits for is skipped");
    });
}

#[test]
fn page_keys_turn_parts_like_up_and_down() {
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view(app, &install);
        view.update(app, |view, _| {
            let key = |name: &'static str| gpui::Keystroke::parse(name).expect("a keystroke");
            assert!(!view.turn_key(&key("pagedown")), "a text document keeps its keys");
            view.turn_parts();
            for name in ["pagedown", "pageup", "down", "up"] {
                assert!(view.turn_key(&key(name)), "{name} turns, not the kernel's key");
            }
        });
    });
}
