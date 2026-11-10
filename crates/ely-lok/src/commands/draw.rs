//! Drawing rows: shapes, lines, fill, outline, arrangement.

use super::{Arg, Check, Command, command};

/// Slide objects (SHOW-03 to 06).
pub(super) const ROWS: &[Command] = &[
    command(
        "draw.text",
        ".uno:Text",
        &[Arg::Bool("CreateDirectly")],
        Check::Undoable,
    ),
    command(
        "draw.shape",
        ".uno:BasicShapes",
        &[Arg::Text("BasicShapes"), Arg::Bool("CreateDirectly")],
        Check::Undoable,
    ),
    command("draw.line", ".uno:Line", &[], Check::Undoable),
    command("draw.arrow", ".uno:LineArrowEnd", &[], Check::Undoable),
    command("draw.connector", ".uno:Connector", &[], Check::Undoable),
    command(
        "object.fill",
        ".uno:FillColor",
        &[Arg::Int("FillColor", "long", 0, 0xFF_FFFF)],
        Check::Undoable,
    ),
    command(
        "object.fill_none",
        ".uno:FillStyle",
        &[Arg::Int("FillStyle", "long", 0, 0)],
        Check::Undoable,
    ),
    command(
        "object.line_none",
        ".uno:XLineStyle",
        &[Arg::Int("XLineStyle", "long", 0, 0)],
        Check::Undoable,
    ),
    command(
        "object.line_color",
        ".uno:XLineColor",
        &[Arg::Int("XLineColor", "long", 0, 0xFF_FFFF)],
        Check::Undoable,
    ),
    command(
        "object.line_width",
        ".uno:LineWidth",
        &[Arg::Int("LineWidth", "long", 0, 5_000)],
        Check::Undoable,
    ),
    command(
        "object.transparency",
        ".uno:FillTransparence",
        &[Arg::Int("FillTransparence", "unsigned short", 0, 100)],
        Check::Undoable,
    ),
    command(
        "object.shadow",
        ".uno:FillShadow",
        &[Arg::Bool("FillShadow")],
        Check::Undoable,
    ),
    command(
        "object.x",
        ".uno:TransformDialog",
        &[Arg::Int("TransformPosX", "long", -1_000_000, 1_000_000)],
        Check::Undoable,
    ),
    command(
        "object.y",
        ".uno:TransformDialog",
        &[Arg::Int("TransformPosY", "long", -1_000_000, 1_000_000)],
        Check::Undoable,
    ),
    command(
        "object.width",
        ".uno:TransformDialog",
        &[Arg::Int("TransformWidth", "long", 1, 1_000_000)],
        Check::Undoable,
    ),
    command(
        "object.height",
        ".uno:TransformDialog",
        &[Arg::Int("TransformHeight", "long", 1, 1_000_000)],
        Check::Undoable,
    ),
    command(
        "object.rotate",
        ".uno:TransformDialog",
        &[
            Arg::Int("TransformRotationAngle", "long", 0, 35_999),
            Arg::Int("TransformRotationX", "long", -1_000_000, 1_000_000),
            Arg::Int("TransformRotationY", "long", -1_000_000, 1_000_000),
        ],
        Check::Undoable,
    ),
    command(
        "object.lock",
        ".uno:TransformDialog",
        &[Arg::Bool("ProtectPos"), Arg::Bool("ProtectSize")],
        Check::Finished,
    ),
    command(
        "object.select",
        ".uno:SelectObject",
        &[Arg::Int("Index", "long", 0, 9_999), Arg::Bool("Add")],
        Check::Finished,
    ),
    // Patch 0117: the marked object's text edit opens.
    command(
        "object.edit",
        ".uno:SelectObject",
        &[Arg::Int("Index", "long", 0, 9_999), Arg::Bool("Add"), Arg::Bool("Edit")],
        Check::Finished,
    ),
    command(
        "object.deselect",
        ".uno:SelectObject",
        &[Arg::Bool("Add")],
        Check::Finished,
    ),
    command(
        "image.replace",
        ".uno:ChangePicture",
        &[Arg::Text("FileName")],
        Check::Undoable,
    ),
    command("object.align_left", ".uno:ObjectAlignLeft", &[], Check::Undoable),
    command("object.align_center", ".uno:AlignCenter", &[], Check::Undoable),
    command("object.align_right", ".uno:ObjectAlignRight", &[], Check::Undoable),
    command("object.align_top", ".uno:AlignUp", &[], Check::Undoable),
    command("object.align_middle", ".uno:AlignMiddle", &[], Check::Undoable),
    command("object.align_bottom", ".uno:AlignDown", &[], Check::Undoable),
    command(
        "object.spread_across",
        ".uno:DistributeHorzDistance",
        &[],
        Check::Undoable,
    ),
    command(
        "object.spread_down",
        ".uno:DistributeVertDistance",
        &[],
        Check::Undoable,
    ),
    command("object.group", ".uno:FormatGroup", &[], Check::Undoable),
    command("object.ungroup", ".uno:FormatUngroup", &[], Check::Undoable),
    command(
        "text.fit",
        ".uno:TextAutoFitToSize",
        &[Arg::Text("Fit")],
        Check::Undoable,
    ),
    command("view.snap_grid", ".uno:GridUse", &[], Check::Finished),
    command("view.snap_guides", ".uno:HelplinesUse", &[], Check::Finished),
    command(
        "guide.add",
        ".uno:CapturePoint",
        &[
            Arg::Int("Kind", "unsigned short", 0, 1),
            Arg::Int("X", "long", -1_000_000, 1_000_000),
            Arg::Int("Y", "long", -1_000_000, 1_000_000),
        ],
        Check::Finished,
    ),
    command(
        "guide.move",
        ".uno:CapturePoint",
        &[
            Arg::Int("Index", "long", 0, 999),
            Arg::Int("X", "long", -1_000_000, 1_000_000),
            Arg::Int("Y", "long", -1_000_000, 1_000_000),
        ],
        Check::Finished,
    ),
    command(
        "guide.delete",
        ".uno:CapturePoint",
        &[Arg::Int("Index", "long", 0, 999), Arg::Bool("Delete")],
        Check::Finished,
    ),
];
