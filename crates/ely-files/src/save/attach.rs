//! Attachments copied or written into a folder.

use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

use super::{numbered_name, with_path};

/// Copies an attachment in; the source is never touched (FILE-07).
pub fn import_attachment(source: &Path, dir: &Path) -> io::Result<PathBuf> {
    let name = source.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} has no file name", source.display()),
        )
    })?;
    let mut from = std::fs::File::open(source).map_err(|e| with_path(source, e))?;
    let target = attach(dir, Path::new(name), |to| io::copy(&mut from, to).map(drop))?;
    log::info!("files: imported {} to {}", source.display(), target.display());
    Ok(target)
}

/// Writes `bytes` into `dir` as `name`, or a free variant.
pub fn write_attachment(name: &str, bytes: &[u8], dir: &Path) -> io::Result<PathBuf> {
    let target = attach(dir, Path::new(name), |to| to.write_all(bytes))?;
    log::info!("files: wrote {} bytes to {}", bytes.len(), target.display());
    Ok(target)
}

/// One file name that stays inside its folder.
fn plain_name(name: &Path) -> bool {
    let mut parts = name.components();
    let single = matches!((parts.next(), parts.next()), (Some(std::path::Component::Normal(_)), None));
    single && !(cfg!(windows) && windows_special(&name.to_string_lossy()))
}

/// A stream suffix or a device name on Windows.
fn windows_special(name: &str) -> bool {
    let base = name.split('.').next().unwrap_or_default().trim_end().to_ascii_uppercase();
    let numbered = |prefix: &str| {
        base.strip_prefix(prefix)
            .is_some_and(|n| n.len() == 1 && n.as_bytes()[0].is_ascii_digit() && n != "0")
    };
    name.contains(':') || ["CON", "PRN", "AUX", "NUL"].contains(&base.as_str()) || numbered("COM") || numbered("LPT")
}

/// Claims a free name like `name`, then fills it.
fn attach(
    dir: &Path,
    name: &Path,
    fill: impl FnOnce(&mut std::fs::File) -> io::Result<()>,
) -> io::Result<PathBuf> {
    if !plain_name(name) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a plain file name", name.display()),
        ));
    }
    std::fs::create_dir_all(dir).map_err(|e| with_path(dir, e))?;
    let stem = name.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let ext = name
        .extension()
        .map(|ext| ext.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut target = dir.join(name);
    let mut counter = 1;
    // Claims the name; another writer's file is never replaced.
    let mut to = loop {
        match std::fs::File::create_new(&target) {
            Ok(file) => break file,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                target = dir.join(numbered_name(&stem, counter, &ext));
                counter += 1;
            }
            Err(e) => return Err(with_path(&target, e)),
        }
    };
    let filled = fill(&mut to).and_then(|()| to.flush());
    if let Err(e) = filled {
        // A half copy is not left behind.
        std::fs::remove_file(&target).ok();
        return Err(with_path(&target, e));
    }
    Ok(target)
}
