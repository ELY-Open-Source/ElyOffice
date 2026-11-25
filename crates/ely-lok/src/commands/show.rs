//! Presentation rows: slides, design, charts, notes, comments.

use super::{Arg, Check, Command, command};

/// Slide management and design (SHOW-01, 02).
pub(super) const ROWS: &[Command] = &[
    command("slide.insert", ".uno:InsertPage", &[], Check::Undoable),
    command(
        "slide.duplicate",
        ".uno:DuplicatePage",
        &[],
        Check::Undoable,
    ),
    command("slide.delete", ".uno:DeletePage", &[], Check::Undoable),
    command("slide.hide", ".uno:HideSlide", &[], Check::Undoable),
    command("slide.show", ".uno:ShowSlide", &[], Check::Undoable),
    command("slide.up", ".uno:MovePageUp", &[], Check::Undoable),
    command("slide.down", ".uno:MovePageDown", &[], Check::Undoable),
    command("slide.first", ".uno:MovePageFirst", &[], Check::Undoable),
    command("slide.last", ".uno:MovePageLast", &[], Check::Undoable),
    command(
        "slide.at",
        ".uno:ElySlide",
        &[
            Arg::Text("Verb"),
            Arg::Int("Slide", "long", 0, 65_534),
            Arg::Int("Value", "long", -1, 65_534),
        ],
        Check::Undoable,
    ),
    command(
        "slide.text",
        ".uno:ElySlideText",
        &[Arg::Int("Slide", "long", 0, 65_534), Arg::Text("Box"), Arg::Loose("Text")],
        Check::Undoable,
    ),
    command(
        "slide.image",
        ".uno:ElySlideImage",
        &[Arg::Int("Slide", "long", 0, 65_534), Arg::Text("Url"), Arg::Loose("Width")],
        Check::Undoable,
    ),
    command(
        "slide.layout",
        ".uno:AssignLayout",
        &[Arg::Int("WhatLayout", "long", 0, 34)],
        Check::Undoable,
    ),
    command(
        "slide.size",
        ".uno:PageSize",
        &[
            Arg::Int("PageWidth", "long", 1000, 300_000),
            Arg::Int("PageHeight", "long", 1000, 300_000),
            Arg::Bool("ScaleObjects"),
        ],
        Check::Undoable,
    ),
    command(
        "slide.master",
        ".uno:PresentationLayout",
        &[Arg::Text("Master")],
        Check::Undoable,
    ),
    command(
        "slide.theme",
        ".uno:ThemeDialog",
        &[Arg::Text("ColorSet")],
        Check::Undoable,
    ),
    command("master.edit", ".uno:SlideMasterPage", &[], Check::Finished),
    command(
        "master.insert",
        ".uno:InsertMasterPage",
        &[],
        Check::Finished,
    ),
    command("master.close", ".uno:CloseMasterView", &[], Check::Finished),
    command(
        "show.chart",
        ".uno:InsertObjectChart",
        &[Arg::Text("Chart")],
        Check::Undoable,
    ),
    command(
        "slide.transition",
        ".uno:SetSlideTransition",
        &[Arg::Text("Transition")],
        Check::Undoable,
    ),
    command(
        "animation.add",
        ".uno:AddAnimation",
        &[Arg::Text("Animation")],
        Check::Undoable,
    ),
    command(
        "animation.change",
        ".uno:ChangeAnimation",
        &[Arg::Text("Animation")],
        Check::Undoable,
    ),
    command(
        "media.insert",
        ".uno:InsertAVMedia",
        &[
            Arg::Text("URL"),
            Arg::Bool("IsLink"),
            Arg::Int("Width", "long", 1, 1_000_000),
            Arg::Int("Height", "long", 1, 1_000_000),
        ],
        Check::Undoable,
    ),
    command(
        "notes.set",
        ".uno:SlideNotes",
        &[Arg::Loose("Text"), Arg::Int("Part", "long", 0, 9_999)],
        Check::Undoable,
    ),
    command(
        "remark.edit",
        ".uno:EditAnnotation",
        &[Arg::Text("Id"), Arg::Text("Text")],
        Check::Undoable,
    ),
    command(
        "remark.delete",
        ".uno:DeleteAnnotation",
        &[Arg::Text("Id")],
        Check::Undoable,
    ),
    // Impress answers false; text edit keeps its own undo.
    command(
        "show.link.insert",
        ".uno:SetHyperlink",
        &[Arg::Text("Hyperlink.Text"), Arg::Text("Hyperlink.URL")],
        Check::Finished,
    ),
];
