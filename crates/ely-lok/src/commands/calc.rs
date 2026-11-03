//! Spreadsheet rows: calculation, names, error tracing.

use super::{Arg, Check, Command, command};

/// Calculation, named ranges and tracing (TABLE-03..06).
pub(super) const ROWS: &[Command] = &[
    command("macro.apply", ".uno:EnterString", &[Arg::Text("ElyMacro")], Check::Result),
    command(
        "calc.auto",
        ".uno:AutomaticCalculation",
        &[Arg::Bool("AutomaticCalculation")],
        Check::Finished,
    ),
    command("calc.now", ".uno:Calculate", &[], Check::Finished),
    command("calc.all", ".uno:CalculateHard", &[], Check::Finished),
    command(
        "names.define",
        ".uno:DefineName",
        &[Arg::Text("Name"), Arg::Text("Symbol"), Arg::Loose("Scope")],
        Check::Undoable,
    ),
    command(
        "names.remove",
        ".uno:DefineName",
        &[Arg::Text("Name"), Arg::Loose("Scope")],
        Check::Undoable,
    ),
    command("trace.error", ".uno:ShowErrors", &[], Check::Undoable),
    command(
        "trace.precedents",
        ".uno:ShowPrecedents",
        &[],
        Check::Undoable,
    ),
    command(
        "trace.dependents",
        ".uno:ShowDependents",
        &[],
        Check::Undoable,
    ),
    command("trace.clear", ".uno:ClearArrows", &[], Check::Undoable),
];
