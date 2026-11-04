//! Points, the index and slot folders on disk.

use std::{
    io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use ely_files::{Recovery, RecoveryKey};
use ely_platform::settings::Store;

use super::Kept;

/// A slot's points, under the limits in `dir`.
pub(super) fn recovery_in(dir: &Path, folder: PathBuf, sealed: bool) -> io::Result<Recovery> {
    let limits = ely_platform::storage::limits_in(&Store::of(dir))?;
    let recovery = match sealed {
        true => Recovery::sensitive(folder, RecoveryKey::load().key),
        false => Recovery::new(folder),
    };
    Ok(recovery.with_limits(limits.keep, limits.megabytes * 1024 * 1024))
}

/// The refused slots that fit their cap now.
pub(super) fn fitting(dir: &Path, refused: Vec<(u64, PathBuf, bool, u64)>) -> Vec<(u64, u64)> {
    let fits = |(id, folder, sealed, size): (u64, PathBuf, bool, u64)| match recovery_in(dir, folder, sealed).and_then(|recovery| recovery.fits(size)) {
        Ok(()) => Some((id, size)),
        Err(error) if error.kind() == io::ErrorKind::FileTooLarge => None,
        Err(error) => {
            log::error!("recovery: tab {id} limits unread: {error}");
            None
        }
    };
    refused.into_iter().filter_map(fits).collect()
}

/// Seals and writes one point; no worker needed.
pub(super) fn write_point(dir: &Path, folder: PathBuf, sealed: bool, bytes: &[u8]) -> io::Result<()> {
    recovery_in(dir, folder, sealed)?.write_once(bytes, None, SystemTime::now()).map(|_| ())
}

/// Staged then renamed: a failed write keeps the last index.
pub(super) fn write_index(path: &Path, bytes: &[u8]) -> io::Result<()> {
    std::fs::create_dir_all(path.parent().expect("the index has a folder"))?;
    ely_files::save_atomic(path, bytes, 0).map(|_| ())
}

pub(super) fn read_index(dir: &Path) -> io::Result<Vec<Kept>> {
    match std::fs::read(dir.join("recovery").join("index.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

/// Removes every slot folder but those `held`.
pub(super) fn prune(recovery: &Path, held: &[Kept]) -> io::Result<()> {
    if held.is_empty() {
        return remove(recovery);
    }
    for entry in std::fs::read_dir(recovery)? {
        let entry = entry?;
        let name = entry.file_name();
        if entry.file_type()?.is_dir() && !held.iter().any(|kept| *kept.slot == *name) {
            remove(&entry.path())?;
        }
    }
    Ok(())
}

pub(super) fn remove(path: &Path) -> io::Result<()> {
    match std::fs::remove_dir_all(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}
