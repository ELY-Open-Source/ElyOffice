//! Header edges: double clicks fit, drags show a line (#15).

use ely_gpui_component::theme::Theme;
use gpui::{InputEvent as _, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, Pixels, Point, VisualTestContext, point, px};

use super::data::open;
use super::menu::{Host, frames, view};
use super::*;

/// Column A's width in print twips.
fn width_of_a(cx: &mut VisualTestContext, book: &Entity<Book>) -> i32 {
    cx.update(|_, app| book.read(app).active_tab().expect("a tab").extra.geometry.columns.size(0))
}

/// The edge between A and B in the column header.
fn a_edge(cx: &mut VisualTestContext, book: &Entity<Book>) -> Point<Pixels> {
    frames(cx);
    let view = view(cx, book);
    cx.update(|_, app| {
        let start = book.read(app).active_tab().expect("a tab").extra.geometry.columns.start(1);
        let read = view.read(app);
        let at = read.origin() + read.place((start, 0));
        // Inside the 4 px grab, clear of the cell border.
        point(at.x - px(3.0), read.origin().y - px(12.0))
    })
}

/// A book drawn in a window, its grid read.
fn shown(cx: &mut TestAppContext, dir: &Path) -> Option<(Entity<Book>, VisualTestContext)> {
    let (_window, book) = super::book(cx, dir)?;
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    open(cx, &book);
    settle(cx, |app| book.read(app).active_tab().is_some_and(|tab| tab.extra.geometry.rows.count() > 0));
    let window = cx.add_window(|_, _| Host(book.clone()));
    Some((book, VisualTestContext::from_window(window.into(), cx)))
}

#[test]
fn a_double_click_on_an_edge_fits_the_column() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, mut shown)) = shown(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let before = width_of_a(&mut shown, &book);
    let edge = a_edge(&mut shown, &book);
    let press = |count| MouseDownEvent {
        button: MouseButton::Left,
        position: edge,
        modifiers: Modifiers::none(),
        click_count: count,
        first_mouse: false,
    };
    let release = |count| MouseUpEvent {
        button: MouseButton::Left,
        position: edge,
        modifiers: Modifiers::none(),
        click_count: count,
    };
    shown.simulate_mouse_move(edge, None, Modifiers::none());
    for count in [1, 2] {
        shown.simulate_event(press(count));
        shown.simulate_event(release(count));
    }
    settle(&cx, |app| book.read(app).active_tab().is_some_and(|tab| tab.extra.geometry.columns.size(0) != before));
    assert!(shown.debug_bounds("table-guide").is_none(), "a double click drags no line");
    // Fitted already: fitting again changes nothing.
    let fitted = width_of_a(&mut shown, &book);
    super::data::run(&mut cx, &book, "columns.fit", &["0", "1"]);
    super::data::quiet(&cx, &book);
    for _ in 0..50 {
        cx.executor().tick();
    }
    assert_eq!(width_of_a(&mut shown, &book), fitted, "the double click fitted A");
}

#[test]
fn dragging_an_edge_shows_where_it_lands() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, mut shown)) = shown(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let before = width_of_a(&mut shown, &book);
    let edge = a_edge(&mut shown, &book);
    let to = edge + point(px(60.0), px(0.0));
    shown.simulate_mouse_move(edge, None, Modifiers::none());
    shown.simulate_mouse_down(edge, MouseButton::Left, Modifiers::none());
    shown.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    frames(&mut shown);
    let line = shown.debug_bounds("table-guide").expect("the line");
    assert!((line.origin.x - to.x).abs() < px(1.0), "{line:?} at {to:?}");
    shown.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    frames(&mut shown);
    assert!(shown.debug_bounds("table-guide").is_none(), "the line goes on release");
    settle(&cx, |app| book.read(app).active_tab().is_some_and(|tab| tab.extra.geometry.columns.size(0) > before));
}

#[test]
fn a_click_on_an_edge_sizes_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, mut shown)) = shown(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let before = width_of_a(&mut shown, &book);
    let edge = a_edge(&mut shown, &book);
    shown.simulate_mouse_move(edge, None, Modifiers::none());
    shown.simulate_click(edge, Modifiers::none());
    super::data::quiet(&cx, &book);
    for _ in 0..50 {
        cx.executor().tick();
    }
    assert_eq!(width_of_a(&mut shown, &book), before);
}

/// The kernel's selected range, as `S!$A$1:$C$1`.
fn selected(cx: &TestAppContext, book: &Entity<Book>) -> String {
    cx.update(|app| {
        let book = book.read(app);
        let tab = book.active_tab().expect("a tab");
        let mut worker = book.core.worker.lock().expect("the worker lock");
        let json = worker.values(&tab.doc, ely_lok::sheet::SELECTED).expect("the selection");
        ely_lok::sheet::selected_range(&json).expect("parses").expect("one").range
    })
}

#[test]
fn a_fast_cell_drag_selects_what_a_slow_one_does() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, mut shown)) = shown(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    // A first click elsewhere settles the view.
    let first = super::menu::spot(&mut shown, &book, 3, 3, None);
    shown.simulate_click(first, Modifiers::none());
    super::data::quiet(&cx, &book);
    let from = super::menu::spot(&mut shown, &book, 0, 0, None);
    let to = super::menu::spot(&mut shown, &book, 2, 0, None);
    shown.simulate_mouse_move(from, None, Modifiers::none());
    frames(&mut shown);
    // Slowly first, a frame after each event.
    shown.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    frames(&mut shown);
    shown.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    frames(&mut shown);
    shown.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    super::data::quiet(&cx, &book);
    assert_eq!(selected(&cx, &book), "S!$A$1:$C$1", "slowly");
    shown.simulate_click(from, Modifiers::none());
    super::data::quiet(&cx, &book);
    frames(&mut shown);
    // Then down, move and up before any frame.
    shown.update(|window, app| {
        let press = MouseDownEvent {
            button: MouseButton::Left,
            position: from,
            modifiers: Modifiers::none(),
            click_count: 1,
            first_mouse: false,
        };
        let moved = gpui::MouseMoveEvent {
            position: to,
            pressed_button: Some(MouseButton::Left),
            modifiers: Modifiers::none(),
        };
        let release = MouseUpEvent { button: MouseButton::Left, position: to, modifiers: Modifiers::none(), click_count: 1 };
        window.dispatch_event(press.to_platform_input(), app);
        window.dispatch_event(moved.to_platform_input(), app);
        window.dispatch_event(release.to_platform_input(), app);
    });
    super::data::quiet(&cx, &book);
    frames(&mut shown);
    super::data::quiet(&cx, &book);
    assert_eq!(selected(&cx, &book), "S!$A$1:$C$1", "in one frame");
}

/// The middle of header `index`, window coordinates.
fn header_at(cx: &mut VisualTestContext, book: &Entity<Book>, column: bool, index: u32) -> Point<Pixels> {
    frames(cx);
    let view = view(cx, book);
    cx.update(|_, app| {
        let geometry = &book.read(app).active_tab().expect("a tab").extra.geometry;
        let axis = if column { &geometry.columns } else { &geometry.rows };
        let middle = (axis.start(index) + axis.start(index + 1)) / 2;
        let read = view.read(app);
        let at = read.origin() + read.place((middle, middle));
        match column {
            true => point(at.x, read.origin().y - px(12.0)),
            false => point(read.origin().x - px(24.0), at.y),
        }
    })
}

#[test]
fn dragging_across_headers_selects_whole_columns_and_rows() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((book, mut shown)) = shown(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let width = width_of_a(&mut shown, &book);
    for (column, from, to, want) in [(true, 1, 3, "S!$B:$D"), (false, 1, 3, "S!$2:$4")] {
        let (from, to) = (header_at(&mut shown, &book, column, from), header_at(&mut shown, &book, column, to));
        shown.simulate_mouse_move(from, None, Modifiers::none());
        shown.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
        frames(&mut shown);
        shown.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
        frames(&mut shown);
        shown.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
        super::data::quiet(&cx, &book);
        assert_eq!(selected(&cx, &book), want, "whole columns or rows, dragged over");
    }
    assert_eq!(width_of_a(&mut shown, &book), width, "a header drag sizes nothing");
}
