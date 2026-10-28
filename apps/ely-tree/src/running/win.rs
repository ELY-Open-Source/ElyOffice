//! Stopping Git on Windows, as Git for Windows does.

use std::{
    ffi::c_void,
    io,
    os::windows::io::{AsRawHandle as _, FromRawHandle as _, OwnedHandle},
    sync::OnceLock,
    time::{Duration, Instant},
};

use windows_sys::{
    Win32::{
        Foundation::{ERROR_INVALID_PARAMETER, ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE, WAIT_OBJECT_0},
        System::{
            Console::SetConsoleCtrlHandler,
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
                TH32CS_SNAPPROCESS,
            },
            LibraryLoader::{GetModuleHandleW, GetProcAddress},
            Threading::{
                CreateRemoteThread, GetCurrentProcess, IsWow64Process, OpenProcess,
                PROCESS_CREATE_THREAD, PROCESS_QUERY_INFORMATION, PROCESS_SYNCHRONIZE,
                PROCESS_TERMINATE, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
                TerminateProcess, WaitForSingleObject,
            },
        },
    },
    core::BOOL,
};

/// Git for Windows waits this long, then ends the tree.
const GRACE: Duration = Duration::from_secs(10);

type Routine = unsafe extern "system" fn(*mut c_void) -> u32;

fn checked(done: BOOL) -> io::Result<()> {
    match done {
        0 => Err(io::Error::last_os_error()),
        _ => Ok(()),
    }
}

/// Git started after this hears Ctrl+C.
pub fn hear() -> io::Result<()> {
    static SET: OnceLock<Result<(), String>> = OnceLock::new();
    // A parent may have passed down "ignore Ctrl+C".
    SET.get_or_init(|| checked(unsafe { SetConsoleCtrlHandler(None, 0) }).map_err(|e| e.to_string()))
        .clone()
        .map_err(io::Error::other)
}

/// Ctrl+C to Git and its helpers; Git cleans up.
pub fn stop(root: u32) -> io::Result<()> {
    let routine = ctrl_routine()?;
    let mut held = Vec::new();
    for (pid, host) in tree(root)? {
        let process = match open(pid) {
            Ok(process) => process,
            Err(error) if error.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) => {
                log::info!("tree: git {root}'s {pid} had ended");
                continue;
            }
            Err(error) => return Err(error),
        };
        // Its console host is no Git.
        if !host {
            interrupt(&process, routine)?;
        }
        held.push(process);
    }
    log::info!("tree: ctrl-c to git {root}, {} processes", held.len());
    std::thread::spawn(move || end_after_grace(root, held));
    Ok(())
}

/// Git's exit can hang behind a blocked read.
fn end_after_grace(root: u32, held: Vec<OwnedHandle>) {
    let deadline = Instant::now() + GRACE;
    // A helper can outlive Git and hold its pipes.
    let ended = held.iter().all(|process| {
        let left = deadline.saturating_duration_since(Instant::now()).as_millis();
        let left = u32::try_from(left).unwrap_or(u32::MAX);
        let waited = unsafe { WaitForSingleObject(process.as_raw_handle(), left) };
        waited == WAIT_OBJECT_0
    });
    if ended {
        return;
    }
    log::warn!("tree: git {root}'s tree still runs after ctrl-c; ending it");
    for process in held.iter().rev() {
        let handle = process.as_raw_handle();
        if unsafe { WaitForSingleObject(handle, 0) } == WAIT_OBJECT_0 {
            continue;
        }
        if let Err(error) = checked(unsafe { TerminateProcess(handle, 130) }) {
            log::error!("tree: git {root} did not end: {error}");
        }
    }
}

fn ctrl_routine() -> io::Result<Routine> {
    let name: Vec<u16> = "kernelbase\0".encode_utf16().collect();
    // SAFETY: both names end in NUL.
    unsafe {
        let module = GetModuleHandleW(name.as_ptr());
        if module.is_null() {
            return Err(io::Error::last_os_error());
        }
        let found = GetProcAddress(module, c"CtrlRoutine".as_ptr().cast())
            .ok_or_else(io::Error::last_os_error)?;
        Ok(std::mem::transmute::<unsafe extern "system" fn() -> isize, Routine>(found))
    }
}

/// `root` and what it started; true marks a console host.
fn tree(root: u32) -> io::Result<Vec<(u32, bool)>> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let snapshot = unsafe { OwnedHandle::from_raw_handle(snapshot) };
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut all = Vec::new();
    let mut more = unsafe { Process32FirstW(snapshot.as_raw_handle(), &mut entry) };
    while more != 0 {
        let end = entry.szExeFile.iter().position(|&unit| unit == 0).unwrap_or(260);
        let name = String::from_utf16_lossy(&entry.szExeFile[..end]);
        all.push((entry.th32ProcessID, entry.th32ParentProcessID, name.eq_ignore_ascii_case("conhost.exe")));
        more = unsafe { Process32NextW(snapshot.as_raw_handle(), &mut entry) };
    }
    let last = io::Error::last_os_error();
    if last.raw_os_error() != Some(ERROR_NO_MORE_FILES as i32) {
        return Err(last);
    }
    let mut found = vec![(root, false)];
    // The list need not come parents first.
    loop {
        let before = found.len();
        for &(pid, parent, host) in &all {
            let known = |id| found.iter().any(|&(seen, _)| seen == id);
            if !known(pid) && known(parent) {
                found.push((pid, host));
            }
        }
        if found.len() == before {
            return Ok(found);
        }
    }
}

fn open(pid: u32) -> io::Result<OwnedHandle> {
    let rights = PROCESS_CREATE_THREAD
        | PROCESS_QUERY_INFORMATION
        | PROCESS_VM_OPERATION
        | PROCESS_VM_READ
        | PROCESS_VM_WRITE
        | PROCESS_TERMINATE
        | PROCESS_SYNCHRONIZE;
    let process = unsafe { OpenProcess(rights, 0, pid) };
    if process.is_null() {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { OwnedHandle::from_raw_handle(process) })
}

/// Windows' own Ctrl+C: a thread at CtrlRoutine.
fn interrupt(process: &OwnedHandle, routine: Routine) -> io::Result<()> {
    let handle = process.as_raw_handle();
    let (mut ours, mut theirs) = (0, 0);
    // SAFETY: handles stay open across the calls.
    unsafe {
        checked(IsWow64Process(GetCurrentProcess(), &mut ours))?;
        checked(IsWow64Process(handle, &mut theirs))?;
        if ours != theirs {
            return Err(io::Error::other("a Git of another architecture cannot be stopped"));
        }
        // The argument is CTRL_C_EVENT, zero.
        let thread = CreateRemoteThread(handle, std::ptr::null(), 0, Some(routine), std::ptr::null(), 0, std::ptr::null_mut());
        if thread.is_null() {
            return Err(io::Error::last_os_error());
        }
        drop(OwnedHandle::from_raw_handle(thread));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use crate::tests::{committed, git, sh};

    /// Run by the test below as its helper.
    #[test]
    #[ignore = "a helper process"]
    fn deaf() {
        let Some(marker) = std::env::var_os("ELY_DEAF") else { return };
        super::checked(unsafe { super::SetConsoleCtrlHandler(None, 1) }).expect("deaf");
        std::fs::write(marker, "").expect("marks");
        std::thread::sleep(Duration::from_secs(60));
    }

    #[test]
    fn a_helper_that_outlives_git_is_ended_too() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let root = committed(dir.path());
        let bare = dir.path().join("bare.git");
        sh(dir.path(), &["init", "-q", "--bare", &bare.to_string_lossy()]);
        let slash = |path: &std::path::Path| path.to_string_lossy().replace('\\', "/");
        let (marker, exe) = (dir.path().join("deaf"), std::env::current_exe().expect("this test"));
        let hook = format!(
            "#!/bin/sh\nELY_DEAF='{}' '{}' --exact running::win::tests::deaf --ignored --nocapture\n",
            slash(&marker),
            slash(&exe)
        );
        std::fs::write(root.join(".git/hooks/pre-push"), hook).expect("a hook");
        let running = git()
            .start(&root, &["push", &bare.to_string_lossy(), "HEAD:main"])
            .expect("starts");
        let stop = running.stop();
        let (done, result) = std::sync::mpsc::channel();
        std::thread::spawn(move || done.send(running.follow(|_| {})));
        let start = Instant::now();
        while !marker.exists() {
            assert!(start.elapsed() < Duration::from_secs(60), "the helper never ran");
            std::thread::sleep(Duration::from_millis(50));
        }
        stop.now().expect("asks");
        let ended = result
            .recv_timeout(super::GRACE + Duration::from_secs(10))
            .expect("the run ends though the helper ignores ctrl-c");
        assert!(ended.is_err(), "a stopped push fails: {ended:?}");
    }
}
