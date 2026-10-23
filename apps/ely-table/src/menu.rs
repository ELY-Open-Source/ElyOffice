//! The right-click menus, laid out as Excel's (TABLE-02).

use ely_gpui_component::{
    menus::{ContextMenu, Menu, MenuItem},
    primitives::Severity,
};
use ely_lok::{
    LokView, Rect,
    menu::{Runs, Target, runs},
    sheet::Geometry,
};
use ely_office::menu::{self, Entry};
use ely_ui::menu::Offer;
use ely_office::prelude::*;
use gpui::{App, Context, Entity, Pixels, Point, Window};

use crate::book::Book;

/// What was right-clicked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Cell,
    Object,
    /// A cell's text, being edited.
    Text,
    Rows,
    Columns,
}

/// A menu's kind and what the kernel lets run.
pub struct What {
    pub kind: Kind,
    runs: Runs,
}

/// One thing a menu does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pick {
    Cut,
    Copy,
    Paste,
    RowsAbove,
    RowsBelow,
    ColumnsBefore,
    ColumnsAfter,
    InsertDown,
    InsertRight,
    DeleteRows,
    DeleteColumns,
    DeleteUp,
    DeleteLeft,
    Clear,
    Merge,
    Split,
    Format,
    RowHeight,
    ColumnWidth,
    HideRows,
    ShowRows,
    HideColumns,
    ShowColumns,
    Front,
    Forward,
    Back,
    Backward,
}

const CLIPBOARD: [Pick; 3] = [Pick::Cut, Pick::Copy, Pick::Paste];

const CELL: &[Entry<Pick>] = &[
    Entry::Line,
    Entry::Sub(
        "table.menu.insert",
        &[
            Pick::RowsAbove,
            Pick::RowsBelow,
            Pick::ColumnsBefore,
            Pick::ColumnsAfter,
            Pick::InsertDown,
            Pick::InsertRight,
        ],
    ),
    Entry::Sub(
        "table.menu.delete",
        &[Pick::DeleteRows, Pick::DeleteColumns, Pick::DeleteUp, Pick::DeleteLeft],
    ),
    Entry::Row(Pick::Clear),
    Entry::Line,
    Entry::Row(Pick::Merge),
    Entry::Row(Pick::Split),
    Entry::Line,
    Entry::Row(Pick::Format),
];

const OBJECT: &[Entry<Pick>] = &[
    Entry::Line,
    Entry::Sub("office.menu.front", &[Pick::Front, Pick::Forward]),
    Entry::Sub("office.menu.back", &[Pick::Back, Pick::Backward]),
];

const ROWS: &[Entry<Pick>] = &[
    Entry::Line,
    Entry::Row(Pick::RowsAbove),
    Entry::Row(Pick::DeleteRows),
    Entry::Row(Pick::Clear),
    Entry::Line,
    Entry::Row(Pick::RowHeight),
    Entry::Row(Pick::HideRows),
    Entry::Row(Pick::ShowRows),
];

const COLUMNS: &[Entry<Pick>] = &[
    Entry::Line,
    Entry::Row(Pick::ColumnsBefore),
    Entry::Row(Pick::DeleteColumns),
    Entry::Row(Pick::Clear),
    Entry::Line,
    Entry::Row(Pick::ColumnWidth),
    Entry::Row(Pick::HideColumns),
    Entry::Row(Pick::ShowColumns),
];

impl menu::Pick for Pick {
    fn row(self) -> (&'static str, &'static str) {
        match self {
            Pick::Cut => ("office.menu.cut", ".uno:Cut"),
            Pick::Copy => ("office.menu.copy", ".uno:Copy"),
            Pick::Paste => ("office.menu.paste", ".uno:Paste"),
            Pick::RowsAbove => ("table.cells.rows_above", ".uno:InsertRowsBefore"),
            Pick::RowsBelow => ("table.cells.rows_below", ".uno:InsertRowsAfter"),
            Pick::ColumnsBefore => ("table.cells.columns_before", ".uno:InsertColumnsBefore"),
            Pick::ColumnsAfter => ("table.cells.columns_after", ".uno:InsertColumnsAfter"),
            Pick::InsertDown => ("table.cells.insert_down", ".uno:InsertCell"),
            Pick::InsertRight => ("table.cells.insert_right", ".uno:InsertCell"),
            Pick::DeleteRows => ("table.cells.delete_rows", ".uno:DeleteRows"),
            Pick::DeleteColumns => ("table.cells.delete_columns", ".uno:DeleteColumns"),
            Pick::DeleteUp => ("table.cells.delete_up", ".uno:DeleteCell"),
            Pick::DeleteLeft => ("table.cells.delete_left", ".uno:DeleteCell"),
            Pick::Clear => ("table.menu.clear", ".uno:ClearContents"),
            Pick::Merge => ("table.cells.merge", ".uno:MergeCells"),
            Pick::Split => ("table.cells.split", ".uno:SplitCell"),
            Pick::Format => ("table.menu.format", ".uno:FormatCellDialog"),
            Pick::RowHeight => ("table.cells.row_height", ".uno:RowHeight"),
            Pick::ColumnWidth => ("table.cells.column_width", ".uno:ColumnWidth"),
            Pick::HideRows => ("table.cells.hide_rows", ".uno:HideRow"),
            Pick::ShowRows => ("table.cells.show_rows", ".uno:ShowRow"),
            Pick::HideColumns => ("table.cells.hide_columns", ".uno:HideColumn"),
            Pick::ShowColumns => ("table.cells.show_columns", ".uno:ShowColumn"),
            Pick::Front => ("office.menu.front", ".uno:BringToFront"),
            Pick::Forward => ("office.menu.forward", ".uno:ObjectForwardOne"),
            Pick::Back => ("office.menu.back", ".uno:SendToBack"),
            Pick::Backward => ("office.menu.backward", ".uno:ObjectBackOne"),
        }
    }
}

impl What {
    /// The kernel lets the pick's own command run now.
    pub(crate) fn on(&self, pick: Pick) -> bool {
        self.runs.allows(menu::Pick::row(pick).1)
    }
}

impl Kind {
    /// Its rows under the clipboard row.
    fn rows(self) -> &'static [Entry<Pick>] {
        match self {
            Kind::Cell => CELL,
            Kind::Object => OBJECT,
            Kind::Text => &[],
            Kind::Rows => ROWS,
            Kind::Columns => COLUMNS,
        }
    }

    /// Every command its rows run.
    fn commands(self) -> Vec<&'static str> {
        let rows = self.rows().iter().flat_map(|entry| match entry {
            Entry::Row(pick) => std::slice::from_ref(pick),
            Entry::Sub(_, picks) => picks,
            Entry::Line => &[],
        });
        let mut commands: Vec<_> = CLIPBOARD.iter().chain(rows).map(|pick| menu::Pick::row(*pick).1).collect();
        commands.sort_unstable();
        commands.dedup();
        commands
    }
}

/// A whole selected row or column covers the header.
pub(crate) fn held(selection: &[Rect], geometry: &Geometry, column: bool, index: u32) -> bool {
    let (along, across) = if column {
        (&geometry.columns, &geometry.rows)
    } else {
        (&geometry.rows, &geometry.columns)
    };
    let (from, to) = (along.start(index), along.start(index + 1));
    let end = across.start(across.count());
    selection.iter().any(|&(x, y, w, h)| {
        let ((start, size), (side, span)) = if column { ((x, w), (y, h)) } else { ((y, h), (x, w)) };
        side == 0 && span + 1 >= end && start <= from && to <= start + size + 1
    })
}

impl Book {
    /// Turns the kernel's context menu into Ely's.
    pub(crate) fn offer_menu(&mut self, id: u64, view: &Entity<LokView>, cx: &mut Context<Self>) {
        let (offered, at) = match menu::take(view, cx) {
            None => return,
            Some(Ok(taken)) => taken,
            Some(Err(error)) => return self.engine_failed(&error.to_string(), cx),
        };
        let kind = match offered.target {
            Target::Cell => Kind::Cell,
            Target::Object | Target::Picture => Kind::Object,
            Target::Text => Kind::Text,
            Target::Table | Target::Frame | Target::Slide => {
                let error = format!("a {:?} menu in a spreadsheet", offered.target);
                return self.toast(Book::text(cx, "office.error.engine", &[("error", &error)]), Severity::Danger, cx);
            }
        };
        self.offer(id, kind, at, cx);
    }

    /// Selects a header, then offers its menu.
    pub(crate) fn header_menu(&mut self, column: bool, index: u32, at: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let id = tab.id;
        // A selected object keeps nothing held.
        let read = tab.view.read(cx);
        if read.graphic().is_some() || !held(read.selection(), &tab.extra.geometry, column, index) {
            self.select_header(column, index, "0", cx);
        }
        log::info!("table: a header menu, column {column}");
        self.menu = None;
        let kind = if column { Kind::Columns } else { Kind::Rows };
        match self.settled(id, cx) {
            true => self.offer(id, kind, at, cx),
            false => self.when_settled(id, move |book, _, cx| book.offer(id, kind, at, cx)),
        }
        cx.notify();
    }

    /// Asks what may run now, then opens the menu.
    fn offer(&mut self, id: u64, kind: Kind, at: Point<Pixels>, cx: &mut Context<Self>) {
        let commands = kind.commands();
        self.ask_in(
            id,
            move |worker, doc| runs(worker, doc, &commands),
            move |book, read, _, cx| match read {
                Ok(runs) => {
                    book.menu = Some(Offer::new(book.menu.as_ref(), id, What { kind, runs }, at));
                    cx.notify();
                }
                Err(error) => book.engine_failed(&error.to_string(), cx),
            },
            cx,
        );
    }

    /// The menu's host, for the active workbook.
    pub(crate) fn context_menu(&self, cx: &mut Context<Self>) -> Option<ContextMenu> {
        let offer = self.menu.as_ref().filter(|offer| Some(offer.tab) == self.core.active)?;
        Some(offer.host("table-menu", menu_of(&cx.entity(), &offer.what, cx)))
    }

    /// Runs one menu row.
    pub(crate) fn choose(&mut self, pick: Pick, window: &mut Window, cx: &mut Context<Self>) {
        log::info!("table: the menu picks {pick:?}");
        let (id, values): (&str, &[&str]) = match pick {
            Pick::Cut | Pick::Copy | Pick::Paste => return self.clipboard(pick, cx),
            Pick::Merge => return self.merge(self.core.active, cx),
            Pick::RowHeight => return self.ask_size(false, window, cx),
            Pick::ColumnWidth => return self.ask_size(true, window, cx),
            Pick::Format => {
                ely_ui::settings::panels::show(false, true, cx);
                return cx.notify();
            }
            Pick::RowsAbove => ("rows.insert_above", &[]),
            Pick::RowsBelow => ("rows.insert_below", &[]),
            Pick::ColumnsBefore => ("columns.insert_before", &[]),
            Pick::ColumnsAfter => ("columns.insert_after", &[]),
            Pick::InsertDown => ("cells.insert", &["V"]),
            Pick::InsertRight => ("cells.insert", &[">"]),
            Pick::DeleteRows => ("rows.delete", &[]),
            Pick::DeleteColumns => ("columns.delete", &[]),
            Pick::DeleteUp => ("cells.delete", &["U"]),
            Pick::DeleteLeft => ("cells.delete", &["L"]),
            Pick::Clear => ("cells.clear", &[]),
            Pick::Split => ("cells.split", &[]),
            Pick::HideRows => ("rows.hide", &[]),
            Pick::ShowRows => ("rows.show", &[]),
            Pick::HideColumns => ("columns.hide", &[]),
            Pick::ShowColumns => ("columns.show", &[]),
            Pick::Front => ("image.front", &[]),
            Pick::Forward => ("image.forward", &[]),
            Pick::Back => ("image.back", &[]),
            Pick::Backward => ("image.backward", &[]),
        };
        self.run_active(id, values, cx);
    }

    fn clipboard(&mut self, pick: Pick, cx: &mut Context<Self>) {
        let Some(view) = self.active_tab().map(|tab| tab.view.clone()) else {
            log::warn!("table: {pick:?} with no workbook");
            return;
        };
        view.update(cx, |view, _| match pick {
            Pick::Paste => view.paste(),
            _ => view.copy(pick == Pick::Cut),
        });
    }
}

/// The clipboard row, then the target's rows.
pub(crate) fn menu_of(this: &Entity<Book>, what: &What, cx: &App) -> Menu {
    let item = |pick: Pick| {
        let this = this.clone();
        MenuItem::new(Book::text(cx, menu::Pick::row(pick).0, &[]))
            .disabled(!what.on(pick))
            .on_click(move |window, cx| this.update(cx, |book, cx| book.choose(pick, window, cx)))
    };
    let menu = menu::clipboard(Menu::new(), CLIPBOARD, item);
    menu::entries(menu, what.kind.rows(), cx, |pick| what.on(pick), item)
}
