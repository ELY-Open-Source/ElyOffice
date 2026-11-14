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
    // Each engine's own folder, set by the app.
    let temp = match dunce::canonicalize(std::env::temp_dir()) {
        Ok(temp) => temp,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(io::Error::new(error.kind(), format!("temp dir unreadable: {error}"))),
    };
    // Never the system's temp folder.
    if !temp.starts_with(profile.join("tmp")) {
        return Err(io::Error::other(format!("temp dir {} is not the profile's", temp.display())));
    }
    let entries = std::fs::read_dir(&temp)
        .map_err(|error| io::Error::new(error.kind(), format!("temp dir {} unreadable: {error}", temp.display())))?;
    for entry in entries {
        let path = entry?.path();
        unprotect(&path);
        let gone = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        gone.map_err(|error| {
            let held = held(&path, &error);
            io::Error::new(error.kind(), format!("temp {} stays: {error}{held}", path.display()))
        })?;
    }
    std::fs::remove_dir(&temp)
        .map_err(|error| io::Error::new(error.kind(), format!("temp dir {} stays: {error}", temp.display())))?;
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

/// A teardown's limit after the app hangs up.
const CLOSE_GRACE: std::time::Duration = std::time::Duration::from_secs(10);

/// A hung teardown still ends the engine.
pub(super) fn deadline() {
    let spawned = std::thread::Builder::new().name("lok-deadline".into()).spawn(|| {
        std::thread::sleep(CLOSE_GRACE);
        log::warn!("lok-worker: teardown passed {} s; quitting", CLOSE_GRACE.as_secs());
        quit(1);
    });
    // No thread, no wait: quit now.
    if let Err(error) = spawned {
        log::warn!("lok-worker: no deadline thread: {error}");
        quit(1);
    }
}

/// Who holds `path`, when Windows says it is in use.
#[cfg(windows)]
fn held(path: &Path, error: &io::Error) -> String {
    match error.raw_os_error() {
        Some(32) => format!("; {}", super::holders::holders(path)),
        _ => String::new(),
    }
}

#[cfg(not(windows))]
fn held(_: &Path, _: &io::Error) -> String {
    String::new()
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
