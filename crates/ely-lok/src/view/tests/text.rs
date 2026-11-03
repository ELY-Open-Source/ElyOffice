use super::*;

#[test]
fn scrolled_text_and_hints_hide_but_frozen_text_stays_visible() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    cx.update(|app| {
        let (view, _) = live_view_of(
            app,
            &install,
            "book.fods",
            crate::client::tests::input::FODS,
        );
        view.update(app, |view, cx| {
            view.geom = Geom {
                zoom: 1.0,
                dpi: 1.0,
            };
            view.set_geometry(
                crate::sheet::geometry(
                    r#"{
                "columns":{"sizes":"1280:999","hidden":"0:999"},
                "rows":{"sizes":"256:1999","hidden":"0:1999"}
            }"#,
                )
                .unwrap(),
                cx,
            );
            for (anchor, frozen, scroll, visible) in [
                ((0, 0), (0, 0), (0, 0), true),
                ((0, 0), (0, 0), (4000, 0), false),
                ((0, 0), (1280, 0), (4000, 0), true),
                ((2560, 0), (1280, 0), (4000, 0), false),
                ((0, 0), (0, 0), (0, 4000), false),
                ((0, 0), (0, 256), (0, 4000), true),
                ((0, 2560), (0, 256), (0, 4000), false),
            ] {
                view.viewport = (0.0, 0.0);
                let cell = (anchor.0, anchor.1, 1280, 256);
                let batch = Events {
                    cursor: Some((317, 15, 0, 292)),
                    selection: vec![(402, 0, 157, 240)],
                    text: crate::events::TextAnchors {
                        cursor: Some(anchor),
                        selection: Some(anchor),
                    },
                    cell: crate::Cell {
                        cursor: Some(cell),
                        usage: "SUM(► Number 1) : Sum".into(),
                        ..crate::Cell::default()
                    },
                    ..Events::default()
                };
                view.apply_batch(batch, None, cx);
                assert!(
                    view.cell_text_selected(),
                    "formula text stays distinct from cell selection"
                );
                view.viewport = (800.0, 600.0);
                view.frozen = frozen;
                view.scroll = scroll;
                assert_eq!(view.is_visible(cell), visible);
                assert_eq!(
                    view.text_caret(view.seen()).is_some(),
                    visible,
                    "{anchor:?} {frozen:?}"
                );
                assert_eq!(view.cursor_bounds().is_some(), visible);
                assert_eq!(!view.text_selection().is_empty(), visible);
                assert_eq!(view.hints(cx).is_some(), visible);
            }
        });
    });
}

#[test]
fn formula_caret_paint_and_ime_follow_the_same_scrolled_cell() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view_of(
            app,
            &install,
            "book.fods",
            crate::client::tests::input::FODS,
        );
        view.update(app, |view, cx| {
            view.geom = Geom {
                zoom: 1.0,
                dpi: 1.0,
            };
            view.set_geometry(
                crate::sheet::geometry(
                    r#"{
                "columns":{"sizes":"1280:999","hidden":"0:999"},
                "rows":{"sizes":"256:1999","hidden":"0:1999"}
            }"#,
                )
                .unwrap(),
                cx,
            );
            view.origin = point(px(10.0), px(20.0));
            let batch = Events {
                cursor: Some((317, 15, 0, 292)),
                cursor_visible: true,
                selection: vec![(402, 0, 157, 292)],
                text: crate::events::TextAnchors {
                    cursor: Some((2560, 256000)),
                    selection: Some((2560, 256000)),
                },
                ..Events::default()
            };
            view.apply_batch(batch, None, cx);
            view.viewport = (800.0, 600.0);
            let cell = (2560, 256000, 1280, 256);
            for (zoom, dpi) in [(1.0, 1.0), (1.25, 2.0), (0.5, 1.0)] {
                view.geom = Geom { zoom, dpi };
                let (_, y, _, _) = view.tile_rect(cell);
                view.scroll = (0, y - 600);
                let caret = view.text_caret(view.seen()).unwrap();
                let ime = view.cursor_bounds().unwrap();
                let selected = view.text_selection()[0];
                let cell = view.screen_rect(cell);
                let painted = view.prepare(Bounds {
                    origin: point(px(10.0), px(20.0)),
                    size: size(px(800.0), px(600.0)),
                });
                assert_eq!(painted.caret, Some(caret));
                assert_eq!(painted.selection, [selected]);
                assert_eq!(caret.origin, ime.origin, "IME and painted caret agree");
                assert!((f32::from(caret.origin.y - cell.origin.y) - zoom).abs() < 0.01);
                assert!((f32::from(selected.origin.y - cell.origin.y)).abs() < 0.01);
                assert!((f32::from(caret.size.height) - 292.0 * zoom / 15.0).abs() < 0.01);
                assert!((f32::from(ime.origin.y) - (20.0 + 41.0 * zoom)).abs() < 0.01);
            }
            assert!(
                view.cursor().unwrap().1 > 255900,
                "public geometry stays in print twips"
            );
            assert!(view.selection()[0].1 > 255900);

            view.scroll = (0, 0);
            view.apply_batch(
                Events {
                    cursor: Some((20, 30, 0, 300)),
                    ..Events::default()
                },
                None,
                cx,
            );
            assert_eq!(view.text_caret(view.seen()), Some(view.screen_rect((20, 30, 0, 300))));
            assert!(
                view.text_selection().is_empty(),
                "a cleared selection loses its anchor"
            );
            assert!(!view.cell_text_selected());
        });
    });
}

#[test]
fn unchanged_formula_cursor_does_not_follow_zoom_or_geometry_changes() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view_of(
            app,
            &install,
            "book.fods",
            crate::client::tests::input::FODS,
        );
        view.update(app, |view, cx| {
            view.geom = Geom {
                zoom: 1.0,
                dpi: 1.0,
            };
            let geometry = |height| {
                crate::sheet::geometry(&format!(
                    r#"{{"columns":{{"sizes":"1280:999","hidden":"0:999"}},
                    "rows":{{"sizes":"{height}:1999","hidden":"0:1999"}}}}"#
                ))
                .unwrap()
            };
            view.set_geometry(geometry(256), cx);
            view.viewport = (0.0, 0.0);
            let batch = Events {
                cursor: Some((317, 15, 0, 292)),
                cursor_visible: true,
                text: crate::events::TextAnchors {
                    cursor: Some((2560, 256000)),
                    ..crate::events::TextAnchors::default()
                },
                ..Events::default()
            };
            view.apply_batch(batch.clone(), None, cx);
            view.viewport = (800.0, 600.0);
            view.scroll = (0, 0);
            let moves = view.moves();
            for (zoom, dpi) in [(0.5, 1.0), (1.25, 2.0), (1.0, 1.0)] {
                view.set_zoom(zoom, cx);
                view.keep_scroll(|view| view.geom.dpi = dpi);
                let scroll = view.scroll();
                view.apply_batch(batch.clone(), None, cx);
                assert_eq!(view.moves(), moves, "rescaling is not a cursor move");
                assert_eq!(
                    view.scroll(),
                    scroll,
                    "rescaling must not follow the cursor"
                );
            }
            view.set_geometry(geometry(257), cx);
            let scroll = view.scroll();
            view.apply_batch(batch.clone(), None, cx);
            assert_eq!(view.moves(), moves, "new row geometry is not a cursor move");
            assert_eq!(view.scroll(), scroll);

            let mut moved = batch;
            moved.cursor = Some((347, 15, 0, 292));
            view.apply_batch(moved, None, cx);
            assert_eq!(view.moves(), moves + 1);
            assert!(
                view.scroll().1 > 200000,
                "an actual move follows the cursor"
            );
        });
    });
}
