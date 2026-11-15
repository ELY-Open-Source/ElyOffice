//! After login and setup: each document app's idle engine.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod registry;

#[cfg(not(any(windows, target_os = "macos")))]
fn main() {
    eprintln!("ely-login runs on Windows and macOS only");
    std::process::exit(2);
}

#[cfg(any(windows, target_os = "macos"))]
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let stop = match args.as_slice() {
        [] => false,
        [verb] if verb == "stop" => true,
        _ => {
            eprintln!("usage: ely-login [stop]");
            std::process::exit(2);
        }
    };
    if let Err(error) = ely_platform::root()
        .and_then(|root| ely_platform::logging::init_at("ely-login", &root.join("ely-login.log")))
    {
        eprintln!("ely-login: logging did not start: {error:#}");
        std::process::exit(1);
    }
    let mut failed = false;
    for app in ely_platform::AppId::DOCUMENTS {
        if let Err(error) = engine(app, stop) {
            log::error!("{}: {error}", app.slug());
            failed = true;
        }
    }
    std::process::exit(i32::from(failed));
}

/// The app's executable, if installed.
#[cfg(windows)]
fn installed(app: ely_platform::AppId) -> std::io::Result<Option<std::path::PathBuf>> {
    Ok(registry::install_dir(app.name())?.map(|dir| dir.join(app.slug()).with_extension("exe")))
}

/// The app's executable, if this release's `.pkg` installed it.
#[cfg(target_os = "macos")]
fn installed(app: ely_platform::AppId) -> std::io::Result<Option<std::path::PathBuf>> {
    let bundle = std::path::Path::new("/Applications").join(format!("{}.app", app.name()));
    let exe = bundle.join("Contents/MacOS").join(app.slug());
    if !exe.try_exists()? {
        return Ok(None);
    }
    // 0.1.16 has no helper and no `engine`.
    if !bundle.join("Contents/Helpers/Ely Engine.app").try_exists()? {
        log::info!("{}: no engine helper; an old layout", app.slug());
        return Ok(None);
    }
    // An unchosen app keeps its old version and kernel.
    let plist = bundle.join("Contents/Info.plist");
    let out = std::process::Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleShortVersionString"])
        .arg(&plist)
        .output()?;
    if !out.status.success() {
        return Err(std::io::Error::other(format!(
            "{}: {}",
            plist.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    let version = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if version != env!("CARGO_PKG_VERSION") {
        log::info!(
            "{}: version {version} is not this release's {}",
            app.slug(),
            env!("CARGO_PKG_VERSION")
        );
        return Ok(None);
    }
    Ok(Some(exe))
}

/// Runs `engine` or `engine stop` of `app`, if installed.
#[cfg(any(windows, target_os = "macos"))]
fn engine(app: ely_platform::AppId, stop: bool) -> std::io::Result<()> {
    use std::{
        process::{Command, Stdio},
        time::Instant,
    };
    let Some(exe) = installed(app)? else {
        log::info!("{}: no engine to start", app.slug());
        return Ok(());
    };
    let verb = if stop { "engine stop" } else { "engine" };
    let mut command = Command::new(&exe);
    command
        .args(verb.split(' '))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let began = Instant::now();
    let status = command.status().map_err(|error| {
        std::io::Error::new(error.kind(), format!("{}: {error}", exe.display()))
    })?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "{} {verb} exited as {status}",
            exe.display()
        )));
    }
    log::info!(
        "{}: {verb} done in {} ms",
        app.slug(),
        began.elapsed().as_millis()
    );
    Ok(())
}
