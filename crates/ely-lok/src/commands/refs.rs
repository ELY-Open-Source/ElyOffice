//! Reference and review rows: notes, marks, formulas, comments.

use super::{Arg, Check, Command, command};

/// Reference, comment and tracked-change commands.
pub(super) const ROWS: &[Command] = &[
    command("note.footnote", ".uno:InsertFootnote", &[], Check::Undoable),
    command("note.endnote", ".uno:InsertEndnote", &[], Check::Undoable),
    command(
        "bookmark.insert",
        ".uno:InsertBookmark",
        &[Arg::Text("Bookmark")],
        Check::Undoable,
    ),
    command("link.jump", ".uno:JumpToMark", &[Arg::Text("Bookmark")], Check::Finished),
    command(
        "link.insert",
        ".uno:SetHyperlink",
        &[Arg::Text("Hyperlink.Text"), Arg::Text("Hyperlink.URL")],
        Check::Undoable,
    ),
    command(
        "reference.insert",
        ".uno:InsertField",
        &[
            Arg::Int("Type", "unsigned short", 13, 13),
            Arg::Int("SubType", "unsigned short", 2, 2),
            Arg::Text("Name"),
            Arg::Int("Format", "uint32", 0, 3),
        ],
        Check::Undoable,
    ),
    command(
        "index.insert",
        ".uno:InsertMultiIndex",
        &[Arg::Text("Title")],
        Check::Undoable,
    ),
    command(
        "index.update",
        ".uno:UpdateAllIndexes",
        &[],
        Check::Undoable,
    ),
    command(
        "caption.insert",
        ".uno:InsertCaptionDialog",
        &[Arg::Text("Category"), Arg::Text("Text")],
        Check::Undoable,
    ),
    command(
        "formula.insert",
        ".uno:InsertObjectStarMath",
        &[Arg::Text("Formula")],
        Check::Undoable,
    ),
    command(
        "comment.insert",
        ".uno:InsertAnnotation",
        &[Arg::Text("Text")],
        Check::Undoable,
    ),
    command(
        "comment.reply",
        ".uno:ReplyComment",
        &[Arg::Text("Id"), Arg::Text("Text")],
        Check::Undoable,
    ),
    command("track.toggle", ".uno:TrackChanges", &[], Check::Result),
    command(
        "track.accept",
        ".uno:AcceptTrackedChange",
        &[Arg::Int("AcceptTrackedChange", "uint32", 1, 4_294_967_295)],
        Check::Undoable,
    ),
    command(
        "track.reject",
        ".uno:RejectTrackedChange",
        &[Arg::Int("RejectTrackedChange", "uint32", 1, 4_294_967_295)],
        Check::Undoable,
    ),
    command(
        "track.accept_all",
        ".uno:AcceptAllTrackedChanges",
        &[],
        Check::Undoable,
    ),
    command(
        "track.reject_all",
        ".uno:RejectAllTrackedChanges",
        &[],
        Check::Undoable,
    ),
];
