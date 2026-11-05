//! Clients waiting for the worker lock; snapshots yield to them.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::ipc::{Unpack, tag};

/// Client threads blocked on the lock, as their notes tell.
static WAITS: AtomicU64 = AtomicU64::new(0);

/// Takes a wait note while reading; false for requests.
pub(super) fn noted(asked: u8, payload: &[u8]) -> bool {
    if asked != tag::WAIT {
        return false;
    }
    match Unpack::new(payload).u64() {
        Ok(1) => {
            WAITS.fetch_add(1, Ordering::SeqCst);
        }
        Ok(0) => {
            let left = WAITS.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |waits| waits.checked_sub(1));
            if left.is_err() {
                log::error!("lok-worker: a wait ended that never began");
            }
        }
        _ => log::error!("lok-worker: a bad wait note"),
    }
    true
}

/// Whether a client waits for the lock now.
pub(super) fn any() -> bool {
    WAITS.load(Ordering::SeqCst) > 0
}
