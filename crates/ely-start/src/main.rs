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
    use std::time::{SystemTime, UNIX_EPOCH};
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
    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|error| fail(&name, &format!("the clock is before 1970: {error}")));
    let mut command = std::process::Command::new(&app);
    command.args(args).env("ELY_START", at.as_millis().to_string());
    if let Err(error) = splash::run(&name, &icon, command) {
        fail(&name, &format!("{}: {error}", app.display()));
    }
}

/// Shows why the app cannot start, then exits 1.
#[cfg(windows)]
fn fail(name: &str, reason: &str) -> ! {
    let text = format!("{name} 无法启动。\n{name} could not start.\n\n{reason}");
    eprintln!("{text}");
    if std::env::var_os("ELY_NO_DIALOG").is_none() {
        splash::alert(name, &text);
    }
    std::process::exit(1)
}
