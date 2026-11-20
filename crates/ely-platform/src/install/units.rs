//! Linux: an AppImage's engine unit that hides the package's.

use std::{
    io,
    path::{Path, PathBuf},
};

/// The package's unit name; the home copy wins (systemd.unit(5)).
pub const SHADOW: &str = "ely-engine@.service";

/// AppRun's own line: `/usr/bin/env "<image>.AppImage" %i engine`.
pub fn appimage_written(unit: &str) -> bool {
    unit.lines().any(|line| {
        let quoted = line.strip_prefix("ExecStart=/usr/bin/env \"").and_then(|rest| rest.strip_suffix("\" %i engine"));
        quoted.is_some_and(|image| image.ends_with(".AppImage"))
    })
}

/// The user's systemd unit folder, as AppRun finds it.
pub fn home_units() -> io::Result<PathBuf> {
    let config = match crate::env::value("XDG_CONFIG_HOME").filter(|dir| !dir.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(crate::env::value("HOME").ok_or_else(|| io::Error::other("there is no HOME"))?).join(".config"),
    };
    Ok(config.join("systemd").join("user"))
}

/// What the read found, and so what was done.
#[derive(Debug, PartialEq)]
pub enum Found {
    /// No unit under the package's name; no reload.
    Nothing,
    /// Someone else's unit, kept; no reload.
    Kept,
    /// An AppImage's: these paths removed here, then one reload.
    Cleared(Vec<PathBuf>),
}

/// Removes an AppImage's shadowing unit and its links; reloads once.
pub fn unshadow(units: &Path, reload: impl FnOnce() -> io::Result<()>) -> io::Result<Found> {
    let shadow = units.join(SHADOW);
    let text = match std::fs::read_to_string(&shadow) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            log::info!("engine: no {} hides the package's unit", shadow.display());
            return Ok(Found::Nothing);
        }
        Err(error) => return Err(io::Error::new(error.kind(), format!("{}: {error}", shadow.display()))),
    };
    if !appimage_written(&text) {
        log::warn!("engine: {} hides the package's unit but no AppImage wrote it; kept", shadow.display());
        return Ok(Found::Kept);
    }
    clear(units, shadow, reload).map(Found::Cleared)
}

/// Seen at the start: reloads even if another removed all.
fn clear(units: &Path, shadow: PathBuf, reload: impl FnOnce() -> io::Result<()>) -> io::Result<Vec<PathBuf>> {
    let mut removed = Vec::new();
    let wants = units.join("graphical-session.target.wants");
    let entries = match std::fs::read_dir(&wants) {
        Ok(entries) => entries.collect::<io::Result<Vec<_>>>()?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(io::Error::new(error.kind(), format!("{}: {error}", wants.display()))),
    };
    for link in entries.into_iter().map(|entry| entry.path()) {
        if std::fs::read_link(&link).is_ok_and(|to| to == shadow) && remove(&link)? {
            removed.push(link);
        }
    }
    if remove(&shadow)? {
        removed.push(shadow);
    }
    reload()?;
    Ok(removed)
}

/// False when another app, starting too, removed it first.
fn remove(path: &Path) -> io::Result<bool> {
    match std::fs::remove_file(path) {
        Ok(()) => {
            log::info!("engine: removed {}, an AppImage's unit hiding the package's", path.display());
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            log::info!("engine: {} was already removed by another start", path.display());
            Ok(false)
        }
        Err(error) => Err(io::Error::new(error.kind(), format!("{}: {error}", path.display()))),
    }
}

#[cfg(test)]
mod tests;
