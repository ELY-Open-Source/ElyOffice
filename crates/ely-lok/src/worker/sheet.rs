//! Calc undo and redo: the sheet or the edited cell.

use std::io;

use super::input::history_count;
use crate::commands::Command;
use crate::ffi::Document;
use crate::ipc;

/// Served only by Calc's cell edit shell, never disabled.
const EDITING: &str = ".uno:InsertFieldSheet";

/// Which of `commands` the kernel runs now (patch 0045).
fn runs(doc: &Document, commands: &[&str]) -> io::Result<crate::menu::Runs> {
    let asked = doc.command_values(&format!("{}?commands={}", crate::menu::STATES, commands.join(",")))?;
    crate::menu::read_runs(&asked, commands)
}

/// Confirms by the history the kernel acts on.
pub(super) fn history(doc: &Document, command: &Command, args: Option<&str>) -> io::Result<()> {
    let refuse = |why: String| Err(io::Error::other(format!("{}: {why}", command.id)));
    let before = runs(doc, &[command.uno, EDITING])?;
    if !before.allows(command.uno) {
        return Err(io::Error::other(format!("{}{}: the kernel has no step", ipc::NOTHING, command.id)));
    }
    if before.allows(EDITING) {
        let other = if command.uno == ".uno:Undo" { ".uno:Redo" } else { ".uno:Undo" };
        doc.post_uno(command.uno, args)?;
        // The edit's step moved to the other stack.
        if !runs(doc, &[other])?.allows(other) {
            return refuse(format!("the cell edit offers no {other} after it"));
        }
        log::info!("lok-worker: {} in a cell edit", command.id);
        return Ok(());
    }
    let history = format!("{}Count", command.uno);
    let count = history_count(doc, &history)?;
    doc.post_uno(command.uno, args)?;
    match history_count(doc, &history)? {
        after if after + 1 == count => Ok(()),
        after => refuse(format!("{history} went {count} to {after}")),
    }
}
