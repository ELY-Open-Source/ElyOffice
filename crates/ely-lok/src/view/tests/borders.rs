use super::*;
use crate::client::tests::{HANG, poll_until};
use gpui::{CursorStyle, MouseButton, MouseDownEvent, MouseUpEvent, TestAppContext};

const TABLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text>
 <table:table table:name="T"><table:table-column table:number-columns-repeated="2"/>
 <table:table-row><table:table-cell><text:p>left</text:p></table:table-cell><table:table-cell><text:p>right</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell><text:p>down</text:p></table:table-cell><table:table-cell><text:p>more</text:p></table:table-cell></table:table-row>
 </table:table><text:p>after</text:p></office:text></office:body>
</office:document>"#;

/// The view takes the cursor's table the kernel reports.
fn borders_read(view: &mut LokView) -> crate::table::Table {
    let doc = view.doc.clone();
    let mut worker = view.worker.lock().expect("the worker lock");
    worker.post_key(&doc, 0, 1027).expect("Right moves the cursor");
    let seen = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| seen.table.is_some());
    let table = seen.table.expect("the cursor's table");
    let now = worker.now(&doc).expect("the revision");
    drop(worker);
    view.doc = now;
    view.read_borders(Some(table.clone()), view.doc.rev);
    table
}

#[test]
fn a_dragged_border_moves_once_where_its_guide_shows() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = TestAppContext::single();
    let (view, _) = cx.update(|app| live_view_of(app, &install, "table.fodt", TABLE));
    let (line, moved) = cx.update(|app| {
        view.update(app, |view, cx| {
            let table = borders_read(view);
            let (columns, rows) = (&table.columns, &table.rows);
            let line = columns.offset + columns.inner[0].position;
            let middle = rows.offset + (rows.left + rows.right) / 2;
            let on = view.window_point((line, middle));
            view.move_border(on);
            assert_eq!(view.border_pointer(), Some(CursorStyle::ResizeColumn), "a hovered line asks to resize");
            assert!(view.hold_border(on), "the line is held");
            assert!(view.drop_border(on, cx), "let go in place");
            assert!(view.hold_border(on));
            let end = on + gpui::point(px(40.0), px(3.0));
            assert!(view.move_border(end));
            let guide = view.guide().expect("a guide line");
            let to = view.doc_point(guide.origin).0;
            assert!(to > line, "the guide follows the pointer: {line} to {to}");
            assert!(view.drop_border(end, cx), "the drop runs");
            assert!(!view.holds_border());
            (line, to - line)
        })
    });
    for _ in 0..80 {
        cx.executor().tick();
    }
    cx.update(|app| {
        let view = view.read(app);
        let mut worker = view.worker.lock().expect("the worker lock");
        let seen = poll_until(&mut worker, &view.doc, &Events::default(), HANG, |seen| {
            seen.table.as_ref().is_some_and(|table| table.columns.offset + table.columns.inner[0].position != line)
        });
        let table = seen.table.expect("still in the table");
        let landed = table.columns.offset + table.columns.inner[0].position;
        assert!((landed - line - moved).abs() <= 1, "the kernel lands it at the guide: {line} + {moved} vs {landed}");
    });
}

#[test]
fn a_held_border_keeps_its_table_and_a_newer_read_refuses_it() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = TestAppContext::single();
    let (view, _) = cx.update(|app| live_view_of(app, &install, "table.fodt", TABLE));
    let errors = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let line = cx.update(|app| {
        let errors = errors.clone();
        app.subscribe(&view, move |_, error: &LokError, _| errors.borrow_mut().push(error.0.clone())).detach();
        view.update(app, |view, cx| {
            let table = borders_read(view);
            let (columns, rows) = (&table.columns, &table.rows);
            let line = columns.offset + columns.inner[0].position;
            let on = view.window_point((line, rows.offset + (rows.left + rows.right) / 2));
            assert!(view.hold_border(on), "the line is held");
            let doc = view.doc.clone();
            let mut worker = view.worker.lock().expect("the worker lock");
            worker.post_key(&doc, 'a' as u32, 512).expect("a types");
            view.doc = worker.now(&doc).expect("the revision");
            drop(worker);
            let mut gone = table.clone();
            gone.columns.inner.clear();
            view.read_borders(Some(gone), view.doc.rev);
            let end = on + gpui::point(px(40.0), px(0.0));
            assert!(view.move_border(end), "it moves by its own table");
            assert!(view.guide().is_some(), "its guide still shows");
            assert!(view.drop_border(end, cx), "the drop runs");
            line
        })
    });
    let deadline = std::time::Instant::now() + HANG;
    while std::time::Instant::now() < deadline {
        for _ in 0..20 {
            cx.executor().tick();
        }
        if !errors.borrow().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(errors.borrow().iter().any(|error| error.contains("stale")), "{:?}", errors.borrow());
    cx.update(|app| {
        let view = view.read(app);
        let mut worker = view.worker.lock().expect("the worker lock");
        worker.post_key(&view.doc, 0, 1027).expect("Right moves the cursor");
        let seen = poll_until(&mut worker, &view.doc, &Events::default(), HANG, |seen| seen.table.is_some());
        let table = seen.table.expect("still in the table");
        assert_eq!(table.columns.offset + table.columns.inner[0].position, line, "the old drag never lands");
    });
}

#[test]
fn a_click_in_a_short_row_reaches_the_document() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = TestAppContext::single();
    let (view, _) = cx.update(|app| live_view_of(app, &install, "table.fodt", TABLE));
    let window = cx.add_window(|_, _| super::input::Blank);
    cx.update_window(window.into(), |_, window, app| {
        view.update(app, |view, cx| {
            view.set_zoom(0.5, cx);
            let table = borders_read(view);
            let (columns, rows) = (&table.columns, &table.rows);
            let x = columns.offset + (columns.left + columns.inner[0].position) / 2;
            let middle = view.window_point((x, rows.offset + (rows.inner[0].position + rows.right) / 2));
            let line = view.window_point((x, rows.offset + rows.inner[0].position));
            assert!(f32::from(line.y - middle.y).abs() < 6.0, "the row is shorter than two reaches");
            let down = |position| MouseDownEvent {
                button: MouseButton::Left,
                position,
                modifiers: Default::default(),
                click_count: 1,
                first_mouse: false,
            };
            let up = MouseUpEvent { button: MouseButton::Left, position: middle, modifiers: Default::default(), click_count: 1 };
            view.on_mouse_down(&down(middle), window, cx);
            assert!(!view.holds_border(), "the row's middle goes to the document");
            view.on_mouse_up(&up, window, cx);
            view.on_mouse_down(&down(line), window, cx);
            assert!(view.holds_border(), "the line itself is held");
            assert!(view.drop_border(line, cx));
        })
    })
    .expect("the window lives");
}

/// The batch whose table has `count` inner columns.
fn table_batch(view: &LokView, count: usize) -> Events {
    let mut worker = view.worker.lock().expect("the worker lock");
    let seen = poll_until(&mut worker, &view.doc, &Events::default(), HANG, |seen| {
        seen.table.as_ref().is_some_and(|table| table.columns.inner.len() == count)
    });
    assert_eq!(seen.table.as_ref().map(|table| table.columns.inner.len()), Some(count));
    seen
}

#[test]
fn a_late_batch_keeps_its_own_revision() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = TestAppContext::single();
    let (view, _) = cx.update(|app| live_view_of(app, &install, "table.fodt", TABLE));
    let errors = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let before = cx.update(|app| {
        let errors = errors.clone();
        app.subscribe(&view, move |_, error: &LokError, _| errors.borrow_mut().push(error.0.clone())).detach();
        view.update(app, |view, cx| {
            borders_read(view);
            let old = table_batch(view, 1);
            let table = old.table.clone().expect("the old table");
            let now = {
                let mut worker = view.worker.lock().expect("the worker lock");
                worker.run(&view.doc, "table.column_right", &[]).expect("a column goes in");
                worker.now(&view.doc).expect("the revision")
            };
            let before = table_batch(view, 2).table.expect("the new table").columns.inner[0].position;
            view.seen_revision(now.rev);
            view.apply_batch(old, None, cx);
            let on = view.window_point((table.columns.offset + table.columns.inner[0].position, table.rows.offset + 100));
            assert!(view.hold_border(on), "the old table's line is held");
            let end = on + gpui::point(px(20.0), px(0.0));
            assert!(view.drop_border(end, cx), "the drop runs");
            before
        })
    });
    let deadline = std::time::Instant::now() + HANG;
    while std::time::Instant::now() < deadline {
        for _ in 0..20 {
            cx.executor().tick();
        }
        if !errors.borrow().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(errors.borrow().iter().any(|error| error.contains("stale")), "{:?}", errors.borrow());
    let after = cx.update(|app| view.update(app, |view, _| table_batch(view, 2).table.expect("the table").columns.inner[0].position));
    assert_eq!(after, before, "the old read never moves the new table");
}

/// The real view, narrower than its window.
struct Narrow(gpui::Entity<LokView>);

impl gpui::Render for Narrow {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{ParentElement, Styled};
        gpui::div().w(px(450.0)).h(px(300.0)).child(self.0.clone())
    }
}

#[test]
fn a_border_let_go_outside_the_view_lands_there() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let (view, _) = cx.update(|app| live_view_of(app, &install, "table.fodt", TABLE));
    let (_, vcx) = cx.add_window_view(|_, _| Narrow(view.clone()));
    vcx.run_until_parked();
    let (line, on) = vcx.update(|_, app| {
        view.update(app, |view, cx| {
            let table = borders_read(view);
            cx.notify();
            let line = table.columns.offset + table.columns.inner[0].position;
            (line, view.window_point((line, table.rows.offset + 100)))
        })
    });
    vcx.run_until_parked();
    assert!(on.x > px(380.0) && on.x < px(440.0), "the line sits near the view's edge: {on:?}");
    vcx.simulate_mouse_down(on, MouseButton::Left, Default::default());
    assert!(vcx.update(|_, app| view.read(app).holds_border()), "the line is held");
    let inside = gpui::point(px(445.0), on.y);
    vcx.simulate_mouse_move(inside, MouseButton::Left, Default::default());
    let at = |vcx: &mut gpui::VisualTestContext| vcx.update(|_, app| view.read(app).guide().expect("a guide").origin.x);
    let held = at(vcx);
    let outside = gpui::point(px(490.0), on.y);
    vcx.simulate_mouse_move(outside, MouseButton::Left, Default::default());
    let out = at(vcx);
    assert!(out > held, "the guide follows out of the view: {held:?} to {out:?}");
    let wanted = vcx.update(|_, app| view.read(app).doc_point(gpui::point(out, on.y)).0);
    vcx.simulate_mouse_up(outside, MouseButton::Left, Default::default());
    assert!(!vcx.update(|_, app| view.read(app).holds_border()), "let go");
    vcx.run_until_parked();
    let landed = vcx.update(|_, app| {
        let view = view.read(app);
        let mut worker = view.worker.lock().expect("the worker lock");
        let seen = poll_until(&mut worker, &view.doc, &Events::default(), HANG, |seen| {
            seen.table.as_ref().is_some_and(|table| table.columns.offset + table.columns.inner[0].position != line)
        });
        let table = seen.table.expect("still in the table");
        table.columns.offset + table.columns.inner[0].position
    });
    assert!((landed - wanted).abs() <= 1, "it lands at the release: {landed} vs {wanted}");
}

#[test]
fn a_still_click_beside_a_border_sends_nothing() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let (view, _) = cx.update(|app| live_view_of(app, &install, "table.fodt", TABLE));
    let (_, vcx) = cx.add_window_view(|_, _| Narrow(view.clone()));
    vcx.run_until_parked();
    let on = vcx.update(|_, app| {
        view.update(app, |view, cx| {
            let table = borders_read(view);
            cx.notify();
            let line = table.columns.offset + table.columns.inner[0].position;
            view.window_point((line, table.rows.offset + 100)) + gpui::point(px(3.0), px(0.0))
        })
    });
    vcx.run_until_parked();
    let sent = vcx.update(|_, app| view.read(app).sent());
    vcx.simulate_mouse_down(on, MouseButton::Left, Default::default());
    assert!(vcx.update(|_, app| view.read(app).holds_border()), "the line is held off its centre");
    vcx.simulate_mouse_up(on, MouseButton::Left, Default::default());
    assert!(!vcx.update(|_, app| view.read(app).holds_border()), "let go");
    assert_eq!(vcx.update(|_, app| view.read(app).sent()), sent, "a still click sends no command");
}
