//! Ends a stuck worker from outside its lock.

use std::{
    io,
    sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError},
};

use super::Worker;
use crate::endpoint::Peer;

/// The worker's process id, good until anything reaps it.
#[derive(Clone)]
pub struct Stopper {
    /// Set once the spawn returns.
    pid: Arc<OnceLock<u32>>,
    /// Set once an idle engine is taken.
    peer: Arc<OnceLock<Arc<Peer>>>,
    reaped: Arc<Mutex<bool>>,
}

impl Stopper {
    pub(super) fn new() -> Self {
        Self { pid: Arc::default(), peer: Arc::default(), reaped: Arc::new(Mutex::new(false)) }
    }

    /// An idle engine was taken; it ends by its peer.
    pub(super) fn adopted(&self, peer: Arc<Peer>) {
        if self.peer.set(peer).is_err() {
            log::error!("lok: a worker was taken twice");
        }
    }

    /// The spawn returned this process.
    pub(super) fn started(&self, pid: u32) {
        if self.pid.set(pid).is_err() {
            log::error!("lok: a worker's process started twice");
        }
    }

    /// Kills the process unless something reaped it first.
    pub fn stop(&self) -> io::Result<()> {
        if let Some(peer) = self.peer.get() {
            log::warn!("lok: the taken engine is stopped while it works");
            return peer.end();
        }
        // Held while killing, so no reap slips between.
        let reaped = self.gate();
        // The spawn held the gate; no id means it failed.
        let Some(&pid) = self.pid.get().filter(|_| !*reaped) else {
            return Ok(());
        };
        log::warn!("lok: the worker {pid} is stopped while it works");
        kill(pid)
    }

    /// Held across a reap: a reaped id may be reused.
    pub(crate) fn gate(&self) -> MutexGuard<'_, bool> {
        // A bool cannot be left half written.
        self.reaped.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Worker {
    /// A new worker on this one's build and profile, unchecked.
    pub fn relaunch(&self) -> io::Result<Self> {
        let (install, profile, dev) = &self.start;
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

#[cfg(unix)]
pub(super) fn kill(pid: u32) -> io::Result<()> {
    let pid = i32::try_from(pid).map_err(|_| io::Error::other(format!("process id {pid} is out of range")))?;
    if unsafe { libc::kill(pid, libc::SIGKILL) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(windows)]
pub(super) fn kill(pid: u32) -> io::Result<()> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};
    let handle = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
    if handle.is_null() {
        return Err(io::Error::last_os_error());
    }
    let ended = unsafe { TerminateProcess(handle, 1) } != 0;
    let error = io::Error::last_os_error();
    unsafe { CloseHandle(handle) };
    if !ended {
        return Err(error);
    }
    Ok(())
}
