//! Kernel dialogs, closed once the kernel's callback flush returns.

use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::ffi::Raw;

#[cfg(windows)]
use super::messages::nudge;
#[cfg(target_os = "linux")]
use super::unipoll::nudge;

/// Dialogs to close: document id, handle, window.
static DUE: Mutex<Vec<(u64, Raw, u32)>> = Mutex::new(Vec::new());

/// The queue; a panic cannot leave it half changed.
fn due() -> MutexGuard<'static, Vec<(u64, Raw, u32)>> {
    DUE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Called inside a kernel callback, where closing re-enters its flush.
pub(crate) fn later(id: u64, raw: Raw, window: u32) {
    due().push((id, raw, window));
    nudge();
}

/// Closes due windows; requests may hold the docs lock.
pub(super) fn now() {
    let ready = std::mem::take(&mut *due());
    for (_, raw, window) in ready {
        crate::ffi::close_window(raw, window);
    }
}

/// Drops the dialogs of a document about to be freed.
pub(super) fn forget(id: u64) {
    due().retain(|(due, ..)| {
        let keep = *due != id;
        if !keep {
            log::info!("lok-worker: a dialog of document {id} closed with it");
        }
        keep
    });
}

#[cfg(target_os = "macos")]
fn nudge() {
    extern "C" fn due(_: *mut std::ffi::c_void) {
        now();
    }
    unsafe {
        super::gcd::dispatch_async_f(super::gcd::main_queue(), std::ptr::null_mut(), due);
    }
}
