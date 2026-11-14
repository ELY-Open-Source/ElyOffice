//! The resident engine: `<app> engine` and `engine stop`.

use std::{
    io,
    path::PathBuf,
    process::{Command, Stdio},
    sync::Once,
    time::{Instant, SystemTime},
};

use ely_lok::Worker;
use ely_platform::AppId;

const GB: u64 = 1 << 30;
/// The apps an idle engine waits for.
const DOCUMENT_APPS: [AppId; 3] = [AppId::Doc, AppId::Table, AppId::Show];
/// Touched when an app opens; the newest is used last.
const USED: &str = "engine-used";
const USAGE: &str = "usage: engine [stop]";

/// Runs `engine` or `engine stop`; None for other arguments.
pub fn command_line(app: AppId) -> Option<i32> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_none_or(|first| first != "engine") {
        return None;
    }
    // The running copy owns the log; this one appends.
    if let Err(error) = ely_platform::logging::join(app) {
        eprintln!("{}: logging did not start: {error:#}", app.slug());
        return Some(1);
    }
    let done = match &args[1..] {
        [] => wait(app),
        [verb] if verb == "stop" => stop(app),
        _ => {
            log::error!("{}: {USAGE}", app.slug());
            return Some(2);
        }
    };
    match done {
        Ok(()) => Some(0),
        Err(error) => {
            log::error!("{}: engine: {error}", app.slug());
            Some(1)
        }
    }
}

/// The kernel, profile and start check of `app`.
pub(crate) fn place(app: AppId) -> io::Result<(PathBuf, PathBuf, ely_lok::Probe)> {
    let data = ely_platform::data_dir(app).map_err(io::Error::other)?;
    // The check loads the module this app needs anyway (#79).
    let probe = match app {
        AppId::Table => ely_lok::Probe::Sheet,
        AppId::Show => ely_lok::Probe::Slides,
        _ => ely_lok::Probe::Text,
    };
    Ok((ely_lok::install_dir()?, data.join("lok-profile"), probe))
}

/// Starts a checked idle engine, unless memory says not.
fn wait(app: AppId) -> io::Result<()> {
    let slug = app.slug();
    let memory = ely_platform::memory::physical()?;
    // One engine command at a time; each decides afresh.
    let _turn = ely_lok::Start::wait(&ely_platform::root().map_err(io::Error::other)?)?;
    if let Some(why) = held_back(app, memory)? {
        log::info!("{slug}: no idle engine: {why}");
        return Ok(());
    }
    // Under 8 GB only one app's engine waits.
    if memory < 8 * GB {
        for other in DOCUMENT_APPS.into_iter().filter(|other| *other != app) {
            stop(other)?;
        }
    }
    let (install, profile, probe) = place(app)?;
    // Held to release: an app waits rather than starting.
    let start = ely_lok::Start::wait(&profile)?;
    if start.ready()? {
        log::info!("{slug}: an idle engine already waits");
        return Ok(());
    }
    let data = ely_platform::data_dir(app).map_err(io::Error::other)?;
    let log = ely_platform::logging::kept(&data.join("engine.log"))?;
    let began = Instant::now();
    let pid = Worker::idle(&start, &install, probe, slug, log)?.release()?;
    log::info!("{slug}: engine {pid} waits, ready in {} ms", began.elapsed().as_millis());
    Ok(())
}

/// Ends the idle engine; none waiting is no error.
fn stop(app: AppId) -> io::Result<()> {
    let (install, profile, probe) = place(app)?;
    let mut worker = match Worker::adopt(&install, &profile, app.slug(), probe) {
        Ok(worker) => worker,
        // None waits, or an app holds it.
        Err(error) if ely_lok::no_idle_engine(&error) => {
            log::info!("{}: no idle engine to stop ({error})", app.slug());
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    worker.shutdown()?;
    log::info!("{}: the idle engine stopped", app.slug());
    Ok(())
}

/// Why `memory` keeps `app` from an idle engine.
fn held_back(app: AppId, memory: u64) -> io::Result<Option<String>> {
    let size = format!("{:.1} GB of memory", memory as f64 / GB as f64);
    if memory < 4 * GB {
        return Ok(Some(size));
    }
    if memory < 8 * GB && last_used()? != Some(app) {
        return Ok(Some(format!("{size}, and another app was used last")));
    }
    Ok(None)
}

/// The document app opened most recently.
fn last_used() -> io::Result<Option<AppId>> {
    let mut newest: Option<(SystemTime, AppId)> = None;
    for app in DOCUMENT_APPS {
        let stamp = ely_platform::data_dir(app).map_err(io::Error::other)?.join(USED);
        let at = match std::fs::metadata(&stamp).and_then(|meta| meta.modified()) {
            Ok(at) => at,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(io::Error::new(error.kind(), format!("{}: {error}", stamp.display()))),
        };
        if newest.is_none_or(|(time, _)| at > time) {
            newest = Some((at, app));
        }
    }
    Ok(newest.map(|(_, app)| app))
}

/// Marks `app` as the one used last.
pub(crate) fn used(app: AppId) -> io::Result<()> {
    let dir = ely_platform::data_dir(app).map_err(io::Error::other)?;
    let stamp = dir.join(USED);
    std::fs::write(&stamp, []).map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", stamp.display())))
}

/// Once per run: an idle engine for the next start.
pub(crate) fn successor(app: AppId) {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        if let Err(error) = start_successor(app) {
            log::error!("{}: no successor engine: {error}", app.slug());
        }
    });
}

fn start_successor(app: AppId) -> io::Result<()> {
    let mut command = Command::new(std::env::current_exe()?);
    ely_ipc::windowless(&mut command).arg("engine").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = command.spawn()?;
    log::info!("{}: successor engine started as {}", app.slug(), child.id());
    // Reaped, so no zombie stays.
    std::thread::Builder::new().name("engine-successor".into()).spawn(move || match child.wait() {
        Ok(status) if status.success() => {}
        Ok(status) => log::error!("{}: the successor engine exited as {status}", app.slug()),
        Err(error) => log::error!("{}: the successor engine was not reaped: {error}", app.slug()),
    })?;
    Ok(())
}
