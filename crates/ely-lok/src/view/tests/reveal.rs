//! §20: a turned page shows whole, the next one ready.

use super::*;

/// Prepares frames until one asks for nothing.
fn settle(cx: &gpui::TestAppContext, view: &Entity<LokView>, bounds: Bounds<Pixels>) {
    let deadline = std::time::Instant::now() + crate::client::tests::HANG;
    while std::time::Instant::now() < deadline {
        cx.update(|app| view.update(app, |view, _| _ = view.prepare(bounds)));
        if cx.update(|app| view.read(app).fetching.is_empty()) {
            return;
        }
        cx.run_until_parked();
        std::thread::sleep(std::time::Duration::from_millis(5));
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

/// Waits until every tile of the asked screen is in.
fn landed(cx: &gpui::TestAppContext, view: &Entity<LokView>) {
    let deadline = std::time::Instant::now() + crate::client::tests::HANG;
    while std::time::Instant::now() < deadline {
        cx.run_until_parked();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let whole = cx.update(|app| {
            let view = view.read(app);
            view.on_screen.iter().all(|key| view.tiles.get(key).is_some_and(|tile| !tile.stale))
        });
        if whole {
            return;
        }
    }
    panic!("the screen never came whole");
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
    cx.update(|app| view.update(app, |view, _| view.selection = vec![(1500, view.scroll.1 + 3000, 3000, 300)]));
    let before = cx.update(|app| view.update(app, |view, _| view.prepare(bounds)));
    assert!(!before.tiles.is_empty(), "a whole first screen");
    assert!(!before.selection.is_empty(), "the selection shows on it");
    let spot = point(px(100.0), px(100.0));
    let under = |cx: &gpui::TestAppContext| cx.update(|app| view.read(app).doc_point(spot));
    let old = under(&cx);
    let caret = cx.update(|app| view.read(app).cursor_bounds());
    assert!(caret.is_some(), "the caret shows on the first screen");
    // A selected object and a table on the old screen.
    let top = cx.update(|app| view.read(app).scroll.1) + 1500;
    let frame = (1500, top, 3000, 1500);
    let line = |at: i32| crate::table::Line { position: at, min: 0, max: 6000, hidden: false };
    let span = |offset, inner| crate::table::Lines { left: 0, right: 6000, offset, inner, ends: None };
    let table = crate::table::Table { columns: span(1500, vec![line(3000)]), rows: span(top + 3000, vec![]) };
    cx.update(|app| view.update(app, |view, _| (view.graphic, view.handle_size) = (Some(frame), px(6.0))));
    cx.update(|app| view.update(app, |view, _| view.read_borders(Some(table), u64::MAX)));
    let corner = cx.update(|app| view.read(app).window_point((4500, top + 1500)));
    let border = cx.update(|app| view.read(app).window_point((4500, top + 4500)));
    cx.update(|app| view.update(app, |view, _| view.scroll.1 = view.extent().1 / 2));
    assert_eq!(under(&cx), old, "read before the next paint, it maps the same");
    let waiting = cx.update(|app| view.update(app, |view, _| view.prepare(bounds)));
    assert_eq!(drawn(&waiting), drawn(&before), "the old screen stays, every tile in place");
    assert_eq!(waiting.pages, before.pages, "its pages stay with it");
    assert_eq!(under(&cx), old, "a click lands on the screen it sees");
    assert_eq!(cx.update(|app| view.read(app).cursor_bounds()), caret, "the caret stays with it");
    assert_eq!(waiting.selection, before.selection, "so does the selection");
    landed(&cx, &view);
    assert_eq!(under(&cx), old, "every tile in, not yet drawn: clicks keep the old screen");
    assert_eq!(cx.update(|app| view.read(app).cursor_bounds()), caret, "so does the caret");
    cx.update(|app| {
        view.update(app, |view, _| {
            assert!(view.grab(corner), "and the handle where it shows");
            view.grip = None;
            assert!(view.hold_border(border), "and the table border");
        })
    });
    let after = cx.update(|app| view.update(app, |view, _| view.prepare(bounds)));
    assert_ne!(drawn(&after), drawn(&before), "the next frame shows the new screen");
    let next = cx.update(|app| view.update(app, |view, _| view.prepare(bounds)));
    assert_eq!((&after.pages, drawn(&after)), (&next.pages, drawn(&next)), "the first new frame is already the new screen");
    let shown = cx.update(|app| view.read(app).on_screen.len());
    assert_eq!(after.tiles.len(), shown, "the new screen comes whole");
    assert_ne!(under(&cx), old, "then clicks land on the new one");
    assert!(after.selection.is_empty(), "the old screen's selection lies off it");
    let old: HashSet<usize> = drawn(&before).into_iter().map(|(_, at)| at).collect();
    assert!(drawn(&after).iter().all(|(_, at)| !old.contains(at)), "and none of the old screen");
}

#[test]
fn a_zoomed_sheet_maps_by_the_screen_it_holds() {
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let mut cx = gpui::TestAppContext::single();
    let (view, geometry) = super::large::far_down(&mut cx, &install);
    let bounds = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(800.0), px(600.0)) };
    cx.update(|app| view.update(app, |view, cx| view.set_frozen((0, geometry.rows.start(5)), cx)));
    settle(&cx, &view, bounds);
    let mid = point(px(400.0), px(300.0));
    let row = cx.update(|app| geometry.rows.at(view.read(app).doc_point(mid).1));
    let (x, y, w, h) = super::large::cell(&geometry, 1, row);
    let centre = (x + w / 2, y + h / 2);
    // Either side of the frozen rows' edge.
    let edge = cx.update(|app| view.read(app).window_point((x, geometry.rows.start(5))));
    let (above, below) = (edge - point(px(0.0), px(0.5)), edge + point(px(0.0), px(0.5)));
    let shown = |cx: &gpui::TestAppContext| {
        cx.update(|app| {
            let view = view.read(app);
            let rows = |at| geometry.rows.at(view.doc_point(at).1);
            (view.window_point(centre), view.is_visible((x, y, w, h)), rows(mid), (rows(above), rows(below)), view.scrolled_start())
        })
    };
    cx.update(|app| view.update(app, |view, _| view.selection = vec![(x, y, w, h)]));
    let before = cx.update(|app| view.update(app, |view, _| view.prepare(bounds)));
    let seen = shown(&cx);
    assert!(seen.1 && seen.3.0 == 4 && seen.3.1 > 4 && !before.selection.is_empty(), "the selected cell shows below the frozen rows");
    cx.update(|app| view.update(app, |view, cx| view.set_zoom(0.8, cx)));
    assert_eq!(shown(&cx), seen, "headers read before the canvas prepares");
    let waiting = cx.update(|app| view.update(app, |view, _| view.prepare(bounds)));
    assert_eq!(drawn(&waiting), drawn(&before), "the old screen stays");
    assert_eq!(shown(&cx), seen, "cells, clicks, panes and headers map by it");
    assert_eq!(waiting.selection, before.selection, "the selection stays on its cell");
    let scroll = cx.update(|app| {
        view.update(app, |view, _| {
            let scroll = view.scroll().1;
            view.keep_scroll(|_| {});
            (scroll, view.scroll().1)
        })
    });
    assert!(scroll.0.abs_diff(scroll.1) < 100, "a rescale keeps the view's own start: {scroll:?}");
    cx.update(|app| {
        view.update(app, |view, _| {
            for kind in [0, 1] {
                view.mouse(kind, seen.0, 1, 1, &gpui::Modifiers::default());
            }
        })
    });
    let want = format!("B{}", row + 1);
    assert_eq!(super::large::selected(&view, &mut cx, &want), want, "a click picks the cell it shows");
}

#[test]
fn a_move_retries_a_failed_tile_and_holds_for_every_reader() {
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let cx = gpui::TestAppContext::single();
    let (view, bounds) = long_view(&cx, &install);
    cx.update(|app| view.update(app, |view, _| view.scroll.1 = 12 * TILE_TWIPS));
    settle(&cx, &view, bounds);
    let keys = |cx: &gpui::TestAppContext| cx.update(|app| view.read(app).on_screen.iter().copied().collect::<HashSet<_>>());
    let screen = keys(&cx);
    // A shown tile failed to repaint and stays stale.
    cx.update(|app| {
        view.update(app, |view, _| {
            let key = view.on_screen[0];
            view.tiles.get_mut(&key).expect("a shown tile").stale = true;
            view.failed.insert((key, view.generation));
        })
    });
    let before = cx.update(|app| view.update(app, |view, _| view.prepare(bounds)));
    assert_eq!(before.tiles.len(), screen.len(), "the stale tile still shows");
    let spot = (1500, 12 * TILE_TWIPS + 3000);
    let at = |cx: &gpui::TestAppContext| cx.update(|app| view.read(app).window_point(spot));
    let old = at(&cx);
    cx.update(|app| view.update(app, |view, _| view.scroll.1 += 300));
    assert_eq!(at(&cx), old, "read before the canvas, the move waits for the retry");
    let waiting = cx.update(|app| view.update(app, |view, _| view.prepare(bounds)));
    assert_eq!(cx.update(|app| view.read(app).scroll().1), 12 * TILE_TWIPS + 300, "the view moved");
    assert_eq!(keys(&cx), screen, "the same tiles cover the moved screen");
    assert_eq!(drawn(&waiting), drawn(&before), "the canvas holds the old screen");
    assert_eq!(at(&cx), old, "and maps by it");
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
fn a_hidden_view_drops_its_queued_screen() {
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let cx = gpui::TestAppContext::single();
    let (view, bounds) = long_view(&cx, &install);
    // One frame asks for the screen; then its tab hides.
    cx.update(|app| {
        view.update(app, |view, cx| {
            view.invalidate(cx);
            _ = view.prepare(bounds);
            assert!(!view.fetching.is_empty(), "the screen is queued");
            view.hide();
        })
    });
    cx.run_until_parked();
    cx.update(|app| {
        let view = view.read(app);
        assert!(view.fetching.is_empty() && view.tiles.values().all(|tile| tile.stale), "nothing paints behind");
    });
    settle(&cx, &view, bounds);
    cx.update(|app| {
        let view = view.read(app);
        let fresh = view.on_screen.iter().all(|key| view.tiles.get(key).is_some_and(|tile| !tile.stale));
        assert!(fresh, "shown again, the screen paints");
    });
}

#[test]
fn an_edit_is_timed_by_the_first_whole_fresh_screen() {
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let cx = gpui::TestAppContext::single();
    let (view, bounds) = long_view(&cx, &install);
    let frame = |cx: &gpui::TestAppContext| {
        cx.update(|app| {
            view.update(app, |view, _| {
                // As the canvas: a shown generation times edits.
                let shows = view.prepare(bounds).shows;
                if let Some(generation) = shows {
                    view.clock.drawn(generation);
                }
                (shows, view.clock.waiting())
            })
        })
    };
    let generation = cx.update(|app| view.read(app).generation);
    assert_eq!(frame(&cx).0, Some(generation), "a settled screen shows its generation");
    cx.update(|app| {
        view.update(app, |view, _| {
            let far = view.tiles.keys().copied().find(|key| !view.on_screen.contains(key)).expect("a tile ahead");
            view.tiles.get_mut(&far).expect("the tile").stale = true;
        })
    });
    assert_eq!(frame(&cx).0, Some(generation), "an off-screen stale tile leaves the screen whole");
    cx.update(|app| {
        view.update(app, |view, _| {
            crate::client::tests::click(&mut view.worker.lock().expect("the worker lock"), &view.doc, 1500, 1500);
            view.commit(None, "7");
        })
    });
    // The key lands; its screen comes whole, undrawn.
    let mut ready = None;
    for _ in 0..200 {
        cx.run_until_parked();
        ready = cx.update(|app| {
            view.update(app, |view, _| {
                _ = view.prepare(bounds);
                let whole = view.on_screen.iter().all(|key| view.tiles.get(key).is_some_and(|tile| !tile.stale));
                view.clock.awaited().first().copied().flatten().filter(|_| whole && view.fetching.is_empty())
            })
        });
        if ready.is_some() {
            break;
        }
    }
    let shows = ready.expect("the key landed and its screen came whole");
    // A recovery point's repaint: every tile stale, no notice.
    cx.update(|app| view.update(app, |view, cx| view.invalidate(cx)));
    assert_eq!(frame(&cx), (None, true), "a stale screen times nothing");
    let mut timed = false;
    for _ in 0..200 {
        cx.run_until_parked();
        let (drawn, waiting) = frame(&cx);
        if !waiting {
            assert!(drawn.is_some_and(|drawn| drawn >= shows), "timed by a screen at {shows} or later");
            timed = true;
            break;
        }
    }
    assert!(timed, "the fresh screen times the key");
}

#[test]
fn a_whole_screen_wider_than_the_page_is_seen() {
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let cx = gpui::TestAppContext::single();
    let (view, _) = long_view(&cx, &install);
    // The shown area runs past the page.
    let wide = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(2000.0), px(1200.0)) };
    settle(&cx, &view, wide);
    landed(&cx, &view);
    let blind = cx.update(|app| {
        let view = view.read(app);
        assert!(view.geom.twips_of(2000.0) > view.extent().0, "the window is wider than the page");
        let mut worker = view.worker.lock().expect("the worker lock");
        worker.state(&view.doc).expect("the state").2
    });
    assert!(!blind, "a whole screen sees the next edit's invalidations");
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
