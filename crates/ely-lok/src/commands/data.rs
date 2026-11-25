//! Spreadsheet rows: formats, data, rules.

use super::{Arg, Check, Command, command};

/// A column for sorting, from 1; 0 for none.
const fn key(name: &'static str) -> Arg {
    Arg::Int(name, "long", 0, 16_384)
}

/// A row or column index, from 0.
const fn index(name: &'static str) -> Arg {
    Arg::Int(name, "long", 0, 1_048_575)
}

/// Cell borders: the outer, the inner lines, or none.
const fn borders(id: &'static str, outer: bool, inner: bool) -> Command {
    command(
        id,
        ".uno:SetBorderStyle",
        if inner {
            &[
                Arg::Line("OuterBorder.LeftBorder", true),
                Arg::Line("OuterBorder.RightBorder", true),
                Arg::Line("OuterBorder.TopBorder", true),
                Arg::Line("OuterBorder.BottomBorder", true),
                Arg::Line("InnerBorder.Horizontal", true),
                Arg::Line("InnerBorder.Vertical", true),
            ]
        } else if outer {
            &[
                Arg::Line("OuterBorder.LeftBorder", true),
                Arg::Line("OuterBorder.RightBorder", true),
                Arg::Line("OuterBorder.TopBorder", true),
                Arg::Line("OuterBorder.BottomBorder", true),
                Arg::Line("InnerBorder.Horizontal", false),
                Arg::Line("InnerBorder.Vertical", false),
            ]
        } else {
            &[
                Arg::Line("OuterBorder.LeftBorder", false),
                Arg::Line("OuterBorder.RightBorder", false),
                Arg::Line("OuterBorder.TopBorder", false),
                Arg::Line("OuterBorder.BottomBorder", false),
                Arg::Line("InnerBorder.Horizontal", false),
                Arg::Line("InnerBorder.Vertical", false),
            ]
        },
        Check::Undoable,
    )
}

/// Formats, sorting, filtering and rules (TABLE-07..09).
pub(super) const ROWS: &[Command] = &[
    command(
        "number.general",
        ".uno:NumberFormatStandard",
        &[],
        Check::Undoable,
    ),
    command(
        "number.decimal",
        ".uno:NumberFormatDecimal",
        &[],
        Check::Undoable,
    ),
    command(
        "number.currency",
        ".uno:NumberFormatCurrency",
        &[],
        Check::Undoable,
    ),
    command(
        "number.percent",
        ".uno:NumberFormatPercent",
        &[],
        Check::Undoable,
    ),
    command("number.date", ".uno:NumberFormatDate", &[], Check::Undoable),
    command(
        "number.code",
        ".uno:NumberFormat",
        &[Arg::Text("FormatCode")],
        Check::Undoable,
    ),
    command("align.left", ".uno:AlignLeft", &[], Check::Undoable),
    command(
        "align.center",
        ".uno:AlignHorizontalCenter",
        &[],
        Check::Undoable,
    ),
    command("align.right", ".uno:AlignRight", &[], Check::Undoable),
    command("align.justify", ".uno:AlignBlock", &[], Check::Undoable),
    command("align.top", ".uno:AlignTop", &[], Check::Undoable),
    command("align.middle", ".uno:AlignVCenter", &[], Check::Undoable),
    command("align.bottom", ".uno:AlignBottom", &[], Check::Undoable),
    command(
        "cells.wrap",
        ".uno:WrapText",
        &[Arg::Bool("WrapText")],
        Check::Undoable,
    ),
    borders("cells.borders_all", true, true),
    borders("cells.borders_outer", true, false),
    borders("cells.borders_none", false, false),
    // Each edge: keep, none, or style:width:color; bits from boxitem.hxx.
    command(
        "cells.border",
        ".uno:SetBorderStyle",
        &[
            Arg::Edge("OuterBorder.LeftBorder", 0x04),
            Arg::Edge("OuterBorder.RightBorder", 0x08),
            Arg::Edge("OuterBorder.TopBorder", 0x01),
            Arg::Edge("OuterBorder.BottomBorder", 0x02),
            Arg::Edge("InnerBorder.Horizontal", 0x10),
            Arg::Edge("InnerBorder.Vertical", 0x20),
        ],
        Check::Undoable,
    ),
    command(
        "cells.fill",
        ".uno:BackgroundColor",
        &[Arg::Int("BackgroundColor.Color", "long", -1, 0xFF_FFFF)],
        Check::Undoable,
    ),
    command(
        "cells.text_color",
        ".uno:Color",
        &[Arg::Int("Color.Color", "long", -1, 0xFF_FFFF)],
        Check::Undoable,
    ),
    command(
        "data.sort",
        ".uno:DataSort",
        &[
            Arg::Bool("HasHeader"),
            key("Col1"),
            Arg::Bool("Ascending1"),
            key("Col2"),
            Arg::Bool("Ascending2"),
            key("Col3"),
            Arg::Bool("Ascending3"),
        ],
        Check::Undoable,
    ),
    command(
        "data.autofilter",
        ".uno:DataFilterAutoFilter",
        &[],
        Check::Undoable,
    ),
    command(
        "data.filter",
        ".uno:DataFilterStandardFilter",
        &[Arg::Text("Query")],
        Check::Undoable,
    ),
    command(
        "data.unfilter",
        ".uno:DataFilterRemoveFilter",
        &[],
        Check::Undoable,
    ),
    command(
        "data.dedupe",
        ".uno:HandleDuplicateRecords",
        &[
            Arg::Bool("Remove"),
            Arg::Bool("IncludesHeaders"),
            Arg::Bool("DuplicateRows"),
            index("StartColumn"),
            index("StartRow"),
            index("EndColumn"),
            index("EndRow"),
            Arg::Int("TabNo", "long", 0, 9_999),
        ],
        Check::Undoable,
    ),
    command(
        "data.columns",
        ".uno:TextToColumns",
        &[Arg::Text("Options")],
        Check::Undoable,
    ),
    command("select.data", ".uno:SelectData", &[], Check::Finished),
    command("paste.values", ".uno:PasteOnlyValue", &[], Check::Undoable),
    command(
        "paste.formulas",
        ".uno:PasteOnlyFormula",
        &[],
        Check::Undoable,
    ),
    command(
        "rules.validation",
        ".uno:Validation",
        &[Arg::Text("Rule")],
        Check::Undoable,
    ),
    command(
        "rules.conditional",
        ".uno:ConditionalFormatEasy",
        &[Arg::Text("Rule")],
        Check::Undoable,
    ),
];
