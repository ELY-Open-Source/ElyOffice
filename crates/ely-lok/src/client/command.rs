//! The worker's command line, and its connect after spawn.

use std::{
    io::{self, BufReader},
    path::PathBuf,
    process::Command,
    sync::mpsc,
};

use super::process::Ends;
use super::{Feed, at, frames, worker_bin};
use crate::endpoint::{self, Half};

/// Replies, or why output stopped.
type Replies = mpsc::Sender<io::Result<(u8, u64, Vec<u8>)>>;

/// The worker on `private`, idle on `idle` once checked.
pub(super) fn build(start: &(PathBuf, PathBuf, bool), private: &str, idle: Option<&str>) -> io::Result<(Command, PathBuf)> {
    let (install, profile_dir, dev) = start;
    std::fs::create_dir_all(profile_dir).map_err(at(profile_dir))?;
    let bin = worker_bin()?;
    log::info!("lok: worker from {}", bin.display());
    let mut command = Command::new(&bin);
    ely_ipc::windowless(&mut command)
        .arg(install)
        .arg(profile_dir)
        .arg(private)
        .env("SAL_FORCE_HIDPI_SCALING", "1")
        // A killed worker's profile lock would hang the next.
        .env("SAL_NOLOCK_PROFILE", "1")
        // The kernel's OpenCL test can hang in AMD's driver.
        .env("SAL_DISABLE_OPENCL", "1");
    // Linux tiles via cairo are BGRA; Skia there is RGBA.
    match cfg!(target_os = "linux") {
        true => command.env("SAL_DISABLESKIA", "1"),
        false => command.env("SAL_SKIA", "raster"),
    };
    // Skia's watchdog wakes four times a second.
    command.env("SAL_DISABLE_WATCHDOG", "1");
    // Calc reports in document twips; unipoll lends LOK's loop.
    command.env("SAL_LOK_OPTIONS", "unipoll:sc_print_twips_msgs");
    // LOK spells no language off this list.
    command.env("LOK_ALLOWLIST_LANGUAGES", "en");
    if *dev {
        command.arg("--dev");
    }
    if let Some(idle) = idle {
        command.arg("--idle").arg(idle);
    }
    // A log file would keep patch lines from the app.
    command.env_remove("SAL_LOG_FILE");
    if let Ok(sal_log) = std::env::var("ELY_LOK_SAL_LOG") {
        command.env("SAL_LOG", sal_log);
    }
    // One folder per engine; its teardown removes it.
    let tmp = profile_dir.join("tmp");
    std::fs::create_dir_all(&tmp).map_err(at(&tmp))?;
    // Only a new folder; another engine may own old ones.
    let temp = tmp.join(private);
    std::fs::create_dir(&temp).map_err(at(&temp))?;
    // Unix reads TMPDIR; Windows reads TMP, then TEMP.
    command.env("TMPDIR", &temp).env("TMP", &temp).env("TEMP", &temp);
    Ok((command, bin))
}

/// After the spawn: connect on `name`, then read replies.
pub(super) fn reach_then(
    name: String,
    kernel: PathBuf,
    feed: Feed,
    sender: Replies,
    wired: mpsc::Sender<io::Result<Half>>,
) -> impl FnOnce(u32, Ends) + Send + 'static {
    move |pid, (stdout, stdin)| match endpoint::reach(stdout, &name, pid, &kernel) {
        Ok((read, write)) => {
            // Connected: the worker no longer watches stdin.
            drop(stdin);
            serve(read, write, &feed, &sender, &wired)
        }
        Err(error) => {
            log::error!("lok: worker {pid} did not connect: {error}");
            refuse(error, &feed, &sender, &wired);
        }
    }
}

/// Hands the request side over, then reads replies.
pub(super) fn serve(read: Half, write: Half, feed: &Feed, sender: &Replies, wired: &mpsc::Sender<io::Result<Half>>) {
    if wired.send(Ok(write)).is_ok() {
        frames::read_frames(BufReader::new(read), feed, sender);
    }
}

/// A start that failed reaches requests and the feed.
pub(super) fn refuse(error: io::Error, feed: &Feed, sender: &Replies, wired: &mpsc::Sender<io::Result<Half>>) {
    let _ = wired.send(Err(io::Error::new(error.kind(), error.to_string())));
    feed.died();
    let _ = sender.send(Err(error));
}
