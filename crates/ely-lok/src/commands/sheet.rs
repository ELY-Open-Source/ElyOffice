//! Spreadsheet rows: sheets, cells, rows and columns.

use super::{Arg, Check, Command, command};

/// A sheet position, counted from 1.
const SHEET: Arg = Arg::Int("Index", "unsigned short", 1, 10000);

/// The selection modifier: none, Shift or the command key.
const MODIFIER: Arg = Arg::Int("Modifier", "short", 0, 0x3000);

/// Workbook and range editing commands (TABLE-01, TABLE-02).
pub(super) const ROWS: &[Command] = &[
    command(
        "sheet.insert",
        ".uno:Insert",
        &[Arg::Text("Name"), SHEET],
        Check::Undoable,
    ),
    command(
        "sheet.rename",
        ".uno:Name",
        &[Arg::Text("Name"), SHEET],
        Check::Undoable,
    ),
    command(
        "sheet.move",
        ".uno:Move",
        &[SHEET, Arg::Bool("Copy"), Arg::Bool("UseCurrentDocument")],
        Check::Undoable,
    ),
    command(
        "sheet.hide",
        ".uno:Hide",
        &[Arg::Int("nTabNumber", "short", 0, 9999)],
        Check::Undoable,
    ),
    command(
        "sheet.show",
        ".uno:Show",
        &[Arg::Text("aTableName")],
        Check::Undoable,
    ),
    command("sheet.delete", ".uno:Remove", &[SHEET], Check::Undoable),
    command(
        "cell.goto",
        ".uno:GoToCell",
        &[Arg::Text("ToPoint")],
        Check::Result,
    ),
    command(
        "cell.enter",
        ".uno:EnterString",
        &[Arg::Loose("StringName")],
        Check::Undoable,
    ),
    // By address, the sheet named; the cursor stays.
    command(
        "cell.set",
        ".uno:EnterString",
        &[Arg::Text("Address"), Arg::Loose("StringName")],
        Check::Undoable,
    ),
    command(
        "rows.insert_above",
        ".uno:InsertRowsBefore",
        &[],
        Check::Undoable,
    ),
    command(
        "rows.insert_below",
        ".uno:InsertRowsAfter",
        &[],
        Check::Undoable,
    ),
    command(
        "columns.insert_before",
        ".uno:InsertColumnsBefore",
        &[],
        Check::Undoable,
    ),
    command(
        "columns.insert_after",
        ".uno:InsertColumnsAfter",
        &[],
        Check::Undoable,
    ),
    command("rows.delete", ".uno:DeleteRows", &[], Check::Undoable),
    command("columns.delete", ".uno:DeleteColumns", &[], Check::Undoable),
    // Flags: V shifts down, > shifts right.
    command(
        "cells.insert",
        ".uno:InsertCell",
        &[Arg::Text("Flags")],
        Check::Undoable,
    ),
    // Flags: U shifts up, L shifts left.
    command(
        "cells.delete",
        ".uno:DeleteCell",
        &[Arg::Text("Flags")],
        Check::Undoable,
    ),
    command(
        "cells.merge",
        ".uno:MergeCells",
        &[Arg::Bool("MoveContents")],
        Check::Undoable,
    ),
    command("cells.split", ".uno:SplitCell", &[], Check::Undoable),
    command("cells.clear", ".uno:ClearContents", &[], Check::Undoable),
    command("view.freeze", ".uno:FreezePanes", &[], Check::Finished),
    command("rows.hide", ".uno:HideRow", &[], Check::Undoable),
    command("rows.show", ".uno:ShowRow", &[], Check::Undoable),
    command("columns.hide", ".uno:HideColumn", &[], Check::Undoable),
    command("columns.show", ".uno:ShowColumn", &[], Check::Undoable),
    // Sizes in 1/100 mm; rows and columns from 1.
    command(
        "rows.height",
        ".uno:RowHeight",
        &[
            Arg::Int("RowHeight", "unsigned short", 1, 65535),
            Arg::Int("Row", "long", 1, 1048576),
        ],
        Check::Undoable,
    ),
    command(
        "columns.width",
        ".uno:ColumnWidth",
        &[
            Arg::Int("ColumnWidth", "unsigned short", 1, 65535),
            Arg::Int("Column", "unsigned short", 1, 16384),
        ],
        Check::Undoable,
    ),
    // The same sizes for every selected row or column.
    command(
        "rows.height_selected",
        ".uno:RowHeight",
        &[Arg::Int("RowHeight", "unsigned short", 1, 65535)],
        Check::Undoable,
    ),
    command(
        "columns.width_selected",
        ".uno:ColumnWidth",
        &[Arg::Int("ColumnWidth", "unsigned short", 1, 65535)],
        Check::Undoable,
    ),
    command(
        "rows.optimal",
        ".uno:SetOptimalRowHeight",
        &[Arg::Int("aExtraHeight", "unsigned short", 0, 0)],
        Check::Undoable,
    ),
    command(
        "columns.optimal",
        ".uno:SetOptimalColumnWidth",
        &[Arg::Int("aExtraWidth", "unsigned short", 0, 0)],
        Check::Undoable,
    ),
    // A header edge's double click: one, or the marked ones.
    command(
        "columns.fit",
        ".uno:SetOptimalColumnWidth",
        &[
            Arg::Int("aExtraWidth", "unsigned short", 0, 0),
            Arg::Int("Column", "unsigned short", 1, 16384),
        ],
        Check::Undoable,
    ),
    command(
        "rows.fit",
        ".uno:SetOptimalRowHeight",
        &[
            Arg::Int("aExtraHeight", "unsigned short", 0, 0),
            Arg::Int("Row", "long", 1, 1_048_576),
        ],
        Check::Undoable,
    ),
    // Rows and columns from 0.
    command(
        "select.column",
        ".uno:SelectColumn",
        &[Arg::Int("Col", "long", 0, 16383), MODIFIER],
        Check::Finished,
    ),
    command(
        "select.row",
        ".uno:SelectRow",
        &[Arg::Int("Row", "long", 0, 1048575), MODIFIER],
        Check::Finished,
    ),
    command(
        "fill.auto",
        ".uno:AutoFill",
        &[Arg::Text("EndCell")],
        Check::Undoable,
    ),
];
