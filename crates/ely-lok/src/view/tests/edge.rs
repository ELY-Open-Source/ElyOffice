//! Dragging past the right edge scrolls and grows (#15).

use std::time::Duration;

use super::*;

/// A sheet with a value far to the right.
const WIDE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S">
 <table:table-row><table:table-cell><text:p>a</text:p></table:table-cell><table:table-cell table:number-columns-repeated="60"/><table:table-cell><text:p>far</text:p></table:table-cell></table:table-row>
 </table:table></office:spreadsheet></office:body>
</office:document>"#;

#[test]
fn a_drag_held_past_the_edge_scrolls_and_grows_the_range() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    let (view, worker) = cx.update(|app| live_view_of(app, &install, "wide.fods", WIDE));
    let json = cx.update(|app| {
        let read = view.read(app);
        let mut worker = worker.lock().expect("the worker lock");
        worker.values(&read.doc, crate::sheet::GEOMETRY).expect("geometry")
    });
    let geometry = crate::sheet::geometry(&json).expect("parsed");
    let bounds = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(800.0), px(600.0)) };
    cx.update(|app| {
        view.update(app, |view, cx| {
            view.set_geometry(geometry.clone(), cx);
            view.prepare(bounds);
        })
    });
    for _ in 0..100 {
        cx.executor().tick();
    }
    let (edge, modifiers) = (point(px(799.0), px(10.0)), gpui::Modifiers::default());
    let last = cx.update(|app| geometry.columns.at(view.read(app).doc_point(edge).0));
    cx.update(|app| {
        view.update(app, |view, cx| {
            view.mouse(0, point(px(20.0), px(10.0)), 1, 1, &modifiers);
            view.dragging = true;
            view.drag_to(point(px(900.0), px(10.0)), modifiers, cx);
        })
    });
    for _ in 0..20 {
        cx.executor().advance_clock(Duration::from_millis(40));
        cx.executor().tick();
        cx.update(|app| view.update(app, |view, _| _ = view.prepare(bounds)));
    }
    let scrolled = cx.update(|app| view.read(app).scroll().0);
    assert!(scrolled > 0, "the view scrolled right");
    cx.update(|app| {
        view.update(app, |view, _| {
            view.mouse(1, edge, 1, 1, &modifiers);
            view.end_drag();
        })
    });
    let mut address = String::new();
    let deadline = std::time::Instant::now() + crate::client::tests::HANG;
    while !address.contains(':') && std::time::Instant::now() < deadline {
        cx.executor().tick();
        address = cx.update(|app| view.read(app).cell().address.clone());
    }
    let end = address.split(':').nth(1).expect("a range").trim_end_matches(char::is_numeric);
    let end = (end.bytes().fold(0, |at, b| at * 26 + u32::from(b - b'A' + 1))) - 1;
    assert!(end > last, "{address} ends past column {last}");
}

#[test]
fn the_wheel_scrolls_past_the_used_columns() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    let (view, worker) = cx.update(|app| live_view_of(app, &install, "wide.fods", WIDE));
    let json = cx.update(|app| {
        let read = view.read(app);
        let mut worker = worker.lock().expect("the worker lock");
        worker.values(&read.doc, crate::sheet::GEOMETRY).expect("geometry")
    });
    let geometry = crate::sheet::geometry(&json).expect("parsed");
    let bounds = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(800.0), px(600.0)) };
    cx.update(|app| {
        view.update(app, |view, cx| {
            view.set_geometry(geometry.clone(), cx);
            view.prepare(bounds);
        })
    });
    for _ in 0..100 {
        cx.executor().tick();
    }
    for _ in 0..200 {
        cx.update(|app| {
            view.update(app, |view, cx| {
                view.scroll_by(-400.0, 0.0, cx);
                view.prepare(bounds);
            })
        });
    }
    let column = cx.update(|app| geometry.columns.at(view.read(app).doc_point(point(px(0.0), px(10.0))).0));
    assert!(column > 200, "the left edge reached column {column}");
}

/// The view at 600 by 300.
struct Small(Entity<LokView>);

impl gpui::Render for Small {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{ParentElement, Styled};
        gpui::div().w(px(600.0)).h(px(300.0)).child(self.0.clone())
    }
}

#[test]
fn a_wide_sheet_has_a_bar_that_scrolls_it() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = gpui::TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let (view, worker) = cx.update(|app| live_view_of(app, &install, "wide.fods", WIDE));
    let json = cx.update(|app| {
        let read = view.read(app);
        let mut worker = worker.lock().expect("the worker lock");
        worker.values(&read.doc, crate::sheet::GEOMETRY).expect("geometry")
    });
    let geometry = crate::sheet::geometry(&json).expect("parsed");
    cx.update(|app| view.update(app, |view, cx| view.set_geometry(geometry, cx)));
    let (_, vcx) = cx.add_window_view(|_, _| Small(view.clone()));
    vcx.run_until_parked();
    // The selector finds the first thumb, the vertical one.
    let thumb = vcx.debug_bounds("scrollbar-thumb").expect("a vertical thumb");
    assert!(thumb.size.height > thumb.size.width, "the first thumb stands: {thumb:?}");
    // The bottom thumb starts left, atop a two-thickness track.
    let grip = point(px(20.0), px(300.0) - thumb.size.width * 1.5);
    vcx.simulate_mouse_move(grip, None, gpui::Modifiers::default());
    vcx.simulate_mouse_down(grip, gpui::MouseButton::Left, gpui::Modifiers::default());
    // The first move starts the drag, the next anchors it.
    for right in [10.0, 20.0, 220.0] {
        let to = grip + point(px(right), px(0.0));
        vcx.simulate_mouse_move(to, gpui::MouseButton::Left, gpui::Modifiers::default());
    }
    let end = grip + point(px(220.0), px(0.0));
    vcx.simulate_mouse_up(end, gpui::MouseButton::Left, gpui::Modifiers::default());
    vcx.run_until_parked();
    let scrolled = vcx.update(|_, app| view.read(app).scroll());
    assert!(scrolled.0 > 0 && scrolled.1 == 0, "the bar scrolled the sheet across: {scrolled:?}");
}
