//! The kept session's disk work, from any thread.

use std::{io, path::Path};

/// Removes every slot folder in `dir` not in `live`.
fn sweep(dir: &Path, live: &[&str]) -> io::Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        entries => entries?,
    };
    for entry in entries {
        let entry = entry?;
        if !live.contains(&entry.file_name().to_string_lossy().as_ref()) {
            super::clear(&entry.path())?;
        }
    }
    Ok(())
}

/// Sweeps the data folder's points; a failure only logs.
pub(super) fn sweep_logged(dir: &Path, live: &[String]) {
    let live: Vec<&str> = live.iter().map(String::as_str).collect();
    if let Err(error) = sweep(&dir.join("recovery"), &live) {
        log::error!("mark: old recovery points stay: {error}");
    }
}
