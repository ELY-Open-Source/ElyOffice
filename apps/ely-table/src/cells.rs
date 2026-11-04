//! The Cells menu: rows, columns, merges, sizes, freeze (TABLE-02).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::TextInput,
    overlays::{Dialog as Card, PromptDialog},
    primitives::Severity,
};
use ely_lok::sheet;
use ely_office::prelude::*;
use gpui::{
    App, AppContext as _, Context, Div, Entity, InteractiveElement as _, Menu, MenuItem, Window,
    actions,
};

use crate::book::Book;
use crate::dialog::Dialog;

actions!(
    table,
    [
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
        Merge,
        Split,
        RowHeight,
        ColumnWidth,
        FitRows,
        FitColumns,
        HideRows,
        ShowRows,
        HideColumns,
        ShowColumns,
        Freeze
    ]
);

pub fn menu(cx: &App) -> Menu {
    let label = |key: &str| Book::text(cx, key, &[]);
    Menu::new(label("menu.cells")).items([
        MenuItem::action(label("table.cells.rows_above"), RowsAbove),
        MenuItem::action(label("table.cells.rows_below"), RowsBelow),
        MenuItem::action(label("table.cells.columns_before"), ColumnsBefore),
        MenuItem::action(label("table.cells.columns_after"), ColumnsAfter),
        MenuItem::action(label("table.cells.insert_down"), InsertDown),
        MenuItem::action(label("table.cells.insert_right"), InsertRight),
        MenuItem::separator(),
        MenuItem::action(label("table.cells.delete_rows"), DeleteRows),
        MenuItem::action(label("table.cells.delete_columns"), DeleteColumns),
        MenuItem::action(label("table.cells.delete_up"), DeleteUp),
        MenuItem::action(label("table.cells.delete_left"), DeleteLeft),
        MenuItem::separator(),
        MenuItem::action(label("table.cells.merge"), Merge),
        MenuItem::action(label("table.cells.split"), Split),
        MenuItem::separator(),
        MenuItem::action(label("table.cells.row_height"), RowHeight),
        MenuItem::action(label("table.cells.column_width"), ColumnWidth),
        MenuItem::action(label("table.cells.fit_rows"), FitRows),
        MenuItem::action(label("table.cells.fit_columns"), FitColumns),
        MenuItem::separator(),
        MenuItem::action(label("table.cells.hide_rows"), HideRows),
        MenuItem::action(label("table.cells.show_rows"), ShowRows),
        MenuItem::action(label("table.cells.hide_columns"), HideColumns),
        MenuItem::action(label("table.cells.show_columns"), ShowColumns),
        MenuItem::separator(),
        MenuItem::action(label("table.cells.freeze"), Freeze),
    ])
}

/// Binds an action to a command with fixed values.
macro_rules! run {
    ($root:expr, $cx:expr, $action:ty, $id:literal $(, $value:literal)*) => {
        $root.on_action($cx.listener(|book, _: &$action, _, cx| {
            book.run_active($id, &[$($value),*], cx)
        }))
    };
}

pub fn bind(root: Div, cx: &mut Context<Book>) -> Div {
    let root = run!(root, cx, RowsAbove, "rows.insert_above");
    let root = run!(root, cx, RowsBelow, "rows.insert_below");
    let root = run!(root, cx, ColumnsBefore, "columns.insert_before");
    let root = run!(root, cx, ColumnsAfter, "columns.insert_after");
    let root = run!(root, cx, InsertDown, "cells.insert", "V");
    let root = run!(root, cx, InsertRight, "cells.insert", ">");
    let root = run!(root, cx, DeleteRows, "rows.delete");
    let root = run!(root, cx, DeleteColumns, "columns.delete");
    let root = run!(root, cx, DeleteUp, "cells.delete", "U");
    let root = run!(root, cx, DeleteLeft, "cells.delete", "L");
    let root = run!(root, cx, Split, "cells.split");
    let root = run!(root, cx, FitRows, "rows.optimal", "0");
    let root = run!(root, cx, FitColumns, "columns.optimal", "0");
    let root = run!(root, cx, HideRows, "rows.hide");
    let root = run!(root, cx, ShowRows, "rows.show");
    let root = run!(root, cx, HideColumns, "columns.hide");
    let root = run!(root, cx, ShowColumns, "columns.show");
    let root = run!(root, cx, Freeze, "view.freeze");
    root.on_action(cx.listener(|book, _: &Merge, _, cx| book.merge(book.core.active(), cx)))
        .on_action(cx.listener(|book, _: &RowHeight, window, cx| book.ask_size(false, window, cx)))
        .on_action(cx.listener(|book, _: &ColumnWidth, window, cx| book.ask_size(true, window, cx)))
}

impl Book {
    /// Merges; asks first when more than one cell holds data.
    pub(crate) fn merge(&mut self, id: Option<u64>, cx: &mut Context<Self>) {
        let Some(tab) = id.and_then(|id| self.tab(id)) else {
            return self.run_in(id, "cells.merge", &["false"], cx);
        };
        if !self.settled(tab.id, cx) {
            return self.when_settled(tab.id, move |book, _, cx| book.merge(id, cx));
        }
        let tab = tab.id;
        self.ask_in(
            tab,
            |worker, doc| sheet::selected_range(&worker.values(doc, sheet::SELECTED)?),
            move |book, read, _, cx| match read {
                Ok(Some(selection)) if selection.filled > 1 => {
                    book.open_dialog(Dialog::Merge { tab });
                    cx.notify();
                }
                Ok(Some(_)) => book.run_in(id, "cells.merge", &["false"], cx),
                Ok(None) => {
                    let text = Book::text(cx, "table.merge.several", &[]);
                    book.toast(text, Severity::Warning, cx);
                }
                Err(error) => book.engine_failed(&error.to_string(), cx),
            },
            cx,
        );
    }

    /// Sets the asked size, in 1/100 mm, in its workbook.
    pub(crate) fn size_asked(&mut self, size: i32, cx: &mut Context<Self>) {
        let Some(Dialog::Size { tab, column, .. }) = self.dialog.take() else {
            return;
        };
        let id = if column {
            "columns.width_selected"
        } else {
            "rows.height_selected"
        };
        self.run_in(Some(tab), id, &[&size.to_string()], cx);
    }

    pub(crate) fn ask_size(&mut self, column: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active() else {
            let text = Book::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, ely_gpui_component::primitives::Severity::Warning, cx);
        };
        let field = cx.new(|cx| TextInput::new(window, cx));
        self.open_dialog(Dialog::Size { tab, column, field });
        cx.notify();
    }
}

/// Centimeters as 1/100 mm, within what the kernel takes.
fn hundredths(text: &str) -> Option<i32> {
    let centimeters: f64 = text.trim().replace(',', ".").parse().ok()?;
    let hundredths = (centimeters * 1000.0).round();
    (1.0..=65535.0)
        .contains(&hundredths)
        .then_some(hundredths as i32)
}

pub fn size_dialog(
    this: Entity<Book>,
    (column, field): (bool, &Entity<TextInput>),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> PromptDialog {
    let title = Book::text(
        cx,
        if column {
            "table.size.column"
        } else {
            "table.size.row"
        },
        &[],
    );
    let bad = Book::text(cx, "table.size.bad", &[]);
    PromptDialog::new("table-size", title, field, dismiss)
        .label(Book::text(cx, "table.size.label", &[]))
        .submit(Book::text(cx, "table.size.apply", &[]))
        .check(move |text| hundredths(text).map(|_| ()).ok_or_else(|| bad.clone()))
        .on_submit(move |text, _, cx| {
            let size = hundredths(text).expect("checked");
            this.update(cx, |book, cx| book.size_asked(size, cx))
        })
}

pub fn merge_dialog(
    this: Entity<Book>,
    tab: u64,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let choice = |id: &'static str, key: &str, moves: &'static str, variant| {
        let (this, label) = (this.clone(), Book::text(cx, key, &[]));
        move |_| {
            Button::new(id, label)
                .variant(variant)
                .on_click(move |_, _, cx| {
                    this.update(cx, |book, cx| {
                        book.dialog = None;
                        book.run_in(Some(tab), "cells.merge", &[moves], cx);
                    })
                })
        }
    };
    Card::new(
        "table-merge",
        Book::text(cx, "table.merge.title", &[]),
        dismiss,
    )
    .detail(Book::text(cx, "table.merge.detail", &[]))
    .action(ely_office::cancel(cx))
    .action(choice(
        "table-merge-keep",
        "table.merge.keep",
        "false",
        ButtonVariant::Secondary,
    ))
    .action(choice(
        "table-merge-move",
        "table.merge.move",
        "true",
        ButtonVariant::Primary,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(hundredths("2.54"), Some(2540));
        assert_eq!(hundredths("1,5"), Some(1500));
        assert_eq!(hundredths("0"), None);
        assert_eq!(hundredths("70"), None, "past 65.535 cm");
        assert_eq!(hundredths("x"), None);
    }
}
