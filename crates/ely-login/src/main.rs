//! After login and setup: each document app's idle engine.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod registry;

#[cfg(not(windows))]
fn main() {
    eprintln!("ely-login runs on Windows only");
    std::process::exit(2);
}

#[cfg(windows)]
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

/// Runs `engine` or `engine stop` of `app`, if installed.
#[cfg(windows)]
fn engine(app: ely_platform::AppId, stop: bool) -> std::io::Result<()> {
    use std::{
        process::{Command, Stdio},
        time::Instant,
    };
    let Some(dir) = registry::install_dir(app.name())? else {
        log::info!("{}: not installed", app.slug());
        return Ok(());
    };
    let exe = dir.join(app.slug()).with_extension("exe");
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
