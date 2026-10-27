//! The session kept across launches (MARK-12, §6.2, §6.3).

use std::{
    io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use ely_files::{Crash, Marker, Recovery};
use ely_office::prelude::*;
use ely_ui::i18n::text;
use gpui::{AppContext as _, Context};
use serde::{Deserialize, Serialize};

use crate::desk::Desk;
use crate::engine::mark;
use crate::view::{Mode, Place};

mod store;
use store::{Turns, sweep_logged};

/// A kept tab: file, mode, place, recovery folder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Kept {
    path: Option<PathBuf>,
    name: String,
    mode: Mode,
    place: Option<Place>,
    slot: String,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
struct Saved {
    tabs: Vec<Kept>,
}

/// A kept tab yet to show; recovered text copied.
struct Waiting {
    kept: Kept,
    real: Option<PathBuf>,
    text: Option<(String, SystemTime)>,
    /// The untitled tab made for it, once made.
    tab: Option<u64>,
}

/// The session's files and what still waits to restore.
pub struct Keeping {
    dir: PathBuf,
    crash: Option<Crash>,
    waiting: Vec<Waiting>,
    written: Option<String>,
    /// The kind of session write failure last shown.
    unwritten: Option<io::ErrorKind>,
    /// A quit began: the kept list stays as then.
    pub leaving: bool,
    /// The list as kept when the quit began.
    frozen: Vec<Kept>,
    turns: Turns,
}

fn recovery(dir: &Path, slot: &str) -> PathBuf {
    dir.join("recovery").join(slot)
}

/// The newest point's text in `slot`, after a crash.
fn recovered(dir: &Path, slot: &str) -> io::Result<Option<(String, SystemTime)>> {
    let Some(point) = Recovery::new(recovery(dir, slot))
        .points()?
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let bytes = std::fs::read(&point.content)?;
    let text = String::from_utf8(bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(Some((text, point.at)))
}

impl Keeping {
    /// Reads the last session; a crash brings its unsaved text.
    pub fn start(dir: &Path) -> io::Result<(Keeping, Vec<PathBuf>, Vec<String>)> {
        let marker = Crash::check(dir, ely_files::alive)?;
        log::info!("mark: the last session ended {marker:?}");
        let crash = Crash::open(dir, std::process::id())?;
        let saved: Saved = match std::fs::read(dir.join("session.json")) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Saved::default(),
            Err(error) => return Err(error),
        };
        let (mut waiting, mut missing) = (Vec::new(), Vec::new());
        for kept in saved.tabs {
            let text = match marker {
                Marker::Crashed => recovered(dir, &kept.slot)?,
                Marker::Clean | Marker::Discarded => None,
            };
            if let Some(path) = &kept.path {
                match dunce::canonicalize(path) {
                    // The file reopens as it is on disk.
                    Ok(real) => waiting.push(Waiting {
                        kept: Kept {
                            slot: String::new(),
                            ..kept.clone()
                        },
                        real: Some(real),
                        text: None,
                        tab: None,
                    }),
                    Err(error) => {
                        log::warn!("mark: a kept file does not open: {error}");
                        missing.push(format!("{}: {error}", path.display()));
                    }
                }
            }
            // Recovered text never overwrites a file: it is a copy.
            if text.is_some() {
                let kept = Kept { path: None, ..kept };
                waiting.push(Waiting {
                    kept,
                    real: None,
                    text,
                    tab: None,
                });
            }
        }
        if marker != Marker::Crashed {
            clear(&dir.join("recovery"))?;
        }
        let paths = waiting
            .iter()
            .filter_map(|waiting| waiting.real.clone())
            .collect();
        let keeping = Keeping {
            dir: dir.to_path_buf(),
            crash: Some(crash),
            waiting,
            written: None,
            unwritten: None,
            leaving: false,
            frozen: Vec::new(),
            turns: Turns::default(),
        };
        Ok((keeping, paths, missing))
    }

    /// The untitled tabs to make: their place here and name.
    pub fn untitled(&self) -> Vec<(usize, String)> {
        let waiting = self.waiting.iter().enumerate();
        let untitled =
            waiting.filter(|(_, waiting)| waiting.kept.path.is_none() && waiting.tab.is_none());
        untitled
            .map(|(at, waiting)| (at, waiting.kept.name.clone()))
            .collect()
    }

    /// Entry `at` belongs to untitled tab `tab`.
    pub fn made(&mut self, at: usize, tab: u64) {
        self.waiting[at].tab = Some(tab);
    }

    /// What tab `id` takes: its file or its copy.
    fn take(&mut self, real: Option<&PathBuf>, id: u64) -> Option<Waiting> {
        let at = self
            .waiting
            .iter()
            .position(|waiting| match (&waiting.real, real) {
                (Some(kept), Some(open)) => kept == open,
                (None, None) => waiting.tab == Some(id),
                _ => false,
            })?;
        Some(self.waiting.remove(at))
    }

    /// An end that never asked: the lock and points stay.
    pub fn hold(&mut self) {
        if self.crash.take().is_some() {
            log::warn!("mark: the session ends unasked; its points stay");
        }
    }

    /// A clean end: no crash next time, no stale points.
    pub fn close(&mut self) {
        let Some(crash) = self.crash.take() else {
            return;
        };
        if let Err(error) = crash
            .close()
            .and_then(|()| clear(&self.dir.join("recovery")))
        {
            log::error!("mark: the session did not close cleanly: {error}");
        }
    }
}

fn session_json(saved: Saved) -> String {
    serde_json::to_string_pretty(&saved).expect("the session serializes")
}

fn clear(dir: &Path) -> io::Result<()> {
    match std::fs::remove_dir_all(dir) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

impl Desk {
    /// A tab just shown takes its kept mode, place, text.
    pub fn restore_shown(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(tab) = self.tab_mut(id).filter(|tab| !tab.extra.restored) else {
            return;
        };
        // Each tab takes one kept entry, on its first show.
        tab.extra.restored = true;
        let tab = self.tab(id).expect("open");
        let real = tab
            .session
            .path()
            .and_then(|path| dunce::canonicalize(path).ok());
        let Some(Waiting {
            kept, text: back, ..
        }) = self.keeping.take(real.as_ref(), id)
        else {
            return;
        };
        let mode = kept.mode;
        log::info!("mark: restores tab {id} as {}", mode.name());
        let tab = self.tab_mut(id).expect("open");
        // A recovered copy keeps writing to the same points.
        tab.extra.slot = kept.slot.clone();
        // Its point stands until a newer lands.
        tab.extra.pointed_at = back.as_ref().map(|(_, at)| *at);
        let view = tab.view.clone();
        if let Some((_, at)) = &back {
            let at = humantime(*at);
            let said = text(
                cx,
                "mark.restore.recovered",
                &[("name", &kept.name), ("at", &at)],
            );
            self.toast(said, ely_gpui_component::primitives::Severity::Info, cx);
        }
        let back = back.map(|(back, _)| back);
        // Modes take focus: in the window, after this update.
        cx.spawn(async move |desk, cx| {
            let shown = ely_office::in_window(&desk, cx, |_, window, cx| {
                view.update(cx, |view, cx| {
                    view.reopen(mode, kept.place, back, window, cx)
                })
            });
            if let Err(error) = shown {
                log::warn!("mark: tab {id} did not restore: {error:#}");
            }
        })
        .detach();
    }

    /// Every ten seconds: unsaved text to points, tabs to disk.
    pub fn keep(&mut self, cx: &mut Context<Self>) {
        let (live, kept) = self.keep_points(cx);
        let json = (!self.keeping.leaving)
            .then(|| {
                self.keeping.frozen = kept.clone();
                session_json(Saved { tabs: kept })
            })
            .filter(|json| self.keeping.written.as_ref() != Some(json));
        let turn = json.as_ref().map(|_| self.keeping.turns.take());
        let (dir, turns) = (self.keeping.dir.clone(), self.keeping.turns.clone());
        // Disk work leaves the window's thread.
        let task = cx.background_spawn(async move {
            sweep_logged(&dir, &live);
            let json = json?;
            let wrote = turns.write(turn?, &dir.join("session.json"), json.as_bytes())?;
            Some((json, wrote))
        });
        cx.spawn(async move |desk, cx| {
            if let Some((json, wrote)) = task.await {
                desk.update(cx, |desk, cx| desk.session_written(json, wrote, cx)).ok();
            }
        })
        .detach();
    }

    /// Writes due points; the live slots and the kept list.
    fn keep_points(&mut self, cx: &mut Context<Self>) -> (Vec<String>, Vec<Kept>) {
        let dir = self.keeping.dir.clone();
        let started = std::time::UNIX_EPOCH
            .elapsed()
            .map_or(0, |since| since.as_millis());
        let mut kept = Vec::new();
        for tab in &mut self.core.tabs {
            if tab.extra.slot.is_empty() {
                tab.extra.slot = format!("{started}-{}", tab.id);
            }
            let view = tab.view.read(cx);
            kept.push(Kept {
                path: tab.session.path().map(Path::to_path_buf),
                name: tab.name.to_string(),
                mode: view.mode,
                place: Some(view.place(cx)),
                slot: tab.extra.slot.clone(),
            });
            let current = tab.doc.lock().expect("the text lock").current.clone();
            let point = mark(&current);
            if !tab.dirty(cx) || tab.extra.pointed.as_ref() == Some(&point) || tab.extra.pointing {
                continue;
            }
            tab.extra.pointing = true;
            let (id, slot) = (tab.id, recovery(&dir, &tab.extra.slot));
            cx.spawn(async move |desk, cx| {
                let folder = slot.clone();
                let wrote = cx
                    .background_executor()
                    .spawn(async move {
                        let now = SystemTime::now();
                        let limits = ely_platform::storage::limits(ely_platform::AppId::Mark)?;
                        let mut recovery = Recovery::new(folder)
                            .with_limits(limits.keep, limits.megabytes * 1024 * 1024);
                        recovery.write_once(current.as_bytes(), None, now)
                    })
                    .await;
                desk.update(cx, |desk, cx| desk.pointed(id, point, wrote, &slot, cx))
                    .ok();
            })
            .detach();
        }
        // Closed or discarded tabs' points go (§6.2).
        let waiting = self.keeping.waiting.iter().map(|waiting| &waiting.kept);
        let live = kept.iter().chain(waiting).map(|kept| kept.slot.clone()).collect();
        (live, kept)
    }

    fn pointed(
        &mut self,
        id: u64,
        point: String,
        wrote: io::Result<Option<PathBuf>>,
        slot: &Path,
        cx: &mut Context<Self>,
    ) {
        // The tab closed while it wrote: the point goes too.
        let Some(tab) = self.tab_mut(id) else {
            if let Err(error) = clear(slot) {
                log::error!("mark: a closed tab's point stays: {error}");
            }
            return;
        };
        tab.extra.pointing = false;
        let error = match wrote {
            Ok(_) => {
                tab.extra.pointed = Some(point);
                tab.extra.pointed_at = Some(SystemTime::now());
                tab.extra.point_failed = None;
                return;
            }
            Err(error) => error,
        };
        log::error!("mark: a recovery point for tab {id} failed: {error}");
        // Each kind of failure shows once; the next tick retries.
        if tab.extra.point_failed == Some(error.kind()) {
            return;
        }
        tab.extra.point_failed = Some(error.kind());
        let error = error.to_string();
        let at = match tab.extra.pointed_at {
            Some(at) => humantime(at),
            None => text(cx, "office.stopped.never", &[]).to_string(),
        };
        let name = tab.name.to_string();
        let said = text(
            cx,
            "mark.recovery.failed",
            &[("name", &name), ("error", &error), ("at", &at)],
        );
        self.toast_unlogged(said, ely_gpui_component::primitives::Severity::Danger, cx);
    }

    /// Writes the list now: a quit needs it on disk.
    fn write_session(&mut self, saved: Saved, cx: &mut Context<Self>) {
        let json = session_json(saved);
        if self.keeping.written.as_ref() == Some(&json) {
            return;
        }
        let file = self.keeping.dir.join("session.json");
        let turn = self.keeping.turns.take();
        let wrote = (self.keeping.turns.write(turn, &file, json.as_bytes()))
            .expect("this thread took the newest turn");
        self.session_written(json, wrote, cx);
    }

    fn session_written(&mut self, json: String, wrote: io::Result<()>, cx: &mut Context<Self>) {
        let error = match wrote {
            Ok(()) => {
                self.keeping.written = Some(json);
                self.keeping.unwritten = None;
                return;
            }
            Err(error) => error,
        };
        log::error!("mark: the session did not save: {error}");
        // Each kind of failure shows once; the next tick retries.
        if self.keeping.unwritten != Some(error.kind()) {
            self.keeping.unwritten = Some(error.kind());
            let said = text(cx, "mark.session.failed", &[("error", &error.to_string())]);
            self.toast(said, ely_gpui_component::primitives::Severity::Danger, cx);
        }
    }

    /// Keeps every ten seconds while the window lives (§6.2).
    pub fn keep_every(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |desk, cx| {
            loop {
                cx.background_executor().timer(Recovery::INTERVAL).await;
                if desk.update(cx, |desk, cx| desk.keep(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    /// Slot `slot` closed: its points go, untitled entries too.
    pub fn drop_slot(
        &mut self,
        slot: &str,
        path: Option<PathBuf>,
        name: String,
        cx: &mut Context<Self>,
    ) {
        if !slot.is_empty()
            && let Err(error) = clear(&recovery(&self.keeping.dir, slot))
        {
            log::error!("mark: a closed tab's points stay: {error}");
        }
        if !self.keeping.leaving {
            return self.keep(cx);
        }
        // Discarded untitled text goes; a first save keeps its file.
        match path {
            None => self.keeping.frozen.retain(|kept| kept.slot != slot),
            Some(path) => {
                let saved = self
                    .keeping
                    .frozen
                    .iter_mut()
                    .filter(|kept| kept.slot == slot);
                for kept in saved {
                    (kept.path, kept.name) = (Some(path.clone()), name.clone());
                }
            }
        }
        let tabs = self.keeping.frozen.clone();
        self.write_session(Saved { tabs }, cx);
    }

    /// A quit or close begins: the tabs now stay kept.
    pub fn leave(&mut self, cx: &mut Context<Self>) {
        let (live, kept) = self.keep_points(cx);
        sweep_logged(&self.keeping.dir, &live);
        self.keeping.frozen = kept.clone();
        self.write_session(Saved { tabs: kept }, cx);
        self.keeping.leaving = true;
    }
}

/// A recovery time as this machine's clock reads it.
fn humantime(at: SystemTime) -> String {
    match jiff::Timestamp::try_from(at) {
        Ok(at) => at
            .to_zoned(jiff::tz::TimeZone::system())
            .strftime("%Y-%m-%d %H:%M")
            .to_string(),
        Err(error) => {
            log::warn!("mark: a recovery time is out of range: {error}");
            error.to_string()
        }
    }
}

#[cfg(test)]
mod tests;
