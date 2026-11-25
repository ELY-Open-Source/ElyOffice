//! Shows an app's splash while Windows scans its exe.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod splash;

#[cfg(not(windows))]
fn main() {
    eprintln!("ely-start runs on Windows only");
    std::process::exit(2);
}

#[cfg(windows)]
fn main() {
    use std::time::{Instant, SystemTime, UNIX_EPOCH};
    let mut args = std::env::args_os().skip(1);
    let (Some(slug), Some(name)) = (args.next(), args.next()) else {
        fail("ely-start", "usage: ely-start <slug> <name> [args...]");
    };
    let name = name.to_string_lossy().into_owned();
    let exe = std::env::current_exe().unwrap_or_else(|error| fail(&name, &format!("its own path is unknown: {error}")));
    let Some(dir) = exe.parent() else {
        fail(&name, &format!("{} has no folder", exe.display()));
    };
    let app = dir.join(&slug).with_extension("exe");
    let icon = dir.join(&slug).with_extension("ico");
    // One zero for launcher and app.
    let began = Instant::now();
    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|error| fail(&name, &format!("the clock is before 1970: {error}")));
    let mut command = std::process::Command::new(&app);
    command.args(args).env("ELY_START", at.as_millis().to_string());
    match splash::run(&name, &icon, command, began) {
        Err(error) => fail(&name, &format!("{}: {error}", app.display())),
        Ok(Some(status)) if !status.success() => {
            // The app gives its own reason; no second dialog.
            said(&name, &format!("{} exited before its window showed: {status}", app.display()));
            std::process::exit(status.code().filter(|code| *code != 0).unwrap_or(1))
        }
        Ok(_) => {}
    }
}

/// Writes why the app cannot start to stderr.
#[cfg(windows)]
fn said(name: &str, reason: &str) -> String {
    let text = format!("{name} 无法启动。\n{name} could not start.\n\n{reason}");
    eprintln!("{text}");
    text
}

/// Shows why the app cannot start, then exits 1.
#[cfg(windows)]
fn fail(name: &str, reason: &str) -> ! {
    let text = said(name, reason);
    if std::env::var_os("ELY_NO_DIALOG").is_none() {
        splash::alert(name, &text);
    }
    std::process::exit(1)
}
