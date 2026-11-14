//! Windows: requests ride the kernel's message loop to a window.

use std::{
    ffi::c_void,
    io,
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, HWND_MESSAGE, PostMessageW, RegisterClassW, WM_APP,
        WNDCLASSW,
    },
};

use super::{answer, asked, closing, events, lok, pin, quit, teardown, waits};
use crate::endpoint::Inbound;
use crate::ffi::Lok;

const ASK: u32 = WM_APP + 1;
const CLOSED: u32 = WM_APP + 2;
const DUE: u32 = WM_APP + 3;

/// The message-only window the requests go to.
static WINDOW: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

/// Requests posted and not yet answered.
static PENDING: AtomicUsize = AtomicUsize::new(0);

pub(super) fn serve(lok: Lok, input: Inbound) -> io::Result<()> {
    let lok = Box::leak(Box::new(lok));
    pin(lok);
    lok.register_callback(asked::on_kit);
    lok.register_any_input(waiting);
    WINDOW.store(window()?, Ordering::SeqCst);
    std::thread::spawn(move || reader(input));
    std::thread::spawn(events::push);
    // Unipoll needs hooks; the Windows loop never calls them.
    extern "C" fn poll(_: *mut c_void, _: i32) -> i32 {
        0
    }
    extern "C" fn wake(_: *mut c_void) {}
    lok.run_loop(Some(poll), Some(wake));
    log::info!("lok-worker: the kernel's loop ended");
    teardown::teardown(0);
}

/// Tiles assume 96 DPI; the kernel's thread draws unaware.
pub(super) fn unaware() -> io::Result<()> {
    use windows_sys::Win32::UI::HiDpi::{
        DPI_AWARENESS_CONTEXT_UNAWARE, DPI_AWARENESS_UNAWARE, GetAwarenessFromDpiAwarenessContext,
        GetThreadDpiAwarenessContext, SetThreadDpiAwarenessContext,
    };
    let awareness = || unsafe { GetAwarenessFromDpiAwarenessContext(GetThreadDpiAwarenessContext()) };
    let was = awareness();
    if was == DPI_AWARENESS_UNAWARE {
        return Ok(());
    }
    // Windows may start a manifestless process aware.
    if unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_UNAWARE) }.is_null() {
        return Err(io::Error::other(format!(
            "this worker is DPI-aware ({was}) and its thread stays so: {}",
            io::Error::last_os_error()
        )));
    }
    let now = awareness();
    if now != DPI_AWARENESS_UNAWARE {
        return Err(io::Error::other(format!("this worker's thread is DPI-aware ({now}) after it set itself unaware")));
    }
    log::info!("lok-worker: the process started DPI-aware ({was}); the kernel's thread is unaware");
    Ok(())
}

/// A window on this thread that only takes messages.
fn window() -> io::Result<HWND> {
    let name: Vec<u16> = "ElyLokWorker\0".encode_utf16().collect();
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    let class = WNDCLASSW {
        lpfnWndProc: Some(procedure),
        hInstance: instance,
        lpszClassName: name.as_ptr(),
        ..unsafe { std::mem::zeroed() }
    };
    if unsafe { RegisterClassW(&class) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let made = unsafe {
        CreateWindowExW(
            0,
            name.as_ptr(),
            std::ptr::null(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        )
    };
    match made.is_null() {
        true => Err(io::Error::last_os_error()),
        false => Ok(made),
    }
}

fn reader(mut input: Inbound) {
    loop {
        let (message, request) = match super::next_frame(&mut input) {
            Ok(Some((tag, _, payload))) if waits::noted(tag, &payload) => continue,
            Ok(Some(request)) => {
                PENDING.fetch_add(1, Ordering::SeqCst);
                (ASK, Box::into_raw(Box::new(request)))
            }
            Ok(None) => {
                log::info!("lok-worker: the app closed its connection");
                teardown::deadline();
                (CLOSED, std::ptr::null_mut())
            }
            Err(error) => {
                log::warn!("lok-worker: frame read failed: {error}");
                quit(1);
            }
        };
        let window = WINDOW.load(Ordering::SeqCst);
        if unsafe { PostMessageW(window, message, 0, request as LPARAM) } == 0 {
            log::error!("lok-worker: a request did not post: {}", io::Error::last_os_error());
            quit(1);
        }
        if message == CLOSED {
            return;
        }
    }
}

/// Asks the loop to close the due dialogs.
pub(super) fn nudge() {
    if unsafe { PostMessageW(WINDOW.load(Ordering::SeqCst), DUE, 0, 0) } == 0 {
        log::error!("lok-worker: a dialog close did not post: {}", io::Error::last_os_error());
    }
}

/// Idle jobs yield to a waiting request.
extern "C" fn waiting(_: *mut c_void, _: i32) -> bool {
    PENDING.load(Ordering::SeqCst) > 0 || waits::any()
}

unsafe extern "system" fn procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        ASK => {
            let request = unsafe { Box::from_raw(lparam as *mut (u8, u64, Vec<u8>)) };
            let (tag, id, payload) = *request;
            PENDING.fetch_sub(1, Ordering::SeqCst);
            answer(lok(), tag, id, &payload);
            0
        }
        CLOSED => teardown::teardown(0),
        DUE => {
            closing::now();
            0
        }
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}

#[cfg(test)]
mod tests {
    use windows_sys::Win32::UI::HiDpi::{
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, DPI_AWARENESS_UNAWARE, GetAwarenessFromDpiAwarenessContext,
        GetThreadDpiAwarenessContext, SetThreadDpiAwarenessContext,
    };

    /// An aware start leaves the kernel's thread unaware.
    #[test]
    fn an_aware_thread_turns_unaware() {
        std::thread::spawn(|| {
            assert!(!unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }.is_null());
            super::unaware().expect("the thread turns unaware");
            let now = unsafe { GetAwarenessFromDpiAwarenessContext(GetThreadDpiAwarenessContext()) };
            assert_eq!(now, DPI_AWARENESS_UNAWARE);
        })
        .join()
        .expect("the thread ends");
    }
}
