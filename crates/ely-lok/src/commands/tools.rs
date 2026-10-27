//! Tool rows: find and replace, spelling, compare.

use super::{Arg, Check, Command, command};

/// Find one (0) or all (1), with the options.
const FIND: [Arg; 6] = [
    Arg::Loose("SearchItem.SearchString"),
    Arg::Bool("SearchItem.Backward"),
    Arg::Int("SearchItem.Command", "unsigned short", 0, 1),
    Arg::Int("SearchItem.TransliterateFlags", "long", 0, 256),
    Arg::Int("SearchItem.SearchFlags", "long", 0, 16),
    Arg::Int("SearchItem.AlgorithmType2", "short", 1, 2),
];

/// Replace one (2) or all (3), with the replacement.
const REPLACE: [Arg; 7] = [
    Arg::Loose("SearchItem.SearchString"),
    Arg::Bool("SearchItem.Backward"),
    Arg::Int("SearchItem.Command", "unsigned short", 2, 3),
    Arg::Int("SearchItem.TransliterateFlags", "long", 0, 256),
    Arg::Int("SearchItem.SearchFlags", "long", 0, 16),
    Arg::Int("SearchItem.AlgorithmType2", "short", 1, 2),
    Arg::Loose("SearchItem.ReplaceString"),
];

/// Replace all, exact and case-sensitive; the find box hears nothing.
const REPLACE_EXACT: [Arg; 8] = [
    Arg::Text("SearchItem.SearchString"),
    Arg::Bool("SearchItem.Backward"),
    Arg::Int("SearchItem.Command", "unsigned short", 3, 3),
    Arg::Int("SearchItem.TransliterateFlags", "long", 0, 0),
    Arg::Int("SearchItem.SearchFlags", "long", 0, 0),
    Arg::Int("SearchItem.AlgorithmType2", "short", 1, 1),
    Arg::Loose("SearchItem.ReplaceString"),
    Arg::Bool("Quiet"),
];

/// Find, replace, spelling and compare commands.
pub(super) const ROWS: &[Command] = &[
    command("find.run", ".uno:ExecuteSearch", &FIND, Check::Finished),
    command(
        "find.replace",
        ".uno:ExecuteSearch",
        &REPLACE,
        Check::Finished,
    ),
    command(
        "text.replace",
        ".uno:ExecuteSearch",
        &REPLACE_EXACT,
        Check::Undoable,
    ),
    command(
        "spell.apply",
        ".uno:SpellCheckApplySuggestion",
        &[Arg::Text("ApplyRule")],
        Check::Undoable,
    ),
    command(
        "spell.ignore_all",
        ".uno:SpellCheckIgnoreAll",
        &[Arg::Text("Type")],
        Check::Finished,
    ),
    command(
        "view.theme",
        ".uno:ChangeTheme",
        &[Arg::Text("NewTheme")],
        Check::Finished,
    ),
    command(
        "doc.compare",
        ".uno:CompareDocuments",
        &[Arg::Text("URL"), Arg::Bool("NoAcceptDialog")],
        Check::Result,
    ),
];
