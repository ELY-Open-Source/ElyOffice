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
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title(app.name())
            .set_description(&text)
            .show();
    }
    process::exit(1)
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
