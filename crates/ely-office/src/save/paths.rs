//! Paths as the file system names them.

use std::{
    io,
    path::{Path, PathBuf},
};

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
