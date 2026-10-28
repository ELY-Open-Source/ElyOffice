//! Stopping Git on Windows, as Git for Windows does.

use std::{
    ffi::c_void,
    io,
    os::windows::io::{AsRawHandle as _, FromRawHandle as _, OwnedHandle},
    sync::OnceLock,
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
const GRACE_MS: u32 = 10_000;

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
    let Some(first) = held.first() else { return };
    if unsafe { WaitForSingleObject(first.as_raw_handle(), GRACE_MS) } == WAIT_OBJECT_0 {
        return;
    }
    log::warn!("tree: git {root} still runs after ctrl-c; ending it");
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
