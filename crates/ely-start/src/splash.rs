//! The splash window: the app's icon and name.

use std::{
    io,
    path::Path,
    process::{Child, Command},
};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, CreateFontW, CreateSolidBrush, DT_CENTER, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW,
        EndPaint, FillRect, PAINTSTRUCT, SelectObject, SetBkMode, SetTextColor, TRANSPARENT, UpdateWindow,
    },
    System::{
        LibraryLoader::GetModuleHandleW,
        Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW},
    },
    UI::{HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForSystem, SetProcessDpiAwarenessContext}, WindowsAndMessaging::*},
};

/// The spawn's result, boxed in the message.
const SPAWNED: u32 = WM_APP + 1;
/// Sizes in 96-dpi pixels.
const WIDTH: i32 = 360;
const HEIGHT: i32 = 240;
const ICON: i32 = 96;

struct Shown {
    name: Vec<u16>,
    icon: HICON,
    scale: i32,
    dark: bool,
    child: Option<Child>,
    failed: Option<io::Error>,
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

/// Shows the splash until the app closes it or exits.
pub fn run(name: &str, icon: &Path, mut command: Command) -> io::Result<()> {
    // SAFETY: plain Win32 calls on this thread's own window.
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let scale = GetDpiForSystem() as i32;
        let side = ICON * scale / 96;
        let path = wide(&icon.to_string_lossy());
        let handle = LoadImageW(std::ptr::null_mut(), path.as_ptr(), IMAGE_ICON, side, side, LR_LOADFROMFILE);
        if handle.is_null() {
            return Err(io::Error::other(format!("no icon at {}: {}", icon.display(), io::Error::last_os_error())));
        }
        let mut shown = Shown { name: wide(name), icon: handle, scale, dark: dark(), child: None, failed: None };
        let module = GetModuleHandleW(std::ptr::null());
        let class = wide("ElySplash");
        let kind = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(proc_),
            hInstance: module,
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_APPSTARTING),
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        if RegisterClassExW(&kind) == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut area: RECT = std::mem::zeroed();
        SystemParametersInfoW(SPI_GETWORKAREA, 0, (&raw mut area).cast(), 0);
        let (width, height) = (WIDTH * scale / 96, HEIGHT * scale / 96);
        let title = wide(name);
        let window = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            area.left + (area.right - area.left - width) / 2,
            area.top + (area.bottom - area.top - height) / 2,
            width,
            height,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            module,
            (&raw mut shown).cast(),
        );
        if window.is_null() {
            return Err(io::Error::last_os_error());
        }
        ShowWindow(window, SW_SHOW);
        UpdateWindow(window);
        // The app may take the foreground from us.
        AllowSetForegroundWindow(ASFW_ANY);
        let target = window as isize;
        // The app closes it at its first frame.
        command.env("ELY_SPLASH", target.to_string());
        std::thread::spawn(move || {
            let started = Box::into_raw(Box::new(command.spawn()));
            PostMessageW(target as HWND, SPAWNED, 0, started as LPARAM);
        });
        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        match shown.failed {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

/// Windows' app mode: dark unless apps use light.
fn dark() -> bool {
    let key = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let value = wide("AppsUseLightTheme");
    let mut light = 1u32;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: `light` holds the DWORD the size names.
    let read = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), value.as_ptr(), RRF_RT_REG_DWORD, std::ptr::null_mut(), (&raw mut light).cast(), &mut size)
    };
    read == 0 && light == 0
}

/// The splash state, set at creation.
///
/// SAFETY: `run` keeps it alive and untouched while messages run.
unsafe fn state<'a>(window: HWND) -> Option<&'a mut Shown> {
    // SAFETY: the slot holds null or `run`'s `Shown`.
    unsafe { (GetWindowLongPtrW(window, GWLP_USERDATA) as *mut Shown).as_mut() }
}

unsafe extern "system" fn proc_(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: Windows calls this on the window's thread; no borrow spans DestroyWindow.
    unsafe {
        match message {
            WM_NCCREATE => {
                let create = &*(lparam as *const CREATESTRUCTW);
                SetWindowLongPtrW(window, GWLP_USERDATA, create.lpCreateParams as isize);
                DefWindowProcW(window, message, wparam, lparam)
            }
            WM_PAINT => {
                if let Some(shown) = state(window) {
                    paint(window, shown);
                }
                0
            }
            SPAWNED => {
                let started = *Box::from_raw(lparam as *mut io::Result<Child>);
                let failed = match (state(window), started) {
                    (Some(shown), Ok(child)) => {
                        shown.child = Some(child);
                        false
                    }
                    (Some(shown), Err(error)) => {
                        shown.failed = Some(error);
                        true
                    }
                    (None, _) => true,
                };
                if failed {
                    DestroyWindow(window);
                } else {
                    SetTimer(window, 1, 30, None);
                }
                0
            }
            WM_TIMER => {
                if state(window).is_none_or(exited) {
                    DestroyWindow(window);
                }
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}

/// The app exited before its first frame.
fn exited(shown: &mut Shown) -> bool {
    let Some(child) = shown.child.as_mut() else {
        return false;
    };
    match child.try_wait() {
        Ok(status) => status.is_some(),
        Err(error) => {
            shown.failed = Some(error);
            true
        }
    }
}

/// Background, icon and name.
unsafe fn paint(window: HWND, shown: &Shown) {
    // SAFETY: a paint cycle on this thread's window.
    unsafe {
        let mut sheet: PAINTSTRUCT = std::mem::zeroed();
        let dc = BeginPaint(window, &mut sheet);
        let (back, ink) = if shown.dark { (0x0025_2525, 0x00f0_f0f0) } else { (0x00ff_ffff, 0x0020_2020) };
        let brush = CreateSolidBrush(back);
        let mut whole: RECT = std::mem::zeroed();
        GetClientRect(window, &mut whole);
        FillRect(dc, &whole, brush);
        DeleteObject(brush);
        let side = ICON * shown.scale / 96;
        let top = whole.bottom * 2 / 9;
        DrawIconEx(dc, (whole.right - side) / 2, top, shown.icon, side, side, 0, std::ptr::null_mut(), DI_NORMAL);
        let face = wide("Segoe UI Semibold");
        let font = CreateFontW(-22 * shown.scale / 96, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr());
        let old = SelectObject(dc, font);
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, ink);
        let mut line = RECT { left: 0, top: top + side, right: whole.right, bottom: top + side + 56 * shown.scale / 96 };
        DrawTextW(dc, shown.name.as_ptr(), shown.name.len() as i32 - 1, &mut line, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
        SelectObject(dc, old);
        DeleteObject(font);
        EndPaint(window, &sheet);
    }
}

/// A blocking error box.
pub fn alert(title: &str, text: &str) {
    let (title, text) = (wide(title), wide(text));
    // SAFETY: both strings end in NUL and outlive the call.
    unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_ICONERROR | MB_OK) };
}
