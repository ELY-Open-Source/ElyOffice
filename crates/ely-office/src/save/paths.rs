//! Paths as the file system names them.

use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{Engine, Tab};

/// The file a path names, links resolved.
pub(super) fn identity(path: &Path) -> io::Result<PathBuf> {
    let real = match std::fs::canonicalize(path) {
        Ok(real) => real,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let parent = path.parent().unwrap_or(Path::new("."));
            let name = path
                .file_name()
                .ok_or_else(|| io::Error::other("no file name"))?;
            std::fs::canonicalize(parent)?.join(name)
        }
        Err(error) => return Err(error),
    };
    // Whole path only: a name may need `\\?\`.
    Ok(dunce::simplified(&real).to_path_buf())
}

/// The tab whose running Save As writes `path`.
pub(crate) fn writer<E, G: Engine>(tabs: &[Tab<E, G>], path: &Path) -> Option<u64> {
    tabs.iter().find_map(|tab| match ely_files::same_file(tab.session.writing()?, path) {
        Ok(same) => same.then_some(tab.id),
        // A tab's gone volume is not this file.
        Err(error) => {
            log::warn!("office: tab {}'s target cannot be compared: {}", tab.id, ely_ipc::Worded::logged(&error));
            None
        }
    })
}

/// Where a panel for `tab` opens: beside it, else Documents.
pub fn folder<E, G: Engine>(tab: &Tab<E, G>) -> Option<PathBuf> {
    tab.session
        .path()
        .and_then(|path| path.parent().map(PathBuf::from))
        .or_else(|| {
            directories::UserDirs::new()?
                .document_dir()
                .map(PathBuf::from)
        })
}
