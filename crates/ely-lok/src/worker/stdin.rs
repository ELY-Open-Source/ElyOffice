//! Windows: the request pipe, hidden from the kernel's stdin.

use std::{ffi::c_char, fs::File, io, os::windows::io::FromRawHandle};

use windows_sys::Win32::{
    Foundation::{DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE},
    System::{
        Console::{GetStdHandle, STD_INPUT_HANDLE, SetStdHandle},
        Threading::GetCurrentProcess,
    },
};

unsafe extern "C" {
    fn _open(path: *const c_char, flags: i32, ...) -> i32;
    fn _dup2(from: i32, to: i32) -> i32;
    fn _close(fd: i32) -> i32;
    fn _get_osfhandle(fd: i32) -> isize;
}

/// The pipe; stdin turns NUL: the kernel's Python seeks it.
pub fn private() -> io::Result<File> {
    let process = unsafe { GetCurrentProcess() };
    let pipe = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    let mut own: HANDLE = std::ptr::null_mut();
    if unsafe { DuplicateHandle(process, pipe, process, &mut own, 0, 0, DUPLICATE_SAME_ACCESS) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let nul = unsafe { _open(c"NUL".as_ptr(), 0) };
    if nul < 0 || unsafe { _dup2(nul, 0) } != 0 {
        return Err(io::Error::other("stdin did not become NUL"));
    }
    unsafe { _close(nul) };
    if unsafe { SetStdHandle(STD_INPUT_HANDLE, _get_osfhandle(0) as HANDLE) } == 0 {
        return Err(io::Error::last_os_error());
    }
    log::info!("lok-worker: requests read from a private pipe handle");
    Ok(unsafe { File::from_raw_handle(own) })
}
