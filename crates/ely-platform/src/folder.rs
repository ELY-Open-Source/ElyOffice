//! Where the suite keeps its data, and moving it (§18.1).

use std::{
    io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    AppId,
    lock::FileLock,
    settings::{LOCK, Store},
};

/// The pointer file, in the suite's first folder.
const POINTER: &str = "folder.json";
const KEY: &str = "folder";
/// A picked folder's subfolder; data never mixes with its files.
const NAME: &str = "ElyOffice";
/// Files a running copy holds; never moved.
const LIVE: [&str; 2] = ["instance.lock", "instance.door"];
/// Held by a start until its instance lock is taken.
const ADMIT: &str = ".folder.lock";
/// The first folder's own files, never data.
const KEPT: [&str; 3] = [POINTER, ADMIT, LOCK];
/// Stays in the target until the copy ends.
const MOVING: &str = ".moving";

/// The data folder now, one asked for, a failed move.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub at: PathBuf,
    pub next: Option<PathBuf>,
    pub failed: Option<String>,
}

/// As written; no `at` means the first folder.
#[derive(Default, Serialize, Deserialize)]
struct Saved {
    at: Option<PathBuf>,
    next: Option<PathBuf>,
    failed: Option<String>,
}

/// What the start did with a move asked for.
#[derive(Debug)]
pub enum Moved {
    Nothing,
    Waits(AppId),
    Used(PathBuf),
    Done { from: PathBuf, to: PathBuf, bytes: u64 },
    Failed(String),
}

impl Moved {
    /// Logs the outcome, once logging runs.
    pub fn log(&self) {
        match self {
            Moved::Nothing => {}
            Moved::Waits(app) => log::info!("data folder: {} runs; the move waits", app.name()),
            Moved::Used(to) => log::info!("data folder: {} holds Ely data; used as it is", to.display()),
            Moved::Done { from, to, bytes } => {
                log::info!("data folder: moved {bytes} bytes from {} to {}", from.display(), to.display())
            }
            Moved::Failed(why) => log::error!("data folder: {why}"),
        }
    }
}

/// The data folder now, and any move asked for.
pub fn state() -> io::Result<State> {
    read(&crate::dirs::home().map_err(io::Error::other)?)
}

/// Moves the data into `parent`'s `ElyOffice` at the next start.
pub fn choose(parent: &Path) -> io::Result<State> {
    choose_in(&crate::dirs::home().map_err(io::Error::other)?, parent)
}

/// At start, before the instance lock: the move asked for.
pub fn at_start() -> io::Result<(Moved, FileLock)> {
    at_start_in(&crate::dirs::home().map_err(io::Error::other)?)
}

pub(crate) fn read(home: &Path) -> io::Result<State> {
    let store = Store::at(home.join(POINTER));
    let saved: Saved = match store.read()?.get(KEY) {
        None => Saved::default(),
        Some(value) => serde_json::from_value(value.clone()).map_err(|error| {
            io::Error::new(io::ErrorKind::InvalidData, format!("{}: {error}", store.path().display()))
        })?,
    };
    let at = saved.at.unwrap_or_else(|| home.to_path_buf());
    for path in std::iter::once(&at).chain(&saved.next) {
        if !path.is_absolute() {
            let why = format!("{}: {} is not absolute", store.path().display(), path.display());
            return Err(io::Error::new(io::ErrorKind::InvalidData, why));
        }
    }
    Ok(State { at, next: saved.next, failed: saved.failed })
}

fn write(home: &Path, state: State) -> io::Result<()> {
    let at = (!same(&state.at, home)?).then_some(state.at);
    let saved = Saved { at, next: state.next, failed: state.failed };
    Store::at(home.join(POINTER)).set(KEY, serde_json::to_value(saved)?)
}

/// One folder, however it is spelled.
fn same(a: &Path, b: &Path) -> io::Result<bool> {
    Ok(dunce::canonicalize(a)? == dunce::canonicalize(b)?)
}

fn choose_in(home: &Path, parent: &Path) -> io::Result<State> {
    let mut state = read(home)?;
    let at = dunce::canonicalize(&state.at)?;
    let target = dunce::canonicalize(parent)?.join(NAME);
    let target = if target.try_exists()? { dunce::canonicalize(&target)? } else { target };
    let refuse = |why: String| Err(io::Error::new(io::ErrorKind::InvalidInput, why));
    if target != at && target.starts_with(&at) {
        return refuse(format!("{} is inside the data folder", target.display()));
    }
    if target != at && content(&target)? == Content::Other {
        return refuse(format!("{} holds files that are not Ely's", target.display()));
    }
    state.next = (target != at).then_some(target);
    state.failed = None;
    write(home, state.clone())?;
    match &state.next {
        Some(next) => log::info!("data folder: {} at the next start", next.display()),
        None => log::info!("data folder: the move is called off"),
    }
    Ok(state)
}

#[derive(PartialEq)]
enum Content {
    Empty,
    Partial,
    Ely,
    Other,
}

fn content(dir: &Path) -> io::Result<Content> {
    if !dir.try_exists()? {
        return Ok(Content::Empty);
    }
    if dir.join(MOVING).try_exists()? {
        return Ok(Content::Partial);
    }
    let mut names = std::fs::read_dir(dir)?.map(|entry| entry.map(|entry| entry.file_name()));
    if names.try_fold(true, |empty, name| name.map(|name| empty && KEPT.iter().any(|kept| name == *kept)))? {
        return Ok(Content::Empty);
    }
    for name in AppId::ALL.map(AppId::slug).into_iter().chain(["settings.json"]) {
        if dir.join(name).try_exists()? {
            return Ok(Content::Ely);
        }
    }
    Ok(Content::Other)
}

/// One start at a time; others wait here.
fn at_start_in(home: &Path) -> io::Result<(Moved, FileLock)> {
    std::fs::create_dir_all(home)?;
    let admitted = FileLock::take(&home.join(ADMIT))?;
    Ok((moving(home)?, admitted))
}

fn moving(home: &Path) -> io::Result<Moved> {
    let state = read(home)?;
    if !state.at.try_exists()? {
        let why = format!(
            "{} is missing; connect it, or delete {} to start in {}",
            state.at.display(),
            home.join(POINTER).display(),
            home.display()
        );
        return Err(io::Error::new(io::ErrorKind::NotFound, why));
    }
    let Some(next) = state.next.clone() else {
        return Ok(Moved::Nothing);
    };
    let held = match crate::instance::closed(&state.at)? {
        Ok(held) => held,
        Err(app) => return Ok(Moved::Waits(app)),
    };
    let from = state.at.clone();
    let fail = |why: String| {
        write(home, State { at: from.clone(), next: None, failed: Some(why.clone()) })?;
        Ok(Moved::Failed(why))
    };
    // A link made since the choice resolves here too.
    match inside(&next, &from) {
        Ok(true) => return fail(format!("{} is inside the data folder", next.display())),
        Ok(false) => {}
        Err(error) => return fail(format!("{} cannot be read: {error}", next.display())),
    }
    match content(&next) {
        Ok(Content::Ely) => {
            write(home, State { at: next.clone(), next: None, failed: None })?;
            return Ok(Moved::Used(next));
        }
        Ok(Content::Other) => return fail(format!("{} holds files that are not Ely's", next.display())),
        Ok(Content::Partial) => {
            if let Err(error) = empty(&next, home) {
                return fail(format!("{} keeps a copy cut short: {error}", next.display()));
            }
        }
        Ok(Content::Empty) => {}
        Err(error) => return fail(format!("{} cannot be read: {error}", next.display())),
    }
    let made = matches!(next.try_exists(), Ok(false));
    let bytes = match stage(&from, &next) {
        Ok(bytes) => bytes,
        Err(error) => {
            let why = format!("copying {} to {} failed: {error}", from.display(), next.display());
            let undone = match next.try_exists() {
                Ok(true) => empty(&next, home).and_then(|()| if made { std::fs::remove_dir(&next) } else { Ok(()) }),
                _ => Ok(()),
            };
            let why = match undone {
                Err(gone) => format!("{why}; the part copied stays: {gone}"),
                Ok(()) => why,
            };
            return fail(why);
        }
    };
    write(home, State { at: next.clone(), next: None, failed: None })?;
    drop(held);
    if let Err(error) = clear(&from, home) {
        let why = format!("moved to {}, but {} stays: {error}", next.display(), from.display());
        write(home, State { at: next.clone(), next: None, failed: Some(why.clone()) })?;
        return Ok(Moved::Failed(why));
    }
    Ok(Moved::Done { from, to: next, bytes })
}

/// Whether `target`, links resolved, lies in `at`.
fn inside(target: &Path, at: &Path) -> io::Result<bool> {
    let target = if target.try_exists()? { dunce::canonicalize(target)? } else { target.to_path_buf() };
    Ok(target.starts_with(dunce::canonicalize(at)?))
}

/// Copies under a mark that goes once the copy ends.
fn stage(from: &Path, to: &Path) -> io::Result<u64> {
    std::fs::create_dir_all(to)?;
    std::fs::write(to.join(MOVING), b"")?;
    let bytes = copy(from, to, &KEPT)?;
    std::fs::remove_file(to.join(MOVING))?;
    Ok(bytes)
}

/// Copies `from` into `to`, all but `skip` and live files.
fn copy(from: &Path, to: &Path, skip: &[&str]) -> io::Result<u64> {
    std::fs::create_dir_all(to)?;
    let mut bytes = 0;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let (path, name) = (entry.path(), entry.file_name());
        if skip.iter().chain(&LIVE).any(|skipped| name == *skipped) {
            continue;
        }
        let kind = entry.file_type()?;
        bytes += if kind.is_dir() {
            copy(&path, &to.join(&name), &[])?
        } else if kind.is_file() {
            std::fs::copy(&path, to.join(&name))?
        } else {
            return Err(io::Error::other(format!("{} is neither a file nor a folder", path.display())));
        };
    }
    Ok(bytes)
}

/// Empties the old folder under the mark.
fn clear(old: &Path, home: &Path) -> io::Result<()> {
    let first = same(old, home)?;
    std::fs::write(old.join(MOVING), b"")?;
    empty(old, home)?;
    if first { Ok(()) } else { std::fs::remove_dir(old) }
}

/// Removes all but the first folder's files, mark last.
fn empty(dir: &Path, home: &Path) -> io::Result<()> {
    let kept: &[&str] = if same(dir, home)? { &KEPT } else { &[] };
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == MOVING || kept.iter().any(|kept| name == *kept) {
            continue;
        }
        match entry.file_type()?.is_dir() {
            true => std::fs::remove_dir_all(entry.path())?,
            false => std::fs::remove_file(entry.path())?,
        }
    }
    match std::fs::remove_file(dir.join(MOVING)) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests;
