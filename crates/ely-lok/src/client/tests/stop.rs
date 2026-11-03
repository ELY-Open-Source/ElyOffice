//! A stuck request ends when its worker is stopped.

use std::time::Duration;

use super::*;

/// Freezes process `pid`: it reads nothing and answers nothing.
#[cfg(unix)]
fn freeze(pid: u32) {
    let pid = i32::try_from(pid).expect("a process id");
    assert_eq!(unsafe { libc::kill(pid, libc::SIGSTOP) }, 0, "{}", std::io::Error::last_os_error());
}

/// Freezes process `pid`: it reads nothing and answers nothing.
#[cfg(windows)]
fn freeze(pid: u32) {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows_sys::Win32::System::Threading::{OpenThread, SuspendThread, THREAD_SUSPEND_RESUME};
    let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    assert_ne!(snap, INVALID_HANDLE_VALUE, "{}", std::io::Error::last_os_error());
    let mut entry = THREADENTRY32 { dwSize: size_of::<THREADENTRY32>() as u32, ..unsafe { std::mem::zeroed() } };
    let mut frozen = 0;
    let mut more = unsafe { Thread32First(snap, &mut entry) } != 0;
    while more {
        if entry.th32OwnerProcessID == pid {
            let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
            assert!(!thread.is_null(), "{}", std::io::Error::last_os_error());
            assert_ne!(unsafe { SuspendThread(thread) }, u32::MAX, "{}", std::io::Error::last_os_error());
            unsafe { CloseHandle(thread) };
            frozen += 1;
        }
        more = unsafe { Thread32Next(snap, &mut entry) } != 0;
    }
    unsafe { CloseHandle(snap) };
    assert!(frozen > 0, "process {pid} has no threads");
}

#[test]
fn a_stuck_load_returns_once_its_worker_is_stopped() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker spawns");
    worker.ready().expect("the worker passes its checks");
    let source = dir.path().join("note.fodt");
    write(&source, FODT);
    let (stopper, pid) = (worker.stopper(), worker.child.id());
    freeze(pid);
    let (done, wait) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let failed = worker.load(&crate::file_url(&source)).is_err();
        // Kept alive: its drop would close the gate too.
        done.send((failed, worker)).expect("the test waits");
    });
    if wait.recv_timeout(Duration::from_secs(3)).is_ok() {
        kill(pid);
        panic!("a frozen worker answered the load");
    }
    stopper.stop().expect("the stop lands");
    let (failed, mut worker) = wait.recv_timeout(Duration::from_secs(30)).expect("the load returns once stopped");
    assert!(failed, "the load reports the stop");
    // Reaped now: a late stop must not touch the id.
    stopper.stop().expect("a late stop does nothing");
    assert!(worker.ended().is_some(), "the worker is gone");
}
