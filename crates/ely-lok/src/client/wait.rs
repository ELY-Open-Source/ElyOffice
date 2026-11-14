//! A request waiting for the worker tells it, lock-free.

use std::{
    io::{self, BufWriter},
    sync::{Arc, Mutex, MutexGuard},
};

use super::Worker;
use crate::endpoint::Half;
use crate::ipc::{self, Pack, tag};

/// Requests; None until connected.
pub(super) type Pipe = Arc<Mutex<Option<BufWriter<Half>>>>;

/// Writes one frame; a poisoned pipe is a broken one.
pub(super) fn send(pipe: &Pipe, request: u8, id: u64, payload: &[u8]) -> io::Result<()> {
    let mut pipe = pipe.lock().map_err(|_| io::Error::other("a frame write panicked"))?;
    let Some(pipe) = pipe.as_mut() else {
        return Err(io::Error::new(io::ErrorKind::NotConnected, "the lok worker is not connected"));
    };
    ipc::write_frame(pipe, request, id, payload)
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
        // Nothing runs in a worker not yet connected.
        if self.0.lock().is_ok_and(|pipe| pipe.is_none()) {
            return;
        }
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
        Waiter(Arc::clone(&self.requests))
    }

    /// Waits once for the worker's connection.
    pub(super) fn wire(&mut self) -> io::Result<()> {
        let Some(wiring) = self.wiring.take() else {
            return Ok(());
        };
        let write = wiring
            .recv()
            .map_err(|_| io::Error::other("the lok worker's spawn ended before it connected"))??;
        let mut requests = self.requests.lock().map_err(|_| io::Error::other("a frame write panicked"))?;
        *requests = Some(io::BufWriter::new(write));
        Ok(())
    }
}
