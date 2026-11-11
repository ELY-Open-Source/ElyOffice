use std::{
    panic, process,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{AppId, logging};

static STARTED: AtomicBool = AtomicBool::new(false);

/// The text a failed start shows, in both languages.
pub fn message(app: AppId, reason: &str, log: &str) -> String {
    let name = app.name();
    format!("{name} 无法启动。\n{name} could not start.\n\n{reason}\n\n日志 / Log: {log}")
}

/// Logs and shows why `app` cannot start, then exits 1.
pub fn fail(app: AppId, reason: &str) -> ! {
    log::error!("{} cannot start: {reason}", app.name());
    let log = logging::path().map_or_else(
        || "none, logging did not start".to_string(),
        |path| path.display().to_string(),
    );
    let text = message(app, reason, &log);
    eprintln!("{text}");
    if std::env::var_os("ELY_NO_DIALOG").is_none() {
        show(app.name(), text);
    }
    process::exit(1)
}

/// Its own thread: the modal loop must not run gpui.
#[cfg(windows)]
fn show(title: &'static str, text: String) {
    if !visible() {
        log::warn!("no visible desktop, so no dialog");
        return;
    }
    let shown = std::thread::spawn(move || shown(title, &text)).join();
    if shown.is_err() {
        log::error!("the failed-start dialog panicked");
    }
}

/// A service's or SSH login's station has none.
#[cfg(windows)]
fn visible() -> bool {
    use windows_sys::Win32::System::StationsAndDesktops::{
        GetProcessWindowStation, GetUserObjectInformationW, UOI_FLAGS, USEROBJECTFLAGS,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::WSF_VISIBLE;
    let mut flags = USEROBJECTFLAGS::default();
    // SAFETY: `flags` is valid for writes of the size passed.
    let read = unsafe {
        GetUserObjectInformationW(
            GetProcessWindowStation(),
            UOI_FLAGS,
            (&raw mut flags).cast(),
            size_of::<USEROBJECTFLAGS>() as u32,
            std::ptr::null_mut(),
        )
    };
    read != 0 && flags.dwFlags & WSF_VISIBLE as u32 != 0
}

#[cfg(not(windows))]
fn show(title: &'static str, text: String) {
    shown(title, &text);
}

fn shown(title: &str, text: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(title)
        .set_description(text)
        .show();
}

/// Until `started`, a panic is a failed start.
pub fn guard(app: AppId) {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if STARTED.load(Ordering::Relaxed) {
            previous(info);
        } else {
            fail(
                app,
                &format!("the app panicked before its first frame: {info}"),
            );
        }
    }));
}

/// Ely restores sessions itself; AppKit's "reopen windows?" would block.
#[cfg(target_os = "macos")]
pub fn no_window_restore() {
    use objc2_foundation::{NSString, NSUserDefaults};
    let key = NSString::from_str("ApplePersistenceIgnoreState");
    NSUserDefaults::standardUserDefaults().setBool_forKey(true, &key);
}

#[cfg(not(target_os = "macos"))]
pub fn no_window_restore() {}

/// A command-line run prints to the terminal that started it.
#[cfg(windows)]
pub fn attach_console() {
    use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_OUTPUT_HANDLE};
    // Redirected output already has somewhere to go.
    if unsafe { GetStdHandle(STD_OUTPUT_HANDLE) }.is_null() {
        unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
    }
}

#[cfg(not(windows))]
pub fn attach_console() {}

/// Marks the start as done; later panics are ordinary panics.
pub fn started() {
    STARTED.store(true, Ordering::Relaxed);
}

/// Logs the time since Windows' splash launcher began.
pub fn launched() {
    let Some(at) = crate::env::value("ELY_START") else {
        return;
    };
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    match (at.parse::<u128>(), now) {
        (Ok(at), Ok(now)) => log::info!("startup: {} ms after the splash launcher", now.as_millis().saturating_sub(at)),
        (Err(error), _) => log::warn!("startup: ELY_START={at} is not a time: {error}"),
        (_, Err(error)) => log::warn!("startup: the clock is before 1970: {error}"),
    }
}

/// Closes the launcher's splash once the first frame shows.
#[cfg(windows)]
pub fn close_splash() {
    use windows_sys::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{GetClassNameW, PostMessageW, WM_CLOSE},
    };
    let Some(handle) = crate::env::value("ELY_SPLASH") else {
        return;
    };
    let Ok(raw) = handle.parse::<isize>() else {
        log::warn!("startup: ELY_SPLASH={handle} is not a window");
        return;
    };
    let window = raw as HWND;
    let mut class = [0u16; 16];
    // SAFETY: `class` holds the length passed.
    let len = unsafe { GetClassNameW(window, class.as_mut_ptr(), class.len() as i32) };
    // A stale handle may name another window.
    if String::from_utf16_lossy(&class[..len.max(0) as usize]) != "ElySplash" {
        log::warn!("startup: the splash window {handle} is gone");
        return;
    }
    // SAFETY: the handle names a live splash window.
    if unsafe { PostMessageW(window, WM_CLOSE, 0, 0) } == 0 {
        log::warn!("startup: the splash did not close: {}", std::io::Error::last_os_error());
        return;
    }
    log::info!("startup: splash closed");
}

#[cfg(not(windows))]
pub fn close_splash() {}

#[cfg(test)]
mod tests {
    use super::message;
    use crate::AppId;

    #[test]
    fn the_message_names_the_app_the_reason_and_the_log_in_both_languages() {
        let text = message(AppId::Pdf, "no usable GPU", "/logs/ely.log");

        assert!(text.contains("ElyPDF 无法启动"), "{text}");
        assert!(text.contains("ElyPDF could not start"), "{text}");
        assert!(text.contains("no usable GPU"), "{text}");
        assert!(text.contains("/logs/ely.log"), "{text}");
    }
}
