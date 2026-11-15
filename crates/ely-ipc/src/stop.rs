//! Ends a worker process from outside its lock.

use std::{
    io,
    sync::{
        Arc, Mutex, MutexGuard, OnceLock, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
};

/// A worker's process id, good until anything reaps it.
#[derive(Clone, Default)]
pub struct Stopper {
    /// Set once the spawn returns.
    pid: Arc<OnceLock<u32>>,
    reaped: Arc<Mutex<bool>>,
    /// A stop came; read and set under the gate.
    asked: Arc<AtomicBool>,
}

impl Stopper {
    /// The spawn returned this process; `held` is the gate.
    pub fn started(&self, held: &MutexGuard<'_, bool>, pid: u32) -> io::Result<()> {
        if self.pid.set(pid).is_err() {
            log::error!("ipc: a worker's process {pid} started twice");
        }
        // A stop before the id still ends it.
        if !**held && self.asked.load(Ordering::Acquire) {
            log::warn!("ipc: the worker {pid} was stopped before it started");
            return kill(pid);
        }
        Ok(())
    }

    /// Kills the process unless something reaped it first.
    pub fn stop(&self) -> io::Result<()> {
        // Held while killing, so no reap slips between.
        let reaped = self.gate();
        self.asked.store(true, Ordering::Release);
        // No id yet: `started` kills it.
        let Some(&pid) = self.pid.get().filter(|_| !*reaped) else {
            return Ok(());
        };
        log::warn!("ipc: the worker {pid} is stopped while it works");
        kill(pid)
    }

    /// Held across a reap: a reaped id may be reused.
    pub fn gate(&self) -> MutexGuard<'_, bool> {
        // A bool cannot be left half written.
        self.reaped.lock().unwrap_or_else(PoisonError::into_inner)
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
