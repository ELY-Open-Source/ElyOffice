use super::*;

/// Fits a huge page into an 800 by 600 view.
fn fitted_large(view: &mut LokView, cx: &mut gpui::Context<LokView>) {
    view.doc_size = (170_000, 170_000);
    view.fit_page(cx);
    view.prepare(Bounds { origin: point(px(0.0), px(0.0)), size: size(px(800.0), px(600.0)) });
}

#[test]
fn gestures_without_magnification_keep_the_fit() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view(app, &install);
        view.update(app, |view, cx| {
            fitted_large(view, cx);
            let (zoom, scroll) = (view.zoom(), view.scroll);
            let at = point(px(300.0), px(200.0));
            for phase in [gpui::TouchPhase::Started, gpui::TouchPhase::Ended] {
                let pinch = gpui::PinchEvent { position: at, delta: 0.0, modifiers: Default::default(), phase };
                view.pinch_zoom(&pinch, cx);
            }
            let sideways = gpui::ScrollEvent {
                position: at,
                delta: gpui::ScrollDelta::Pixels(point(px(30.0), px(0.0))),
                modifiers: gpui::Modifiers::secondary_key(cx),
                ..Default::default()
            };
            assert!(view.wheel_zoom(&sideways, cx), "a Ctrl scroll is taken");
            assert!(view.fitting(), "nothing magnified: still fitting");
            assert_eq!((view.zoom(), view.scroll), (zoom, scroll), "the page stays put");
        });
    });
}

/// The print-twips start of the scrolling pane.
fn start(view: &LokView) -> (i32, i32) {
    let (fx, fy) = view.frozen_tiles();
    view.print_of((fx + view.scroll.0, fy + view.scroll.1))
}

#[test]
fn a_sheet_zoom_keeps_the_top_left_cell_and_the_pointed_one() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, worker) = live_view_of(app, &install, "full.fods", super::large::FULL_SHEET);
        let json = worker.lock().expect("the worker lock").values(&view.read(app).doc, crate::sheet::GEOMETRY).expect("geometry");
        let geometry = crate::sheet::geometry(&json).expect("parsed");
        view.update(app, |view, cx| {
            view.set_geometry(geometry.clone(), cx);
            view.prepare(Bounds { origin: point(px(0.0), px(0.0)), size: size(px(400.0), px(200.0)) });
            // The control and the menu set; keys step.
            for zoom in [0.67, 1.25, 4.0, 0.25, 1.0] {
                view.set_zoom(zoom, cx);
                assert_eq!(view.scroll, (0, 0), "A1 left the top left at {zoom}");
            }
            for larger in [false, false, false, false, true, true, true, true, true, true, true, true, true] {
                view.zoom_step(larger, cx);
                assert_eq!(view.scroll, (0, 0), "A1 left the top left at {}", view.zoom());
            }
            // Ctrl and the wheel over A1 keep A1 whole there.
            for notch in [-1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0] {
                let wheel = gpui::ScrollEvent {
                    position: view.window_point((geometry.columns.start(1) / 2, geometry.rows.start(1) / 2)),
                    delta: gpui::ScrollDelta::Lines(point(0.0, notch)),
                    modifiers: gpui::Modifiers::secondary_key(cx),
                    ..Default::default()
                };
                assert!(view.wheel_zoom(&wheel, cx));
                assert_eq!(view.scroll, (0, 0), "A1 left the top left at {}", view.zoom());
            }
            let cell = |at: (i32, i32)| (geometry.columns.at(at.0), geometry.rows.at(at.1));
            let near = |a: (i32, i32), b: (i32, i32)| (a.0 - b.0).abs() <= 15 && (a.1 - b.1).abs() <= 15;
            // Scrolled, then frozen: the scrolling pane's first cell stays.
            for frozen in [(0, 0), (geometry.columns.start(2), geometry.rows.start(3))] {
                view.set_frozen(frozen, cx);
                view.set_zoom(1.0, cx);
                view.scroll = view.tile_of((geometry.columns.start(30) + 100, geometry.rows.start(200) + 50));
                let first = start(view);
                for larger in [false, false, false, false, true, true, true, true, true, true, true, true, true] {
                    view.zoom_step(larger, cx);
                    let now = start(view);
                    assert!(cell(now) == cell(first) && near(now, first), "{first:?} became {now:?} at {}", view.zoom());
                }
            }
            // Over frozen rows: top row and pointed column stay.
            view.set_zoom(1.0, cx);
            view.scroll = view.tile_of((geometry.columns.start(200), geometry.rows.start(200)));
            let (x, y) = (200.0, 5.0);
            let pointed = |view: &LokView| geometry.columns.at(view.print_of((view.scroll.0 + view.geom.twips_of(x), 0)).0);
            let edge = |view: &LokView, column| view.geom.px_of(view.tile_of((geometry.columns.start(column), 0)).0 - view.scroll.0);
            let first = start(view);
            for zoom in [0.25, 0.67, 1.25, 4.0] {
                let column = pointed(view);
                let left = edge(view, column);
                view.zoom_at(zoom, Some(point(px(x), px(y))), cx);
                let (now, moved) = (start(view), edge(view, column) - left);
                assert!((now.1 - first.1).abs() <= 15, "the pane's top row moved: {first:?} became {now:?} at {zoom}");
                assert!(moved.abs() <= 1.0, "column {column}'s edge moved {moved} px at {zoom}");
            }
        });
    });
}

#[test]
fn zooming_out_from_a_small_fit_never_enlarges() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view(app, &install);
        view.update(app, |view, cx| {
            fitted_large(view, cx);
            let fit = view.zoom();
            assert!(fit < 0.25, "the fit is under the least step: {fit}");
            view.zoom_step(false, cx);
            assert!(view.zoom() <= fit, "out went from {fit} to {}", view.zoom());
            view.zoom_step(true, cx);
            assert_eq!(view.zoom(), 0.25, "in reaches the least step");
        });
    });
}
