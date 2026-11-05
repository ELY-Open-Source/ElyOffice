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

/// Dev probe: input waits from this check; 0 is off.
static FROM: AtomicU64 = AtomicU64::new(0);
/// Input checks since the probe began.
static CHECKS: AtomicU64 = AtomicU64::new(0);

/// Whether a client waits for the lock now.
pub(super) fn any() -> bool {
    let from = FROM.load(Ordering::SeqCst);
    let probed = from > 0 && CHECKS.fetch_add(1, Ordering::SeqCst) + 1 >= from;
    WAITS.load(Ordering::SeqCst) > 0 || probed
}

/// Starts the dev probe: check `from` on finds input.
pub(super) fn probe(from: u64) {
    CHECKS.store(0, Ordering::SeqCst);
    FROM.store(from, Ordering::SeqCst);
}

/// Ends the dev probe; the checks it saw.
pub(super) fn unprobe() -> u64 {
    FROM.store(0, Ordering::SeqCst);
    CHECKS.load(Ordering::SeqCst)
}
