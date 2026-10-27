use super::*;

mod borders;
mod clip;
mod commands;
mod edge;
mod fit;
mod formula;
mod input;
mod large;
mod show_borders;
mod theme;
mod text;
mod zoom;
use gpui::{App, Bounds, Entity, point, size};

#[test]
fn geom_converts_both_ways() {
    let geom = Geom {
        zoom: 2.0,
        dpi: 2.0,
    };
    assert_eq!(geom.px_of(15), 2.0, "15 twips at 200% zoom is 2 logical px");
    assert_eq!(geom.twips_of(2.0), 15, "2 logical px back to 15 twips");
    assert_eq!(geom.tile_pixels(), 1024, "the texture is 1024 device px");
    let base = Geom {
        zoom: 1.0,
        dpi: 2.0,
    };
    assert_eq!(base.px_of(TILE_TWIPS), 256.0, "one tile is 256 logical px");
    assert_eq!(
        base.twips_of(256.0),
        TILE_TWIPS,
        "256 logical px is one tile"
    );
    assert_eq!(base.tile_pixels(), 512, "the texture is 512 device px");
}

pub(super) fn live_view(
    app: &mut App,
    install: &std::path::Path,
) -> (Entity<LokView>, Arc<Mutex<crate::Worker>>) {
    live_view_of(app, install, "note.fodt", crate::client::tests::FODT)
}

/// A view on `body` saved as `name`.
pub(super) fn live_view_of(
    app: &mut App,
    install: &std::path::Path,
    name: &str,
    body: &str,
) -> (Entity<LokView>, Arc<Mutex<crate::Worker>>) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker =
        crate::Worker::spawn(install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join(name);
    std::fs::write(&source, body).expect("the fixture writes");
    let doc = worker
        .load(&crate::file_url(&source))
        .expect("the document loads");
    std::mem::forget(dir);
    let shared = Arc::new(Mutex::new(worker));
    let laid = super::layout(&mut shared.lock().expect("the worker lock"), &doc)
        .expect("the layout reads");
    let view = LokView::open(Arc::clone(&shared), doc, laid, app);
    (view, shared)
}

#[test]
fn edits_grow_the_view_size() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view(app, &install);
        view.update(app, |view, cx| {
            let before = view.doc_size;
            {
                let mut worker = view.worker.lock().expect("the worker lock");
                for _ in 0..80 {
                    worker.post_key(&view.doc, 0, 1280).expect("Return posts");
                }
            }
            let measured = super::measure(&view.worker, &view.doc);
            view.apply_batch(
                crate::Events {
                    generation: 1,
                    all: true,
                    rects: Vec::new(),
                    ..crate::Events::default()
                },
                Some(measured),
                cx,
            );
            assert!(
                view.doc_size.1 > before.1,
                "the view size follows the document: {before:?} to {:?}",
                view.doc_size
            );
        });
    });
}

#[test]
fn releasing_the_view_stops_the_watch() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    let (view, shared) = cx.update(|app| live_view(app, &install));
    for _ in 0..5 {
        cx.executor().tick();
    }
    let weak = Arc::downgrade(&shared);
    drop(shared);
    cx.update(|_| drop(view));
    let mut released = false;
    for _ in 0..20 {
        cx.executor().tick();
        released = weak.upgrade().is_none();
        if released {
            break;
        }
    }
    assert!(released, "the idle watch releases the worker");
}

#[test]
fn tiles_paint_in_logical_pixels_at_2x() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view(app, &install);
        view.update(app, |view, cx| {
            view.geom.dpi = 2.0;
            let side = view.geom.tile_pixels() as usize;
            assert_eq!(side, 512, "a 2x tile texture is 512 px");
            for key in [(0, 0), (1, 0)] {
                let tile = crate::Tile {
                    generation: 0,
                    pixels: vec![255u8; side * side * 4],
                };
                view.finish_tile(key, view.geom, Ok(tile), cx);
            }
            let bounds = Bounds {
                origin: point(px(10.0), px(20.0)),
                size: size(px(800.0), px(600.0)),
            };
            let ready = view.prepare(bounds).tiles;
            let rects: Vec<_> = ready.iter().map(|(rect, ..)| *rect).collect();
            assert!(rects.len() >= 2, "both cached tiles paint: {rects:?}");
            assert_eq!(rects[0].size.width, px(256.0), "one tile is 256 logical px");
            assert_eq!(rects[0].origin, point(px(10.0), px(20.0)));
            assert_eq!(rects[1].origin.x, px(266.0), "the next tile sits 256 px on");
            assert_eq!(view.doc_point(point(px(266.0), px(20.0))), (TILE_TWIPS, 0));
        });
    });
}

#[test]
fn a_red_tile_stays_red_in_the_render_image() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view(app, &install);
        view.update(app, |view, cx| {
            let side = view.geom.tile_pixels() as usize;
            let red_bgra = [0u8, 0, 255, 255];
            let tile = crate::Tile {
                generation: 0,
                pixels: red_bgra.repeat(side * side),
            };
            view.finish_tile((0, 0), view.geom, Ok(tile), cx);
            let image = &view.tiles[&(0, 0)].image;
            let bytes = image.as_bytes(0).expect("one frame");
            assert_eq!(&bytes[..4], &red_bgra, "GPUI gets the BGRA bytes as is");
        });
    });
}

#[test]
fn the_view_clamps_scroll_and_guards_generations() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view(app, &install);
        view.update(app, |view, cx| {
            view.viewport = (800.0, 600.0);
            view.scroll_by(0.0, -100_000.0, cx);
            let max_y = (view.doc_size.1 - view.geom.twips_of(600.0)).max(0);
            assert_eq!(view.scroll.1, max_y, "scroll clamps at the bottom");
            view.scroll_by(0.0, 1_000_000.0, cx);
            assert_eq!(view.scroll.1, 0, "scroll clamps at the top");
            assert!(view.doc_size.0 > 0 && view.doc_size.1 > 0, "the size loads");

            let geom = view.geom;
            let side = geom.tile_pixels() as usize;
            let painted = |generation: u64| {
                Ok(crate::Tile {
                    generation,
                    pixels: vec![0u8; side * side * 4],
                })
            };
            view.finish_tile((0, 0), geom, painted(0), cx);
            assert_eq!(view.tile_count(), 1, "a current tile lands");
            view.apply_batch(
                crate::Events {
                    generation: 5,
                    all: true,
                    rects: Vec::new(),
                    ..crate::Events::default()
                },
                None,
                cx,
            );
            assert_eq!(view.tile_count(), 0, "an all batch outdates the grid");
            assert_eq!(view.tiles.len(), 1, "the old image shows until its repaint");
            view.finish_tile((0, 0), geom, painted(4), cx);
            assert_eq!(view.tile_count(), 0, "a stale tile never lands");
            view.finish_tile((0, 0), geom, painted(5), cx);
            assert_eq!(view.tile_count(), 1, "a fresh tile lands again");
            view.apply_batch(
                crate::Events {
                    generation: 6,
                    all: false,
                    rects: vec![(100, 100, 50, 50)],
                    ..crate::Events::default()
                },
                None,
                cx,
            );
            assert_eq!(
                view.tile_count(),
                0,
                "a rect batch outdates intersecting tiles"
            );
            view.finish_tile((0, 0), geom, painted(6), cx);
            view.finish_tile((3, 3), geom, painted(6), cx);
            view.apply_batch(
                crate::Events {
                    generation: 7,
                    all: false,
                    rects: vec![(284, 1418, 3715, 551)],
                    ..crate::Events::default()
                },
                None,
                cx,
            );
            assert!(!view.tiles[&(3, 3)].stale, "a distant tile stays current");
            assert!(
                view.tiles[&(0, 0)].stale,
                "the dirty tile waits for its repaint"
            );
            let bounds = Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(800.0), px(600.0)),
            };
            let painted = view.prepare(bounds);
            assert!(!painted.tiles.is_empty(), "stale tiles still paint");
            assert!(
                view.fetching.contains(&(0, 0)),
                "and their repaint is asked"
            );
        });
    });
}

#[test]
fn scroll_stays_inside_after_shrink_zoom_and_resize() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view(app, &install);
        view.update(app, |view, cx| {
            let bottom =
                |view: &LokView| (view.doc_size.1 - view.geom.twips_of(view.viewport.1)).max(0);
            view.viewport = (800.0, 600.0);
            {
                let mut worker = view.worker.lock().expect("the worker lock");
                for _ in 0..80 {
                    worker.post_key(&view.doc, 0, 1280).expect("Return posts");
                }
            }
            let batch = |generation| crate::Events {
                generation,
                all: true,
                ..crate::Events::default()
            };
            view.apply_batch(batch(1), None, cx);
            view.scroll_by(0.0, -1_000_000.0, cx);
            assert_eq!(view.scroll.1, bottom(view), "scrolled to the grown bottom");
            {
                let mut worker = view.worker.lock().expect("the worker lock");
                worker
                    .run(&view.doc, "edit.select_all", &[])
                    .expect("select all");
                worker.cut(&view.doc).expect("the cut runs");
            }
            view.apply_batch(batch(2), None, cx);
            assert_eq!(
                view.scroll.1,
                bottom(view),
                "a shrink pulls the scroll back"
            );

            view.set_zoom(2.0, cx);
            view.scroll_by(0.0, -1_000_000.0, cx);
            view.set_zoom(0.25, cx);
            assert_eq!(
                view.scroll.1,
                bottom(view),
                "a smaller zoom pulls the scroll back"
            );

            view.set_zoom(2.0, cx);
            view.viewport = (800.0, 300.0);
            view.scroll_by(0.0, -1_000_000.0, cx);
            let bounds = Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(800.0), px(1200.0)),
            };
            view.prepare(bounds);
            assert_eq!(
                view.scroll.1,
                bottom(view),
                "a taller viewport pulls the scroll back"
            );
        });
    });
}

#[test]
fn a_moved_cursor_scrolls_into_view_and_a_still_one_does_not() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view(app, &install);
        view.update(app, |view, cx| {
            view.viewport = (800.0, 200.0);
            let far = Some((1418, view.doc_size.1 - 600, 1, 276));
            let batch = || crate::Events {
                cursor: far,
                ..crate::Events::default()
            };
            view.apply_batch(batch(), None, cx);
            assert!(view.scroll.1 > 0, "the view follows the cursor");
            view.scroll_by(0.0, 1_000_000.0, cx);
            view.apply_batch(batch(), None, cx);
            assert_eq!(view.scroll.1, 0, "a still cursor leaves the scroll alone");
        });
    });
}

#[test]
fn a_closing_view_hands_over_every_image() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view(app, &install);
        view.update(app, |view, cx| {
            let side = view.geom.tile_pixels() as usize;
            let tile = |generation| Ok(crate::Tile { generation, pixels: vec![0; side * side * 4] });
            let (geom, generation) = (view.geom, view.generation);
            view.finish_tile((0, 0), geom, tile(generation), cx);
            view.finish_tile((0, 0), geom, tile(generation), cx);
            assert_eq!(view.tiles.len(), 1);
            let images = view.retire_all();
            assert!(!images.is_empty(), "the shown and the replaced tile go");
            assert!(view.tiles.is_empty() && view.retired.is_empty(), "the view keeps none");
        });
    });
}

#[test]
fn zooming_keeps_the_pointed_spot_and_walks_the_steps() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view(app, &install);
        view.update(app, |view, cx| {
            view.viewport = (800.0, 600.0);
            view.origin = gpui::point(gpui::px(0.0), gpui::px(0.0));
            view.doc_size = (60_000, 80_000);
            view.scroll = (3_000, 9_000);
            view.fit = true;
            let at = gpui::point(gpui::px(300.0), gpui::px(200.0));
            let spot = |view: &LokView| {
                (view.scroll.0 + view.geom.twips_of(300.0), view.scroll.1 + view.geom.twips_of(200.0))
            };
            let before = spot(view);
            view.zoom_at(2.0, Some(at), cx);
            let after = spot(view);
            assert!((before.0 - after.0).abs() <= 1 && (before.1 - after.1).abs() <= 1, "{before:?} {after:?}");
            assert!(!view.fitting(), "a chosen zoom ends fitting");
            view.set_zoom(1.0, cx);
            view.zoom_step(true, cx);
            assert_eq!(view.zoom(), 1.25);
            view.zoom_step(false, cx);
            view.zoom_step(false, cx);
            assert_eq!(view.zoom(), 0.8);
            view.set_zoom(9.0, cx);
            assert_eq!(view.zoom(), 4.0, "the most is 400 %");
            view.zoom_step(true, cx);
            assert_eq!(view.zoom(), 4.0);
            view.fit_page(cx);
            assert!(view.fitting());
        });
    });
}

#[test]
fn ctrl_and_a_notch_zoom_one_step_about_the_pointer() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view(app, &install);
        view.update(app, |view, cx| {
            view.viewport = (800.0, 600.0);
            view.origin = gpui::point(gpui::px(0.0), gpui::px(0.0));
            view.doc_size = (60_000, 80_000);
            view.scroll = (3_000, 9_000);
            view.set_zoom(1.0, cx);
            let at = gpui::point(gpui::px(300.0), gpui::px(200.0));
            let wheel = |modifiers, y| gpui::ScrollEvent {
                position: at,
                delta: gpui::ScrollDelta::Lines(gpui::point(0.0, y)),
                modifiers,
                ..Default::default()
            };
            let other = match cfg!(target_os = "macos") {
                true => gpui::Modifiers { control: true, ..Default::default() },
                false => gpui::Modifiers { platform: true, ..Default::default() },
            };
            assert!(!view.wheel_zoom(&wheel(other, 3.0), cx), "only Ctrl, or Cmd on macOS");
            assert_eq!(view.zoom(), 1.0);
            let spot = |view: &LokView| view.scroll.1 + view.geom.twips_of(200.0);
            let before = spot(view);
            assert!(view.wheel_zoom(&wheel(gpui::Modifiers::secondary_key(cx), 3.0), cx));
            assert_eq!(view.zoom(), 1.25, "a three-line notch is one step");
            assert!((spot(view) - before).abs() <= 1, "the pointed spot stays");
            view.wheel_zoom(&wheel(gpui::Modifiers::secondary_key(cx), -1.0), cx);
            view.wheel_zoom(&wheel(gpui::Modifiers::secondary_key(cx), -1.0), cx);
            assert_eq!(view.zoom(), 0.8);
        });
    });
}
