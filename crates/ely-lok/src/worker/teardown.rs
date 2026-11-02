use std::{io, path::Path};

use super::{LOK, docs, quit};

/// Frees documents, destroys the kit, and reports what failed (LOK-10).
pub(super) fn teardown_work() -> io::Result<()> {
    let mut docs = docs().lock().expect("the docs lock");
    crate::events::stop_all(&docs);
    docs.keys().for_each(|id| super::closing::forget(*id));
    docs.clear();
    let kit = LOK.swap(std::ptr::null_mut(), std::sync::atomic::Ordering::SeqCst);
    if !kit.is_null() {
        let kit = unsafe { Box::from_raw(kit) };
        let joined = kit.join_threads();
        log::info!("lok-worker: threads joined={joined}");
        drop(kit);
    }
    let Some(profile) = PROFILE.get() else {
        return Ok(());
    };
    let lock = profile.join(".lock");
    match std::fs::remove_file(&lock) {
        Ok(()) => log::info!("lok-worker: stale profile lock removed"),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(io::Error::new(
                error.kind(),
                format!("profile lock stays: {error}"),
            ));
        }
    }
    let temp = profile.join("tmp");
    let entries = match std::fs::read_dir(&temp) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(io::Error::new(
                error.kind(),
                format!("temp dir unreadable: {error}"),
            ));
        }
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        if !name.to_string_lossy().starts_with("lu") {
            continue;
        }
        let path = entry.path();
        unprotect(&path);
        let gone = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        gone.map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("temp {} stays: {error}", path.display()),
            )
        })?;
    }
    log::info!("lok-worker: torn down");
    Ok(())
}

/// Tears down and exits with the work's verdict.
pub(super) fn teardown(code: i32) -> ! {
    let outcome = teardown_work();
    if let Err(error) = &outcome {
        log::warn!("lok-worker: teardown failed: {error}");
    }
    quit(if outcome.is_ok() { code } else { 1 });
}

/// Clears macOS's immutable flag on a kernel temp file.
#[cfg(target_os = "macos")]
fn unprotect(path: &Path) {
    use std::os::unix::ffi::OsStrExt;
    let Some(c) = std::ffi::CString::new(path.as_os_str().as_bytes()).ok() else {
        return;
    };
    unsafe { libc::chflags(c.as_ptr(), 0) };
}

#[cfg(not(target_os = "macos"))]
fn unprotect(_: &Path) {}

/// The isolated profile directory, for the stale lock at exit.
pub(super) static PROFILE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
