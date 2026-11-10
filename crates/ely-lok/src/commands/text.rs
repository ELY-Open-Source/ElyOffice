//! Text rows: editing, characters, paragraphs, lists, styles.

use super::{Arg, Check, Command, command};

/// Editing, formatting and style commands.
pub(super) const ROWS: &[Command] = &[
    command("edit.undo", ".uno:Undo", &[], Check::History),
    command("edit.redo", ".uno:Redo", &[], Check::History),
    command("edit.select_all", ".uno:SelectAll", &[], Check::Result),
    command("format.bold", ".uno:Bold", &[], Check::Result),
    command("format.italic", ".uno:Italic", &[], Check::Result),
    command("format.underline", ".uno:Underline", &[], Check::Result),
    command("format.strikeout", ".uno:Strikeout", &[], Check::Result),
    command(
        "format.font",
        ".uno:CharFontName",
        &[Arg::Font("CharFontName.FamilyName")],
        Check::Reports,
    ),
    command(
        "format.size",
        ".uno:FontHeight",
        &[Arg::Points("FontHeight.Height")],
        Check::Reports,
    ),
    command(
        "format.style",
        ".uno:StyleApply",
        &[Arg::Text("Style"), Arg::Text("FamilyName")],
        Check::Result,
    ),
    command("format.superscript", ".uno:SuperScript", &[], Check::Result),
    command("format.subscript", ".uno:SubScript", &[], Check::Result),
    command(
        "format.color",
        ".uno:FontColor",
        &[Arg::Int("FontColor.Color", "long", 0, 0xFF_FFFF)],
        Check::Reports,
    ),
    command(
        "format.highlight",
        ".uno:CharBackColor",
        &[Arg::Int("CharBackColor.Color", "long", -1, 0xFF_FFFF)],
        Check::Reports,
    ),
    command("paragraph.left", ".uno:LeftPara", &[], Check::Result),
    command("paragraph.center", ".uno:CenterPara", &[], Check::Result),
    command("paragraph.right", ".uno:RightPara", &[], Check::Result),
    command("paragraph.justify", ".uno:JustifyPara", &[], Check::Result),
    command(
        "paragraph.indent",
        ".uno:IncrementIndent",
        &[],
        Check::Result,
    ),
    command(
        "paragraph.outdent",
        ".uno:DecrementIndent",
        &[],
        Check::Result,
    ),
    command(
        "paragraph.first_line",
        ".uno:ElyFirstLineChars",
        &[Arg::Int("Chars", "long", -2_000, 2_000)],
        Check::Undoable,
    ),
    command("paragraph.spacing_1", ".uno:SpacePara1", &[], Check::Result),
    command(
        "paragraph.spacing_15",
        ".uno:SpacePara15",
        &[],
        Check::Result,
    ),
    command("paragraph.spacing_2", ".uno:SpacePara2", &[], Check::Result),
    command(
        "paragraph.spacing",
        ".uno:ULSpacing",
        &[
            Arg::Int("ULSpacing.TopMargin", "long", 0, 10_000),
            Arg::Int("ULSpacing.BottomMargin", "long", 0, 10_000),
        ],
        Check::Undoable,
    ),
    command(
        "paragraph.tab",
        ".uno:ChangeTabStop",
        &[
            Arg::Int("Index", "long", -1, 99),
            Arg::Int("Position", "long", 0, 31_680),
            Arg::Bool("Remove"),
        ],
        Check::Undoable,
    ),
    command("list.bullets", ".uno:DefaultBullet", &[], Check::Result),
    command(
        "list.numbering",
        ".uno:DefaultNumbering",
        &[],
        Check::Result,
    ),
    command("list.deeper", ".uno:DecrementLevel", &[], Check::Result),
    command("list.higher", ".uno:IncrementLevel", &[], Check::Result),
    command(
        "style.new",
        ".uno:StyleNewByExample",
        &[
            Arg::Text("Param"),
            Arg::Int("Family", "unsigned short", 1, 8),
        ],
        Check::Listed,
    ),
    command(
        "style.update",
        ".uno:StyleUpdateByExample",
        &[
            Arg::Text("Param"),
            Arg::Int("Family", "unsigned short", 1, 2),
        ],
        Check::Undoable,
    ),
];
