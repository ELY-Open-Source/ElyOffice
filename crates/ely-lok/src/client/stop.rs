//! Ends a stuck worker from outside its lock.

use std::{
    io,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use super::Worker;

/// The worker's process id, good until anything reaps it.
#[derive(Clone)]
pub struct Stopper {
    pid: u32,
    reaped: Arc<Mutex<bool>>,
}

impl Stopper {
    pub(super) fn new(pid: u32) -> Self {
        Self { pid, reaped: Arc::new(Mutex::new(false)) }
    }

    /// Kills the process unless something reaped it first.
    pub fn stop(&self) -> io::Result<()> {
        // Held while killing, so no reap slips between.
        let reaped = self.gate();
        if *reaped {
            return Ok(());
        }
        log::warn!("lok: the worker {} is stopped while it works", self.pid);
        kill(self.pid)
    }

    /// Held across a reap: a reaped id may be reused.
    pub(crate) fn gate(&self) -> MutexGuard<'_, bool> {
        // A bool cannot be left half written.
        self.reaped.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Worker {
    /// Ends this worker's process without this worker's lock.
    pub fn stopper(&self) -> Stopper {
        self.stopper.clone()
    }

    /// Why the worker process ended, if it has.
    pub fn ended(&mut self) -> Option<String> {
        let mut reaped = self.stopper.gate();
        match self.child.try_wait() {
            Ok(Some(status)) => {
                *reaped = true;
                Some(status.to_string())
            }
            Ok(None) => None,
            Err(error) => Some(error.to_string()),
        }
    }
}

#[cfg(unix)]
fn kill(pid: u32) -> io::Result<()> {
    let pid = i32::try_from(pid).map_err(|_| io::Error::other(format!("process id {pid} is out of range")))?;
    if unsafe { libc::kill(pid, libc::SIGKILL) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(windows)]
fn kill(pid: u32) -> io::Result<()> {
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
