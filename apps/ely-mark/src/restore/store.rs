//! The kept session's disk work, from any thread.

use std::{
    io,
    path::Path,
    sync::{Arc, Mutex},
};

/// Numbers session writes; a later one supersedes an earlier.
#[derive(Clone, Default)]
pub(super) struct Turns(Arc<Mutex<u64>>);

impl Turns {
    /// The next turn, taken on the window's thread.
    pub(super) fn take(&self) -> u64 {
        let mut last = self.0.lock().expect("the turn lock");
        *last += 1;
        *last
    }

    /// Writes unless a later turn came; None when it did.
    pub(super) fn write(&self, turn: u64, file: &Path, bytes: &[u8]) -> Option<io::Result<()>> {
        let last = self.0.lock().expect("the turn lock");
        (*last == turn).then(|| ely_files::save_atomic(file, bytes, 0).map(drop))
    }
}

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

#[cfg(test)]
mod tests {
    #[test]
    fn a_later_turn_supersedes_an_earlier() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let file = dir.path().join("session.json");
        let turns = super::Turns::default();
        let (early, late) = (turns.take(), turns.take());
        assert!(turns.write(early, &file, b"old").is_none(), "the early write skips");
        assert!(!file.exists());
        turns.write(late, &file, b"new").expect("the newest writes").expect("it lands");
        assert_eq!(std::fs::read(&file).expect("reads"), b"new");
    }
}
