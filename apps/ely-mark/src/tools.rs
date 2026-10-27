//! The table menu's work on the shown file (MARK-06).

use ely_gpui_component::secondary;
use ely_gpui_component::documents::TableEdit;
use ely_ui::{commands, i18n::text};
use gpui::{Action, App, Context, InteractiveElement as _, KeyBinding, Menu, MenuItem, actions};

use crate::{blocks::edits, desk::Desk, keys::item};

actions!(
    mark,
    [
        InsertTable,
        RowAbove,
        RowBelow,
        ColumnLeft,
        ColumnRight,
        MoveRowUp,
        MoveRowDown,
        MoveColumnLeft,
        MoveColumnRight,
        DeleteRow,
        DeleteColumn,
        TidyTable,
        DeleteTable
    ]
);

pub fn bind(cx: &mut App) {
    let at = Some(commands::CONTEXT);
    // An open block's input takes Ctrl+Enter as Submit.
    let open = Some("MarkdownDocument > ElyInput");
    cx.bind_keys([
        KeyBinding::new(secondary!("t"), InsertTable, at),
        KeyBinding::new(secondary!("enter"), RowBelow, at),
        KeyBinding::new(secondary!("enter"), RowBelow, open),
        KeyBinding::new("alt-up", MoveRowUp, at),
        KeyBinding::new("alt-down", MoveRowDown, at),
        KeyBinding::new("alt-left", MoveColumnLeft, at),
        KeyBinding::new("alt-right", MoveColumnRight, at),
        KeyBinding::new(secondary!("shift-backspace"), DeleteRow, at),
    ]);
}

/// Typora's Paragraph ▸ Table.
pub fn table_menu(cx: &App) -> Menu {
    let item = |key: &str, action: Box<dyn Action>| item(key, action, cx);
    Menu::new(text(cx, "mark.menu.table", &[])).items([
        item("mark.table.insert", Box::new(InsertTable)),
        MenuItem::separator(),
        item("mark.table.row_above", Box::new(RowAbove)),
        item("mark.table.row_below", Box::new(RowBelow)),
        item("mark.table.column_left", Box::new(ColumnLeft)),
        item("mark.table.column_right", Box::new(ColumnRight)),
        MenuItem::separator(),
        item("mark.table.move_row_up", Box::new(MoveRowUp)),
        item("mark.table.move_row_down", Box::new(MoveRowDown)),
        item("mark.table.move_column_left", Box::new(MoveColumnLeft)),
        item("mark.table.move_column_right", Box::new(MoveColumnRight)),
        MenuItem::separator(),
        item("mark.table.delete_row", Box::new(DeleteRow)),
        item("mark.table.delete_column", Box::new(DeleteColumn)),
        MenuItem::separator(),
        item("mark.table.tidy", Box::new(TidyTable)),
        MenuItem::separator(),
        item("mark.table.delete", Box::new(DeleteTable)),
    ])
}

impl Desk {
    pub fn table_actions(root: gpui::Div, cx: &mut Context<Self>) -> gpui::Div {
        use TableEdit as T;
        edits!(root, cx,
            InsertTable => |d, cx| d.insert_table(cx),
            RowAbove => |d, cx| d.edit_table(T::RowAbove, cx),
            RowBelow => |d, cx| d.edit_table(T::RowBelow, cx),
            ColumnLeft => |d, cx| d.edit_table(T::ColumnLeft, cx),
            ColumnRight => |d, cx| d.edit_table(T::ColumnRight, cx),
            MoveRowUp => |d, cx| d.edit_table(T::MoveRowUp, cx),
            MoveRowDown => |d, cx| d.edit_table(T::MoveRowDown, cx),
            MoveColumnLeft => |d, cx| d.edit_table(T::MoveColumnLeft, cx),
            MoveColumnRight => |d, cx| d.edit_table(T::MoveColumnRight, cx),
            DeleteRow => |d, cx| d.edit_table(T::DeleteRow, cx),
            DeleteColumn => |d, cx| d.edit_table(T::DeleteColumn, cx),
            TidyTable => |d, cx| d.edit_table(T::Tidy, cx),
            DeleteTable => |d, cx| d.edit_table(T::Delete, cx),
        )
    }
}
