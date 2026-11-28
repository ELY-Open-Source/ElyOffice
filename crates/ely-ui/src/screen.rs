//! Taking a window to another screen.

use gpui::{App, DisplayId, Task, Window};

/// Moves `window` onto `display`; only Windows can.
#[cfg(not(windows))]
pub fn move_to_display(window: &Window, display: DisplayId, cx: &App) -> Task<()> {
    if window.display(cx).is_some_and(|own| own.id() != display) {
        log::info!("screen: this system places windows itself");
    }
    Task::ready(())
}

/// Moves `window` onto `display`, once the update ends.
#[cfg(windows)]
pub fn move_to_display(window: &Window, display: DisplayId, cx: &App) -> Task<()> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        log::error!("screen: the window has no handle to move");
        return Task::ready(());
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        log::error!("screen: the window is not a Win32 window");
        return Task::ready(());
    };
    let (window, monitor) = (handle.hwnd.get(), u64::from(display) as usize);
    // Moving sends messages that gpui takes outside updates.
    cx.foreground_executor().spawn(async move { win32::place(window as _, monitor as _) })
}

#[cfg(windows)]
mod win32 {
    use windows_sys::Win32::{
        Foundation::{HWND, RECT},
        Graphics::Gdi::{GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow},
        UI::WindowsAndMessaging::{GetWindowRect, IsZoomed, SW_MAXIMIZE, SW_RESTORE, SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos, ShowWindow},
    };

    /// Centres the window there; a maximized one stays so.
    pub fn place(window: HWND, monitor: HMONITOR) {
        // SAFETY: Win32 refuses stale handles; the structs are sized.
        unsafe {
            if MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) == monitor {
                return;
            }
            let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..std::mem::zeroed() };
            if GetMonitorInfoW(monitor, &mut info) == 0 {
                return log::error!("screen: the target screen is gone");
            }
            let maximized = IsZoomed(window) != 0;
            if maximized {
                ShowWindow(window, SW_RESTORE);
            }
            let mut rect: RECT = std::mem::zeroed();
            if GetWindowRect(window, &mut rect) == 0 {
                return log::error!("screen: the window's place is unreadable");
            }
            let work = info.rcWork;
            let width = (rect.right - rect.left).min(work.right - work.left);
            let height = (rect.bottom - rect.top).min(work.bottom - work.top);
            let x = work.left + (work.right - work.left - width) / 2;
            let y = work.top + (work.bottom - work.top - height) / 2;
            if SetWindowPos(window, std::ptr::null_mut(), x, y, width, height, SWP_NOZORDER | SWP_NOACTIVATE) == 0 {
                log::error!("screen: the window did not move: {}", std::io::Error::last_os_error());
            }
            if maximized {
                ShowWindow(window, SW_MAXIMIZE);
            }
        }
    }
}
