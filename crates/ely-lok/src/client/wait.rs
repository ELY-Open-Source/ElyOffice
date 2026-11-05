//! A request waiting for the worker tells it, lock-free.

use std::{
    io::{self, BufWriter},
    process::ChildStdin,
    sync::{Arc, Mutex, MutexGuard},
};

use super::Worker;
use crate::ipc::{self, Pack, tag};

/// The request pipe, shared by the worker and its waiters.
pub(super) type Pipe = Arc<Mutex<BufWriter<ChildStdin>>>;

/// Writes one frame; a poisoned pipe is a broken one.
pub(super) fn send(pipe: &Pipe, request: u8, id: u64, payload: &[u8]) -> io::Result<()> {
    let mut pipe = pipe.lock().map_err(|_| io::Error::other("a frame write panicked"))?;
    ipc::write_frame(&mut *pipe, request, id, payload)
}

/// Tells a busy worker that a request waits for it.
#[derive(Clone)]
pub struct Waiter(Pipe);

impl Waiter {
    /// The worker, held; a snapshot in the way stops.
    pub fn hold<'a>(&self, worker: &'a Mutex<Worker>) -> MutexGuard<'a, Worker> {
        if let Ok(held) = worker.try_lock() {
            return held;
        }
        self.note(true);
        let held = worker.lock();
        self.note(false);
        held.expect("the worker lock")
    }

    pub(super) fn note(&self, waits: bool) {
        let note = Pack::new().u64(u64::from(waits)).bytes();
        // A dead worker fails the request that follows.
        if let Err(error) = send(&self.0, tag::WAIT, 0, &note) {
            log::warn!("lok: the wait note did not go: {error}");
        }
    }
}

impl Worker {
    /// A waiter on this worker's request pipe.
    pub fn waiter(&self) -> Waiter {
        Waiter(Arc::clone(&self.stdin))
    }
}
