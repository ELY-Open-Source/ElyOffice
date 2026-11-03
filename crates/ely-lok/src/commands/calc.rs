//! Spreadsheet rows: calculation, names, tracing, R batches.

use super::{Arg, Check, Command, command};

/// Calculation, named ranges, tracing, R (TABLE-03..06, 17).
pub(super) const ROWS: &[Command] = &[
    // A whole R batch in one undo step (0081).
    command("macro.apply", ".uno:EnterString", &[Arg::Read("ElyMacro")], Check::Result),
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
