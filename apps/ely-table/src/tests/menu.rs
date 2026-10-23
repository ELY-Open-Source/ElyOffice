use ely_gpui_component::theme::Theme;
use ely_lok::{LokView, sheet};
use gpui::{Modifiers, MouseButton, Pixels, Point, VisualTestContext};

use super::data::{cells, open, run};
use super::*;
use crate::menu::{Kind, Pick, What, held};

type Shows = fn(&What) -> bool;

/// The grid with the menu's host over it.
pub(super) struct Host(pub(super) Entity<Book>);

impl gpui::Render for Host {
    fn render(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};
        let grid = self.0.update(cx, |book, cx| book.grid(window, cx));
        let menu = self.0.update(cx, |book, cx| book.context_menu(cx));
        gpui::div().relative().size_full().flex().flex_col().child(grid).children(menu)
    }
}

pub(super) fn frames(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

pub(super) fn view(cx: &mut VisualTestContext, book: &Entity<Book>) -> Entity<LokView> {
    cx.update(|_, app| book.read(app).active_tab().expect("a tab").view.clone())
}

/// A cell's middle or its header's, in window pixels.
pub(super) fn spot(cx: &mut VisualTestContext, book: &Entity<Book>, column: u32, row: u32, header: Option<bool>) -> Point<Pixels> {
    frames(cx);
    let view = view(cx, book);
    cx.update(|_, app| {
        let geometry = &book.read(app).active_tab().expect("a tab").extra.geometry;
        let x = geometry.columns.start(column) + geometry.columns.size(column) / 2;
        let y = geometry.rows.start(row) + geometry.rows.size(row) / 2;
        let read = view.read(app);
        let at = read.origin() + read.place((x, y));
        match header {
            None => at,
            Some(false) => gpui::point(read.origin().x - gpui::px(24.0), at.y),
            Some(true) => gpui::point(at.x, read.origin().y - gpui::px(12.0)),
        }
    })
}

/// Right-clicks and waits until `shown` holds for the menu.
fn right_click(cx: &mut VisualTestContext, book: &Entity<Book>, at: Point<Pixels>, shown: Shows) {
    cx.update(|_, app| book.update(app, |book, _| book.menu = None));
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::none());
    for _ in 0..500 {
        cx.run_until_parked();
        if cx.update(|_, app| book.read(app).menu.as_ref().is_some_and(|offer| shown(&offer.what))) {
            return frames(cx);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("no menu");
}

fn on(cx: &mut VisualTestContext, book: &Entity<Book>, pick: Pick) -> bool {
    cx.update(|_, app| book.read(app).menu.as_ref().expect("a menu").what.on(pick))
}

fn choose(cx: &mut VisualTestContext, book: &Entity<Book>, pick: Pick) {
    cx.update(|window, app| book.update(app, |book, cx| book.choose(pick, window, cx)));
}

/// A column range's cells, empty ones kept.
fn column(cx: &mut TestAppContext, book: &Entity<Book>, range: &str) -> Vec<String> {
    cells(cx, book, range).lines().map(|line| line.trim().to_string()).collect()
}

fn cell(what: &What) -> bool {
    what.kind == Kind::Cell
}

fn rows(what: &What) -> bool {
    what.kind == Kind::Rows
}

fn columns(what: &What) -> bool {
    what.kind == Kind::Columns
}

/// A new, selected rectangle over A1.
fn corner_rectangle(cx: &mut TestAppContext, book: &Entity<Book>) {
    run(cx, book, "draw.shape", &["rectangle", "true"]);
    run(cx, book, "object.x", &["0"]);
    run(cx, book, "object.y", &["0"]);
    let selected = cx.update(|app| book.read(app).active_tab().expect("a tab").view.read(app).graphic());
    assert!(selected.is_some_and(|(x, y, ..)| x < 100 && y < 100), "selected at A1: {selected:?}");
}

/// Protects the sheet, allowing selection and `more`.
fn protect(cx: &mut TestAppContext, book: &Entity<Book>, more: &[&str]) {
    let allow: Vec<String> = ["selectLocked", "selectUnlocked"].iter().chain(more).map(|key| key.to_string()).collect();
    run(cx, book, "sheet.protect", &[&sheet::protect_json("pw", &allow)]);
}

#[test]
fn right_clicks_open_excel_rows_on_cells_and_headers() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = super::book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    open(&mut cx, &book);
    settle(&cx, |app| book.read(app).active_tab().is_some_and(|tab| tab.extra.geometry.rows.count() > 0));
    let window = cx.add_window(|_, _| Host(book.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    let a2 = spot(&mut shown, &book, 0, 1, None);
    right_click(&mut shown, &book, a2, cell);
    let cut = shown.debug_bounds("menu-button-0-0").expect("the clipboard row");
    let off = cut.origin - a2;
    assert!(off.x >= gpui::px(0.0) && off.x < gpui::px(40.0) && off.y < gpui::px(40.0), "at the click: {off:?}");
    assert!(on(&mut shown, &book, Pick::RowsBelow) && on(&mut shown, &book, Pick::Clear));
    assert!(!on(&mut shown, &book, Pick::Merge), "one cell does not merge");
    choose(&mut shown, &book, Pick::RowsBelow);
    assert_eq!(column(&mut cx, &book, "A2:A4"), ["a", "", "b"], "a row below A2");
    // The read left A2:A4 selected; a click outside it moves.
    run(&mut cx, &book, "cell.goto", &["D9"]);
    let a2 = spot(&mut shown, &book, 0, 1, None);
    right_click(&mut shown, &book, a2, cell);
    choose(&mut shown, &book, Pick::Clear);
    assert_eq!(column(&mut cx, &book, "A1:A4"), ["Name", "", "", "b"], "cleared");
    assert_eq!(cells(&mut cx, &book, "B2"), "3", "one cell only");
    let row5 = spot(&mut shown, &book, 0, 4, Some(false));
    right_click(&mut shown, &book, row5, rows);
    assert!(on(&mut shown, &book, Pick::HideRows) && on(&mut shown, &book, Pick::DeleteRows));
    choose(&mut shown, &book, Pick::DeleteRows);
    assert_eq!(cells(&mut cx, &book, "A5"), "c", "row 5 went, not row 2");
    // A selected rectangle over A1 hides no header menu.
    corner_rectangle(&mut cx, &book);
    let row1 = spot(&mut shown, &book, 0, 0, Some(false));
    right_click(&mut shown, &book, row1, rows);
    assert!(on(&mut shown, &book, Pick::RowsAbove) && on(&mut shown, &book, Pick::HideRows));
    choose(&mut shown, &book, Pick::RowsAbove);
    assert_eq!(cells(&mut cx, &book, "A2"), "Name", "a row above row 1");
    corner_rectangle(&mut cx, &book);
    let a = spot(&mut shown, &book, 0, 0, Some(true));
    right_click(&mut shown, &book, a, columns);
    assert!(on(&mut shown, &book, Pick::ColumnsBefore) && on(&mut shown, &book, Pick::DeleteColumns));
    protect(&mut cx, &book, &[]);
    let headers: [(Point<Pixels>, Shows, [Pick; 4]); 2] = [
        (spot(&mut shown, &book, 0, 2, Some(false)), rows, [Pick::RowsAbove, Pick::DeleteRows, Pick::HideRows, Pick::ShowRows]),
        (spot(&mut shown, &book, 1, 0, Some(true)), columns, [Pick::ColumnsBefore, Pick::DeleteColumns, Pick::HideColumns, Pick::ShowColumns]),
    ];
    for (at, shown_as, picks) in headers {
        right_click(&mut shown, &book, at, shown_as);
        assert!(on(&mut shown, &book, Pick::Copy), "copy stays on");
        for pick in picks.into_iter().chain([Pick::Cut, Pick::Clear]) {
            assert!(!on(&mut shown, &book, pick), "{pick:?} is off on a protected sheet");
        }
    }
    // Protection that lets rows and columns in.
    run(&mut cx, &book, "sheet.protect", &[&sheet::unprotect_json("pw")]);
    protect(&mut cx, &book, &["insertRows", "insertColumns"]);
    let b = spot(&mut shown, &book, 1, 0, Some(true));
    right_click(&mut shown, &book, b, columns);
    assert!(on(&mut shown, &book, Pick::ColumnsBefore), "inserting columns is allowed");
    assert!(!on(&mut shown, &book, Pick::DeleteColumns), "deleting columns is not");
    let row2 = spot(&mut shown, &book, 0, 1, Some(false));
    right_click(&mut shown, &book, row2, rows);
    assert!(on(&mut shown, &book, Pick::RowsAbove), "inserting rows is allowed");
    assert!(!on(&mut shown, &book, Pick::DeleteRows), "deleting rows is not");
    choose(&mut shown, &book, Pick::RowsAbove);
    // Protected cells read as nothing.
    run(&mut cx, &book, "sheet.protect", &[&sheet::unprotect_json("pw")]);
    assert_eq!(cells(&mut cx, &book, "A3"), "Name", "a row above row 2");
}

#[test]
fn a_header_keeps_a_whole_selection_that_holds_it() {
    let geometry = sheet::geometry(
        r#"{"rows":{"sizes":"256:1048575","hidden":"0:1048575"},"columns":{"sizes":"1280:16383","hidden":"0:16383"}}"#,
    )
    .expect("parses");
    // As Calc reports column B, then row 3.
    let column = [(1280, 0, 1279, 268435455)];
    assert!(held(&column, &geometry, true, 1));
    assert!(!held(&column, &geometry, true, 0));
    assert!(!held(&column, &geometry, false, 0), "a column is no row");
    let row = [(0, 512, 20971519, 255)];
    assert!(held(&row, &geometry, false, 2));
    assert!(!held(&row, &geometry, true, 0));
    let one = [(1280, 256, 1279, 255)];
    assert!(!held(&one, &geometry, true, 1), "one cell is no column");
    assert!(!held(&[], &geometry, false, 0));
}

/// Starts typing into the selected shape's text.
fn type_in_shape(cx: &mut TestAppContext, book: &Entity<Book>) {
    cx.update(|app| {
        let book = book.read(app);
        let tab = book.active_tab().expect("a tab");
        let mut worker = book.core.worker.lock().expect("the worker");
        worker.post_key(&tab.doc, 0, 769).expect("F2");
        worker.post_key(&tab.doc, 'z' as u32, 537).expect("a key");
    });
    fresh(cx, book);
    let graphic = cx.update(|app| book.read(app).active_tab().expect("a tab").view.read(app).graphic());
    assert!(graphic.is_none(), "editing the shape's text: {graphic:?}");
}

#[test]
fn a_header_ends_shape_text_editing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = super::book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    open(&mut cx, &book);
    fresh(&cx, &book);
    let window = cx.add_window(|_, _| Host(book.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    let headers: [(bool, u32, Shows, Pick); 2] =
        [(false, 2, rows, Pick::RowsAbove), (true, 1, columns, Pick::ColumnsBefore)];
    for (column, index, shown_as, insert) in headers {
        corner_rectangle(&mut cx, &book);
        type_in_shape(&mut cx, &book);
        let at = if column { spot(&mut shown, &book, index, 2, Some(true)) } else { spot(&mut shown, &book, 0, index, Some(false)) };
        right_click(&mut shown, &book, at, shown_as);
        let picked = cx.update(|app| {
            let tab = book.read(app).active_tab().expect("a tab");
            held(tab.view.read(app).selection(), &tab.extra.geometry, column, index)
        });
        assert!(picked, "the header's line is selected, column {column}");
        assert!(on(&mut shown, &book, insert), "{insert:?} is on");
        choose(&mut shown, &book, insert);
    }
    assert_eq!(cells(&mut cx, &book, "A4"), "b", "a row above row 3");
    assert_eq!(cells(&mut cx, &book, "C2"), "3", "a column before B");
    let path = dir.path().join("shapes.fods");
    cx.update(|app| {
        let book = book.read(app);
        let mut worker = book.core.worker.lock().expect("the worker");
        let doc = &book.active_tab().expect("a tab").doc;
        doc.save_as(&mut worker, &ely_lok::file_url(&path), "fods").expect("the save");
    });
    let xml = std::fs::read_to_string(&path).expect("the save");
    assert_eq!(xml.matches(">z</text:p>").count(), 2, "each shape keeps its typed text");
}

#[test]
fn format_cells_opens_a_folded_right_panel() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = super::book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    super::blank(&mut cx, &book);
    let shown = |cx: &mut TestAppContext| cx.update(|app| ely_ui::settings::panels::shown(false, app));
    cx.update(|app| ely_ui::settings::panels::show(false, false, app));
    assert!(!shown(&mut cx), "folded");
    window.update(&mut cx, |_, window, app| book.update(app, |book, cx| book.choose(Pick::Format, window, cx))).expect("the window lives");
    assert!(shown(&mut cx), "Format Cells opens it");
}
