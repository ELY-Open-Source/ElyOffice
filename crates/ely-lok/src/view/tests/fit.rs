use super::*;

#[test]
fn a_fitted_slide_fills_the_viewport_centred() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let deck = crate::client::tests::show::DECK;
        let (view, _) = live_view_of(app, &install, "deck.fodp", deck);
        view.update(app, |view, cx| {
            view.fit_page(cx);
            let bounds = Bounds {
                origin: point(px(10.0), px(20.0)),
                size: size(px(800.0), px(600.0)),
            };
            view.prepare(bounds);
            let (w, h) = view.doc_size;
            let want = (800.0 * 15.0 / w as f32).min(600.0 * 15.0 / h as f32) * 0.94;
            assert!(
                (view.geom.zoom - want).abs() < 0.001,
                "{} vs {want}",
                view.geom.zoom
            );
            assert!(
                view.spare.0 > 0.0 && view.spare.1 > 0.0,
                "centred: {:?}",
                view.spare
            );
            let corner = view.window_point((0, 0));
            assert_eq!(view.doc_point(corner), (0, 0), "points map both ways");
            // A cursor far out must not move a fitted page.
            view.reach = (w * 3, h * 3);
            view.prepare(bounds);
            assert_eq!(view.window_point((0, 0)), corner, "the page stays put");
            let far = view.window_point((w, h));
            assert!(
                f32::from(far.y) < 620.0 && f32::from(far.x) < 810.0,
                "the page fits"
            );
        });
    });
}

#[test]
fn a_resized_slide_reaches_the_view_and_still_fits() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    let view = cx.update(|app| {
        let deck = crate::client::tests::show::DECK;
        live_view_of(app, &install, "deck.fodp", deck).0
    });
    cx.update(|app| {
        view.update(app, |view, cx| {
            view.fit_page(cx);
            view.run("slide.size", &["300000", "300000", "true"])
                .expect("sent");
        })
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while cx.update(|app| view.read(app).doc_size.0 < 100_000) {
        assert!(
            std::time::Instant::now() < deadline,
            "the view never read the new size"
        );
        if !cx.executor().tick() {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
    cx.update(|app| {
        view.update(app, |view, _| {
            let bounds = Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(800.0), px(600.0)),
            };
            view.prepare(bounds);
            let (w, h) = view.doc_size;
            let far = view.window_point((w, h));
            assert!(
                f32::from(far.x) <= 800.0 && f32::from(far.y) <= 600.0,
                "a 3 m page fits: {far:?}"
            );
        })
    });
}

#[test]
fn a_page_stays_centred_when_the_cursor_moves() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view(app, &install);
        view.update(app, |view, cx| {
            let wide = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(2400.0), px(900.0)) };
            view.prepare(wide);
            let centred = view.spare.0;
            assert!(centred > 0.0, "a page narrower than the view sits centred");
            view.show((1440, 1440, 20, 300), cx);
            view.prepare(wide);
            assert_eq!(view.spare.0, centred, "moving the cursor keeps it centred");
            let narrow = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(800.0), px(900.0)) };
            view.prepare(narrow);
            view.set_zoom(2.0, cx);
            view.prepare(narrow);
            let right = view.scroll.0 + view.geom.twips_of(view.viewport.0);
            assert!(right <= view.doc_size.0, "no grey past the page: {right} > {}", view.doc_size.0);
        })
    });
}

#[test]
fn zooming_keeps_the_left_page_edge_or_the_pointed_spot() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view(app, &install);
        view.update(app, |view, cx| {
            let room = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(1200.0), px(900.0)) };
            view.prepare(room);
            let page = *view.pages.first().expect("a laid out page");
            // Unclamped, so a cut edge reads below zero.
            let left = |view: &LokView| view.spare.0 + view.geom.px_of(view.tile_of((page.0, page.1)).0 - view.scroll.0);
            for _ in 0..3 {
                view.zoom_step(true, cx);
                view.prepare(room);
                assert!(left(view) >= 0.0, "at {}% the left edge is at {}", view.zoom() * 100.0, left(view));
            }
            view.set_zoom(1.0, cx);
            view.prepare(room);
            view.set_zoom(2.0, cx);
            view.prepare(room);
            assert!(left(view) >= 0.0, "a set zoom keeps the left edge: {}", left(view));
            view.set_zoom(1.0, cx);
            view.prepare(room);
            let at = point(px(600.0), px(400.0));
            let before = view.doc_point(at);
            view.zoom_at(1.5, Some(at), cx);
            view.prepare(room);
            let after = view.doc_point(at);
            assert!((after.0 - before.0).abs() < 30 && (after.1 - before.1).abs() < 30, "{before:?} moved to {after:?}");
            // Wheel steps that land within one frame.
            view.set_zoom(1.0, cx);
            view.prepare(room);
            let before = view.doc_point(at);
            for zoom in [1.25, 1.5, 2.0] {
                view.zoom_at(zoom, Some(at), cx);
            }
            view.prepare(room);
            let after = view.doc_point(at);
            assert!((after.0 - before.0).abs() < 30 && (after.1 - before.1).abs() < 30, "bunched: {before:?} moved to {after:?}");
        })
    });
}
