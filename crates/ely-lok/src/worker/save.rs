//! Saves and exports (LOK-09); snapshots give way to input.

use std::io;

use super::{lok, with_doc};
use crate::ipc::{self, Unpack};

/// The kernel's word for a snapshot it stopped.
const KERNEL_STOPPED: &str = "ely: stopped for input";

/// Saves a document under a new url and format (LOK-09).
pub(super) fn save(unpack: &mut Unpack) -> io::Result<()> {
    let id = unpack.u64()?;
    let url = unpack.str()?;
    let format = unpack.str()?;
    let options = unpack.str()?;
    if !matches!(options.as_str(), "" | "Snapshot" | "TakeOwnership" | "TakeOwnership,TerminateEditing") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unknown save option",
        ));
    }
    with_doc(id, |doc| doc.save_as(&url, &format, &options))?.map_err(|error| match lok().error() {
        // Patch 0090 names the stop; the client reads it.
        said if said == KERNEL_STOPPED => {
            log::info!("lok-worker: a snapshot gave way to input");
            io::Error::other(ipc::STOPPED)
        }
        said => io::Error::other(format!("{error}: {said}")),
    })?;
    log::info!("lok-worker: saved as {format} to {url} ({options})");
    Ok(())
}

/// Writes a PDF from typed options, never raw text.
pub(super) fn export_pdf(unpack: &mut Unpack) -> io::Result<()> {
    let (id, url) = (unpack.u64()?, unpack.str()?);
    let (from, to) = (unpack.u64()?, unpack.u64()?);
    let (comments, changes) = (unpack.u64()? != 0, unpack.u64()? != 0);
    let handouts = unpack.u64()?;
    let flag = |on: bool| serde_json::json!({ "type": "boolean", "value": on.to_string() });
    let mut options = serde_json::json!({
        "ExportNotes": flag(false),
        "ExportNotesInMargin": flag(comments),
        "ExportTrackedChanges": flag(changes),
    });
    if handouts != 0 {
        let per = handouts.to_string();
        options["ExportHandouts"] = serde_json::json!({ "type": "long", "value": per });
    }
    if from != 0 {
        if from > to {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a page range ends first",
            ));
        }
        let range = format!("{from}-{to}");
        options["PageRange"] = serde_json::json!({ "type": "string", "value": range });
    }
    with_doc(id, |doc| doc.save_as(&url, "pdf", &options.to_string()))??;
    log::info!(
        "lok-worker: exported PDF pages {from}-{to}, comments {comments}, changes {changes}, handouts {handouts}"
    );
    Ok(())
}
