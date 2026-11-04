use super::*;
use crate::client::tests::{HANG, poll_until, show::DECK};
use crate::table::Table;
use gpui::{Pixels, Point, TestAppContext};

/// A line's place along its axis, in twips.
type Place = fn(&Table) -> i32;

/// A new table on the slide, read into the view.
fn table_in(view: &mut LokView, (columns, rows): (&str, &str)) -> Table {
    let doc = view.doc.clone();
    let mut worker = view.worker.lock().expect("the worker lock");
    worker.run(&doc, "table.insert", &[columns, rows]).expect("a table");
    let seen = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| seen.table.is_some());
    assert!(seen.faults.is_empty(), "{:?}", seen.faults);
    let table = seen.table.expect("the selected table");
    view.doc = worker.now(&doc).expect("the revision");
    drop(worker);
    view.read_borders(Some(table.clone()), view.doc.rev);
    table
}

/// The view's table once `place` moves off `from`.
fn moved(view: &mut LokView, place: Place, from: i32) -> Table {
    let mut worker = view.worker.lock().expect("the worker lock");
    let seen = poll_until(&mut worker, &view.doc, &Events::default(), HANG, |seen| {
        seen.table.as_ref().is_some_and(|table| place(table) != from)
    });
    let table = seen.table.expect("still the table");
    view.doc = worker.now(&view.doc).expect("the revision");
    drop(worker);
    view.read_borders(Some(table.clone()), view.doc.rev);
    table
}

/// The last drag's place: before, guide, landed, undone.
fn drag(size: (&str, &str), place: Place, upright: bool, bys: &[Point<Pixels>]) -> Option<[i32; 4]> {
    let install = crate::client::tests::install()?;
    let cx = TestAppContext::single();
    let (view, _) = cx.update(|app| live_view_of(app, &install, "deck.fodp", DECK));
    let mut table = cx.update(|app| view.update(app, |view, _| table_in(view, size)));
    let mut last = [0; 3];
    for &by in bys {
        let (before, guide) = cx.update(|app| {
            view.update(app, |view, cx| {
                let at = place(&table);
                let across = if upright { &table.rows } else { &table.columns };
                // A quarter along: halfway may cross another line.
                let along = across.offset + across.left + (across.right - across.left) / 4;
                let on = view.window_point(if upright { (at, along) } else { (along, at) });
                assert!(view.hold_border(on), "the line is held");
                assert!(view.move_border(on + by));
                let guide = view.doc_point(view.guide().expect("a guide").origin);
                assert!(view.drop_border(on + by, cx), "the drop runs");
                (at, if upright { guide.0 } else { guide.1 })
            })
        });
        for _ in 0..80 {
            cx.executor().tick();
        }
        table = cx.update(|app| view.update(app, |view, _| moved(view, place, before)));
        last = [before, guide, place(&table)];
    }
    let undone = cx.update(|app| {
        view.update(app, |view, _| {
            view.worker.lock().expect("the worker lock").run(&view.doc, "edit.undo", &[]).expect("undoes");
            place(&moved(view, place, last[2]))
        })
    });
    Some([last[0], last[1], last[2], undone])
}

/// Lands at its guide; one undo puts it back.
#[track_caller]
fn lands([before, guide, landed, undone]: [i32; 4]) {
    assert!((landed - guide).abs() <= 2, "{before} to the guide {guide}, landed {landed}");
    assert!((undone - before).abs() <= 1, "one undo: {undone} vs {before}");
}

#[test]
fn an_inner_line_lands_at_its_guide() {
    let place: Place = |t| t.columns.offset + t.columns.inner[0].position;
    let Some(moved) = drag(("2", "2"), place, true, &[gpui::point(px(40.0), px(0.0))]) else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    assert!(moved[1] > moved[0], "the guide went right");
    lands(moved);
}

#[test]
fn the_left_edge_narrows_the_first_column_and_moves_the_table() {
    let place: Place = |t| t.columns.offset + t.columns.left;
    let Some(moved) = drag(("2", "2"), place, true, &[gpui::point(px(40.0), px(0.0))]) else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    assert!(moved[1] > moved[0], "the guide went right");
    lands(moved);
}

#[test]
fn a_one_row_bottom_grows() {
    let place: Place = |t| t.rows.offset + t.rows.right;
    let Some(moved) = drag(("2", "1"), place, false, &[gpui::point(px(0.0), px(40.0))]) else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    assert!(moved[1] > moved[0], "the guide went down");
    lands(moved);
}

#[test]
fn a_grown_bottom_stops_at_the_last_rows_minimum() {
    let place: Place = |t| t.rows.offset + t.rows.right;
    let Some(moved) = drag(("2", "2"), place, false, &[gpui::point(px(0.0), px(60.0)), gpui::point(px(0.0), px(-200.0))]) else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    assert!(moved[1] < moved[0] && moved[1] > moved[0] - 3000, "the guide stops at the minimum: {moved:?}");
    lands(moved);
}

#[test]
fn a_one_column_right_edge_stops_at_the_columns_minimum() {
    let place: Place = |t| t.columns.offset + t.columns.right;
    let Some(moved) = drag(("1", "2"), place, true, &[gpui::point(px(-600.0), px(0.0))]) else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    assert!(moved[1] > moved[0] - 9000, "the guide stops at the minimum: {moved:?}");
    lands(moved);
}

#[test]
fn a_still_press_beside_a_minimum_column_sends_nothing() {
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let cx = TestAppContext::single();
    let (view, _) = cx.update(|app| live_view_of(app, &install, "deck.fodp", DECK));
    let right: Place = |t| t.columns.offset + t.columns.right;
    let quarter = |t: &Table| t.rows.offset + t.rows.left + (t.rows.right - t.rows.left) / 4;
    let mut table = cx.update(|app| view.update(app, |view, _| table_in(view, ("2", "2"))));
    // Its minimum width drops once it narrows; shrink twice.
    for _ in 0..2 {
        let before = cx.update(|app| {
            view.update(app, |view, cx| {
                let on = view.window_point((right(&table), quarter(&table)));
                let to = on + gpui::point(px(-600.0), px(0.0));
                assert!(view.hold_border(on) && view.move_border(to) && view.drop_border(to, cx));
                right(&table)
            })
        });
        for _ in 0..80 {
            cx.executor().tick();
        }
        table = cx.update(|app| view.update(app, |view, _| moved(view, right, before)));
    }
    let sent = cx.update(|app| {
        view.update(app, |view, cx| {
            let line = table.columns.inner[0];
            assert_eq!(i64::from(line.position), line.max, "the right column is at its minimum");
            let on = view.window_point((table.columns.offset + line.position, quarter(&table)));
            let sent = view.sent();
            assert!(view.hold_border(on));
            assert!(view.drop_border(on, cx));
            sent
        })
    });
    for _ in 0..80 {
        cx.executor().tick();
    }
    assert_eq!(cx.update(|app| view.read(app).sent()), sent, "a still press sends no command");
}
