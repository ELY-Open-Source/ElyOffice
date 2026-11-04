use super::*;

/// A sheet whose last value sits 100,000 rows down.
const TALL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S">
 <table:table-row><table:table-cell><text:p>top</text:p></table:table-cell></table:table-row>
 <table:table-row table:number-rows-repeated="99998"><table:table-cell/></table:table-row>
 <table:table-row><table:table-cell><text:p>bottom</text:p></table:table-cell></table:table-row>
 </table:table></office:spreadsheet></office:body>
</office:document>"#;

#[test]
fn a_tall_sheet_paints_and_keeps_only_tiles_near_the_screen() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view_of(app, &install, "tall.fods", TALL);
        view.update(app, |view, cx| {
            let bounds = Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(800.0), px(600.0)),
            };
            let side = view.geom.tile_pixels() as usize;
            let painted = |generation| {
                Ok(crate::Tile {
                    generation,
                    pixels: vec![0u8; side * side * 4],
                })
            };
            // 800 by 600 needs 5 by 4 tiles.
            view.prepare(bounds);
            let top = view.on_screen.clone();
            assert!(!top.is_empty() && top.len() <= 20, "{} tiles", top.len());
            for key in &top {
                view.finish_tile(*key, view.geom, painted(view.generation), cx);
            }
            assert_eq!(view.tile_count(), top.len(), "the top paints");
            view.scroll_by(0.0, -3.0e6, cx);
            view.prepare(bounds);
            let bottom = view.on_screen.clone();
            assert!(bottom.len() <= 20, "{} tiles at the bottom", bottom.len());
            assert!(
                bottom.iter().all(|key| key.1 > 1000),
                "far down: {bottom:?}"
            );
            assert!(
                top.iter().all(|key| !view.tiles.contains_key(key)),
                "the top's tiles are gone"
            );
            assert!(
                view.fetching.len() <= 20 + top.len(),
                "only shown tiles fetch"
            );
        });
    });
}

/// 6000 rows of 0.5 cm; tiles round each to pixels.
const SHORT_ROWS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:automatic-styles><style:style style:name="ro1" style:family="table-row"><style:table-row-properties style:row-height="0.5cm" style:use-optimal-row-height="false"/></style:style></office:automatic-styles>
 <office:body><office:spreadsheet><table:table table:name="S">
 <table:table-row table:style-name="ro1" table:number-rows-repeated="6000"><table:table-cell table:number-columns-repeated="4"/></table:table-row>
 </table:table></office:spreadsheet></office:body>
</office:document>"#;

/// A short-row sheet at 2× DPI, scrolled to `cell`'s row.
pub(super) fn far_down(cx: &mut gpui::TestAppContext, install: &std::path::Path) -> (Entity<LokView>, crate::sheet::Geometry) {
    let (view, worker) = cx.update(|app| live_view_of(app, install, "short.fods", SHORT_ROWS));
    let json = cx.update(|app| {
        let read = view.read(app);
        worker.lock().expect("the worker lock").values(&read.doc, crate::sheet::GEOMETRY).expect("geometry")
    });
    let geometry = crate::sheet::geometry(&json).expect("parsed");
    let row = cell(&geometry, 1, 4997);
    let bounds = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(800.0), px(600.0)) };
    cx.update(|app| {
        view.update(app, |view, cx| {
            view.geom.dpi = 2.0;
            view.set_geometry(geometry.clone(), cx);
            view.prepare(bounds);
        })
    });
    // The first scale's report and size land before the move.
    for _ in 0..100 {
        cx.executor().tick();
    }
    cx.update(|app| view.update(app, |view, cx| {
        view.show(row, cx);
        view.prepare(bounds);
    }));
    (view, geometry)
}

/// Column `x`, row `y` in print twips.
pub(super) fn cell(geometry: &crate::sheet::Geometry, x: u32, y: u32) -> crate::Rect {
    (geometry.columns.start(x), geometry.rows.start(y), geometry.columns.size(x), geometry.rows.size(y))
}

/// Waits until the kernel reports `want` as the selection.
pub(super) fn selected(view: &Entity<LokView>, cx: &mut gpui::TestAppContext, want: &str) -> String {
    let mut address = String::new();
    let deadline = std::time::Instant::now() + crate::client::tests::HANG;
    while address != want && std::time::Instant::now() < deadline {
        cx.executor().tick();
        address = cx.update(|app| view.read(app).cell().address.clone());
    }
    address
}

#[test]
fn a_click_far_down_a_sheet_selects_the_cell_under_it() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = gpui::TestAppContext::single();
    let (view, geometry) = far_down(&mut cx, &install);
    let (x, y, w, h) = cell(&geometry, 1, 4997);
    cx.update(|app| {
        view.update(app, |view, _| {
            let at = view.window_point((x + w / 2, y + h / 2));
            for kind in [0, 1] {
                view.mouse(kind, at, 1, 1, &gpui::Modifiers::default());
            }
        })
    });
    assert_eq!(selected(&view, &mut cx, "B4998"), "B4998");
}

#[test]
fn a_drag_far_down_a_sheet_selects_the_range_under_it() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = gpui::TestAppContext::single();
    let (view, geometry) = far_down(&mut cx, &install);
    let middle = |(x, y, w, h): crate::Rect| (x + w / 2, y + h / 2);
    let (from, to) = (middle(cell(&geometry, 1, 4997)), middle(cell(&geometry, 3, 5000)));
    cx.update(|app| {
        view.update(app, |view, _| {
            let modifiers = gpui::Modifiers::default();
            view.mouse(0, view.window_point(from), 1, 1, &modifiers);
            view.mouse(2, view.window_point(((from.0 + to.0) / 2, (from.1 + to.1) / 2)), 1, 1, &modifiers);
            view.mouse(2, view.window_point(to), 1, 1, &modifiers);
            view.mouse(1, view.window_point(to), 1, 1, &modifiers);
        })
    });
    assert_eq!(selected(&view, &mut cx, "B4998:D5001"), "B4998:D5001");
}

#[test]
fn zooming_far_down_a_sheet_keeps_the_top_row() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = gpui::TestAppContext::single();
    let (view, geometry) = far_down(&mut cx, &install);
    let top = |cx: &mut gpui::TestAppContext| {
        cx.update(|app| {
            let read = view.read(app);
            let at = read.origin() + point(px(read.viewport().0 / 2.0), px(1.0));
            geometry.rows.at(read.doc_point(at).1)
        })
    };
    let before = top(&mut cx);
    cx.update(|app| view.update(app, |view, cx| view.set_zoom(0.8, cx)));
    let after = top(&mut cx);
    assert_eq!(before, after, "the top row moved");
}

#[test]
fn a_sheet_brings_far_cut_and_frozen_cells_into_view() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, worker) = live_view_of(app, &install, "short.fods", SHORT_ROWS);
        let json = worker.lock().expect("the worker lock").values(&view.read(app).doc, crate::sheet::GEOMETRY).expect("geometry");
        view.update(app, |view, cx| {
            let wide = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(6000.0), px(200.0)) };
            view.prepare(wide);
            assert_eq!(view.spare, (0.0, 0.0), "before its geometry a sheet starts left");
            view.set_geometry(crate::sheet::geometry(&json).expect("parsed"), cx);
            view.prepare(wide);
            assert_eq!(view.spare, (0.0, 0.0), "and stays left after it");
            let size = view.doc_size;
            view.doc_size = view.tile_of((view.print_of(size).0 + 20 * 1280, size.1));
            let extent = view.extent();
            assert!(extent.0 >= view.doc_size.0 && extent.1 >= view.doc_size.1, "the kernel's size is uncut");
            view.doc_size = size;
            view.viewport = (800.0, 200.0);
            let (width, height) = (view.geom.twips_of(view.viewport.0), view.geom.twips_of(view.viewport.1));
            let shows = |view: &LokView, rect: crate::Rect| {
                let (x, y, w, h) = view.tile_rect(rect);
                let (fx, fy) = view.frozen_tiles();
                x + w <= fx + view.scroll.0 + width && y >= fy + view.scroll.1 && y + h <= fy + view.scroll.1 + height
            };
            let far = (1418, view.doc_size.1 + 20_000, 1280, 277);
            let cell = crate::Cell { cursor: Some(far), ..crate::Cell::default() };
            view.apply_batch(crate::Events { cell, ..crate::Events::default() }, None, cx);
            assert!(shows(view, far), "a cell past the kernel's size still comes into view");
            view.scroll = (0, 0);
            let cut = (width - 200, 0, 1280, 256);
            view.show(cut, cx);
            assert!(view.scroll.0 > 0 && shows(view, cut), "a cell cut at the right edge comes into view");
            let (x, _, w, _) = view.tile_rect(cut);
            assert_eq!(view.scroll.0, x + w - width, "it scrolls no further than the cell's edge");
            let before = view.scroll;
            view.show((0, 0, view.print_of((x + w + width, 0)).0, 256), cx);
            assert_eq!(view.scroll, before, "a cell wider than the view and covering it keeps still");
            view.dragging = true;
            let cell = crate::Cell { cursor: Some((1418, 90_000, 1280, 277)), ..crate::Cell::default() };
            view.apply_batch(crate::Events { cell, ..crate::Events::default() }, None, cx);
            assert_eq!(view.scroll, before, "a drag does not scroll under the pointer");
            view.dragging = false;
            view.scroll = (0, 0);
            view.set_frozen((0, 600), cx);
            let frozen = view.place((0, 300)).y;
            view.show((1418, 300, 1280, 277), cx);
            assert_eq!(view.scroll.1, 0, "a frozen row needs no scroll");
            view.show((1418, 9000, 1280, 277), cx);
            assert!(view.scroll.1 > 0, "a far row scrolls the rest");
            assert_eq!(view.place((0, 300)).y, frozen, "the frozen row stays");
            let back = view.twips_at(view.place((0, 9000)));
            assert!((back.1 - 9000).abs() <= 15, "a scrolled row maps back: {back:?}");
        });
    });
}

#[test]
fn a_newly_selected_object_comes_into_view_a_moved_one_stays() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view_of(app, &install, "tall.fods", TALL);
        view.update(app, |view, cx| {
            let bounds = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(800.0), px(600.0)) };
            view.prepare(bounds);
            let far = (60_000, 40_000, 9_000, 5_000);
            let chart = |graphic| crate::Events { graphic, ..crate::Events::default() };
            view.apply_batch(chart(Some(far)), None, cx);
            view.prepare(bounds);
            let (x, y) = (view.scroll.0, view.scroll.1);
            assert!(x > 0 && y > 0, "the new chart is shown: scroll {x},{y}");
            view.apply_batch(chart(Some((61_000, 40_000, 9_000, 5_000))), None, cx);
            assert_eq!((view.scroll.0, view.scroll.1), (x, y), "dragging it does not scroll");
        });
    });
}

/// Data out to the last cell sets the size.
pub(super) const FULL_SHEET: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:automatic-styles><style:style style:name="co1" style:family="table-column"><style:table-column-properties style:column-width="0.4cm"/></style:style><style:style style:name="ro1" style:family="table-row"><style:table-row-properties style:row-height="0.5cm" style:use-optimal-row-height="false"/></style:style></office:automatic-styles>
 <office:body><office:spreadsheet><table:table table:name="S">
 <table:table-column table:style-name="co1" table:number-columns-repeated="2048"/>
 <table:table-row table:style-name="ro1" table:number-rows-repeated="5999"><table:table-cell table:number-columns-repeated="2048"/></table:table-row>
 <table:table-row table:style-name="ro1"><table:table-cell table:number-columns-repeated="2047"/><table:table-cell office:value-type="float" office:value="1"><text:p>1</text:p></table:table-cell></table:table-row>
 </table:table></office:spreadsheet></office:body>
</office:document>"#;

#[test]
fn a_zoom_measures_the_sheet_again() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = gpui::TestAppContext::single();
    let (view, _) = cx.update(|app| live_view_of(app, &install, "full.fods", FULL_SHEET));
    let bounds = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(400.0), px(200.0)) };
    cx.update(|app| view.update(app, |view, _| view.prepare(bounds)));
    for _ in 0..100 {
        cx.executor().tick();
    }
    let before = cx.update(|app| view.read(app).doc_size);
    cx.update(|app| view.update(app, |view, cx| {
        view.set_zoom(1.25, cx);
        view.prepare(bounds);
    }));
    // The kernel's size at the new zoom, read fresh.
    let kernel = |cx: &mut gpui::TestAppContext| {
        cx.update(|app| {
            let read = view.read(app);
            let (w, h) = read.doc.size(&mut read.worker.lock().expect("the worker lock")).expect("a size");
            (w as i32, h as i32)
        })
    };
    // The kernel takes the zoom, then the view its size.
    let (mut seen, mut now) = (before, before);
    let deadline = std::time::Instant::now() + crate::client::tests::HANG;
    while std::time::Instant::now() < deadline {
        cx.executor().tick();
        (seen, now) = (cx.update(|app| view.read(app).doc_size), kernel(&mut cx));
        if now != before && seen == now {
            break;
        }
    }
    assert_ne!(now, before, "the zoom changes the kernel's size here");
    assert_eq!(seen, now, "the view takes the size the kernel has at the new zoom");
}

#[test]
fn a_zoom_at_the_end_of_a_sheet_keeps_the_top_row() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = gpui::TestAppContext::single();
    let (view, worker) = cx.update(|app| live_view_of(app, &install, "full.fods", FULL_SHEET));
    let json = cx.update(|app| worker.lock().expect("the worker lock").values(&view.read(app).doc, crate::sheet::GEOMETRY).expect("geometry"));
    let geometry = crate::sheet::geometry(&json).expect("parsed");
    let bounds = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(400.0), px(200.0)) };
    cx.update(|app| view.update(app, |view, cx| {
        view.set_geometry(geometry.clone(), cx);
        view.prepare(bounds);
    }));
    for _ in 0..100 {
        cx.executor().tick();
    }
    // Scrolled to the end, as by the wheel.
    cx.update(|app| view.update(app, |view, cx| {
        view.scroll_by(0.0, -1.0e7, cx);
        view.prepare(bounds);
    }));
    for _ in 0..100 {
        cx.executor().tick();
    }
    let top = |cx: &mut gpui::TestAppContext| {
        cx.update(|app| {
            let read = view.read(app);
            let at = read.origin() + point(px(200.0), px(1.0));
            geometry.rows.at(read.doc_point(at).1)
        })
    };
    let before = top(&mut cx);
    cx.update(|app| view.update(app, |view, cx| {
        view.set_zoom(1.25, cx);
        view.prepare(bounds);
    }));
    let right_after = top(&mut cx);
    for _ in 0..400 {
        cx.executor().tick();
    }
    let measured = top(&mut cx);
    assert_eq!(before, right_after, "the top row moved at the zoom");
    assert_eq!(before, measured, "the top row moved once measured");
}

#[test]
fn a_late_size_from_the_last_zoom_keeps_the_row() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = gpui::TestAppContext::single();
    let (view, worker) = cx.update(|app| live_view_of(app, &install, "full.fods", FULL_SHEET));
    let json = cx.update(|app| worker.lock().expect("the worker lock").values(&view.read(app).doc, crate::sheet::GEOMETRY).expect("geometry"));
    let geometry = crate::sheet::geometry(&json).expect("parsed");
    let bounds = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(400.0), px(200.0)) };
    let settle = |cx: &mut gpui::TestAppContext| {
        for _ in 0..200 {
            cx.executor().tick();
        }
    };
    cx.update(|app| view.update(app, |view, cx| {
        view.set_geometry(geometry.clone(), cx);
        view.set_zoom(1.25, cx);
        view.prepare(bounds);
    }));
    settle(&mut cx);
    cx.update(|app| view.update(app, |view, cx| {
        view.scroll_by(0.0, -1.0e7, cx);
        view.prepare(bounds);
    }));
    settle(&mut cx);
    // Measured at 125%, landing after the zoom to 150%.
    let late = cx.update(|app| super::super::measure(&view.read(app).worker, &view.read(app).doc));
    let middle = |cx: &mut gpui::TestAppContext| {
        cx.update(|app| {
            let read = view.read(app);
            geometry.rows.at(read.doc_point(read.origin() + point(px(200.0), px(100.0))).1)
        })
    };
    cx.update(|app| view.update(app, |view, cx| view.set_zoom(1.5, cx)));
    let before = middle(&mut cx);
    cx.update(|app| view.update(app, |view, cx| view.apply_batch(crate::Events::default(), Some(late), cx)));
    let after = middle(&mut cx);
    assert!(before.abs_diff(after) <= 1, "row {before} became {after} when the old size landed");
}

#[test]
fn the_first_size_has_a_scale_and_keeps_the_row() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = gpui::TestAppContext::single();
    let (view, worker) = cx.update(|app| live_view_of(app, &install, "full.fods", FULL_SHEET));
    // Read before any visible area: the load set a scale.
    let first = cx.update(|app| super::super::measure(&view.read(app).worker, &view.read(app).doc));
    assert_eq!(first.as_ref().map(|(_, zoom)| *zoom).ok(), Some((256, 3840)), "a known scale");
    let json = cx.update(|app| worker.lock().expect("the worker lock").values(&view.read(app).doc, crate::sheet::GEOMETRY).expect("geometry"));
    let geometry = crate::sheet::geometry(&json).expect("parsed");
    let bounds = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(400.0), px(200.0)) };
    cx.update(|app| view.update(app, |view, cx| {
        view.set_geometry(geometry.clone(), cx);
        view.prepare(bounds);
        view.scroll_by(0.0, -1.0e7, cx);
    }));
    let middle = |cx: &mut gpui::TestAppContext| {
        cx.update(|app| {
            let read = view.read(app);
            geometry.rows.at(read.doc_point(read.origin() + point(px(200.0), px(100.0))).1)
        })
    };
    cx.update(|app| view.update(app, |view, cx| view.set_zoom(1.25, cx)));
    let before = middle(&mut cx);
    cx.update(|app| view.update(app, |view, cx| view.apply_batch(crate::Events::default(), Some(first), cx)));
    let after = middle(&mut cx);
    assert!(before.abs_diff(after) <= 1, "row {before} became {after} when the first size landed");
}

#[test]
fn an_empty_sheet_scrolls_on_without_waiting() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    let (view, worker) = cx.update(|app| live_view_of(app, &install, "short.fods", SHORT_ROWS));
    let json = cx.update(|app| worker.lock().expect("the worker lock").values(&view.read(app).doc, crate::sheet::GEOMETRY).expect("geometry"));
    let geometry = crate::sheet::geometry(&json).expect("parsed");
    let bounds = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(400.0), px(200.0)) };
    cx.update(|app| view.update(app, |view, cx| {
        view.set_geometry(geometry.clone(), cx);
        view.prepare(bounds);
    }));
    // Each step a viewport down, before the kernel answers.
    for step in 1..=20 {
        let moved = cx.update(|app| view.update(app, |view, cx| {
            let from = view.scroll.1;
            view.scroll_by(0.0, -200.0, cx);
            view.scroll.1 - from
        }));
        let full = cx.update(|app| view.read(app).geom.twips_of(200.0));
        assert_eq!(moved, full, "step {step} went the whole way");
    }
}
