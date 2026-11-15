//! Ends a stuck worker from outside its lock.

use std::{
    io,
    sync::{Arc, MutexGuard, OnceLock},
};

use super::Worker;
use crate::endpoint::Peer;

/// The worker's process, or the taken engine's peer.
#[derive(Clone)]
pub struct Stopper {
    process: ely_ipc::Stopper,
    /// Set once an idle engine is taken.
    peer: Arc<OnceLock<Arc<Peer>>>,
}

impl Stopper {
    pub(super) fn new() -> Self {
        Self { process: ely_ipc::Stopper::default(), peer: Arc::default() }
    }

    /// An idle engine was taken; it ends by its peer.
    pub(super) fn adopted(&self, peer: Arc<Peer>) {
        if self.peer.set(peer).is_err() {
            log::error!("lok: a worker was taken twice");
        }
    }

    /// The spawn returned this process; `held` is the gate.
    pub(super) fn started(&self, held: &MutexGuard<'_, bool>, pid: u32) -> io::Result<()> {
        self.process.started(held, pid)
    }

    /// Kills the process unless something reaped it first.
    pub fn stop(&self) -> io::Result<()> {
        if let Some(peer) = self.peer.get() {
            log::warn!("lok: the taken engine is stopped while it works");
            return peer.end();
        }
        self.process.stop()
    }

    /// Held across a reap: a reaped id may be reused.
    pub(crate) fn gate(&self) -> MutexGuard<'_, bool> {
        self.process.gate()
    }
}

impl Worker {
    /// This build and profile again, unchecked; taken ones take again.
    pub fn relaunch(&self) -> io::Result<Self> {
        let (install, profile, dev) = &self.start;
        if let Some(app) = &self.idle {
            return Self::take(install, profile, app, self.probe);
        }
        let mut worker = Self::spawn_mode(install, profile, *dev, None)?;
        worker.probe = self.probe;
        Ok(worker)
    }

    /// Ends this worker's process without this worker's lock.
    pub fn stopper(&self) -> Stopper {
        self.stopper.clone()
    }

    /// Why the worker process ended, if it has.
    pub fn ended(&mut self) -> Option<String> {
        self.child.ended(&self.stopper)
    }
}
