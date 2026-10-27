//! Recovery points of open documents, back after crashes (§6.3).

use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use ely_files::{Crash, Marker, Recovery, RecoveryKey, Turns};
use ely_gpui_component::primitives::Severity;
use gpui::{AppContext as _, AsyncApp, Context, WeakEntity};
use serde::{Deserialize, Serialize};

use crate::{Engine, Host, Tabs as _, text};

mod reopen;

/// A tab's points, and the edit the last holds.
struct Slot {
    kept: Kept,
    written: Option<u64>,
    writing: bool,
    /// The last failure, shown once until it changes.
    failed: Option<String>,
}

impl Slot {
    /// Notes a failed point; words to show, once each.
    fn fail(&mut self, edited: u64, error: &io::Error) -> Option<String> {
        // Too big for the cap: wait for the next edit.
        if error.kind() == io::ErrorKind::InvalidInput {
            self.written = Some(edited);
        }
        let said = error.to_string();
        (self.failed.as_ref() != Some(&said)).then(|| {
            self.failed = Some(said.clone());
            said
        })
    }
}

/// What reopening a slot needs, kept beside its points.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Kept {
    slot: String,
    name: String,
    format: String,
    sealed: bool,
    /// Not reopened yet; a clean end keeps it.
    #[serde(default)]
    held: bool,
}

/// The app's recovery: marker, slots, what a crash left.
pub struct Keeping {
    dir: PathBuf,
    crash: Option<Crash>,
    slots: HashMap<u64, Slot>,
    crashed: Vec<Kept>,
    /// Copies opening from points: path, point time, old slot.
    reopening: Vec<(PathBuf, SystemTime, Kept)>,
    /// Slots that could not reopen; kept for a later start.
    held: Vec<Kept>,
    /// When the last point was written.
    pub(crate) last: Option<SystemTime>,
    /// The index on disk is behind; the failure shown.
    behind: Option<String>,
    /// An index write runs; another follows it if asked.
    flushing: bool,
    flush_again: bool,
    /// Orders the index writes: the newest lands last.
    turns: Turns,
}

impl Keeping {
    /// Takes the marker; a crash leaves its slots to reopen.
    pub fn start(app: ely_platform::AppId) -> io::Result<Self> {
        Self::start_in(ely_platform::data_dir(app).map_err(io::Error::other)?)
    }

    fn start_in(dir: PathBuf) -> io::Result<Self> {
        let marker = Crash::check(&dir, ely_files::alive)?;
        log::info!("recovery: the last run ended {marker:?}");
        let listed = read_index(&dir)?;
        let crashed = match marker {
            Marker::Crashed => listed,
            // A clean end keeps only what never reopened.
            Marker::Clean | Marker::Discarded => {
                let held: Vec<Kept> = listed.into_iter().filter(|kept| kept.held).collect();
                prune(&dir.join("recovery"), &held)?;
                held
            }
        };
        let crash = Crash::open(&dir, std::process::id())?;
        let (reopening, held) = (Vec::new(), Vec::new());
        Ok(Self { dir, crash: Some(crash), slots: HashMap::new(), crashed, reopening, held, last: None, behind: None, flushing: false, flush_again: false, turns: Turns::default() })
    }

    /// A clean quit: the next start finds nothing to recover.
    pub fn end(&mut self) {
        if let Some(crash) = self.crash.take()
            && let Err(error) = crash.close()
        {
            log::error!("recovery: the clean-quit mark failed: {error}");
        }
    }

    fn folder(&self, slot: &str) -> PathBuf {
        self.dir.join("recovery").join(slot)
    }

    /// Writes the index now, after any write in flight.
    fn save_index(&self) -> io::Result<()> {
        let (path, bytes) = self.index()?;
        let turn = self.turns.take();
        let wrote = self.turns.write(turn, || write_index(&path, &bytes));
        wrote.expect("this thread took the newest turn")
    }

    /// The index file and what it should hold now.
    fn index(&self) -> io::Result<(PathBuf, Vec<u8>)> {
        let waiting = self.reopening.iter().map(|(_, _, kept)| kept).chain(&self.held);
        let waiting = waiting.map(|kept| Kept { held: true, ..kept.clone() });
        let slots = self.slots.values().map(|slot| slot.kept.clone());
        let kept: Vec<Kept> = slots.chain(self.crashed.iter().cloned()).chain(waiting).collect();
        let bytes = serde_json::to_vec(&kept).map_err(io::Error::other)?;
        Ok((self.dir.join("recovery").join("index.json"), bytes))
    }
}

/// Staged then renamed: a failed write keeps the last index.
fn write_index(path: &Path, bytes: &[u8]) -> io::Result<()> {
    std::fs::create_dir_all(path.parent().expect("the index has a folder"))?;
    ely_files::save_atomic(path, bytes, 0).map(|_| ())
}

fn read_index(dir: &Path) -> io::Result<Vec<Kept>> {
    match std::fs::read(dir.join("recovery").join("index.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

/// Removes every slot folder but those `held`.
fn prune(recovery: &Path, held: &[Kept]) -> io::Result<()> {
    if held.is_empty() {
        return remove(recovery);
    }
    for entry in std::fs::read_dir(recovery)? {
        let entry = entry?;
        let name = entry.file_name();
        if entry.file_type()?.is_dir() && !held.iter().any(|kept| *kept.slot == *name) {
            remove(&entry.path())?;
        }
    }
    Ok(())
}

fn remove(path: &Path) -> io::Result<()> {
    match std::fs::remove_dir_all(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// A slot taken from a released tab, waiting to reopen.
pub struct Taken(Kept, String);

/// When recovery points are written and crashes come back.
pub trait Recovering: Host {
    /// From the app's window, never its tests.
    fn start_recovery(&mut self, cx: &mut Context<Self>) {
        match Keeping::start(Self::KIND.app) {
            Ok(keeping) => self.core_mut().keeping = Some(keeping),
            Err(error) => {
                log::error!("recovery: off for this run: {error}");
                self.core_mut().recovery_off = Some(error.to_string());
            }
        }
        Self::keep_points_every(cx);
        cx.on_app_quit(|host, cx| {
            // A write still in flight must not be the last.
            if let Some(keeping) = host.core().keeping.as_ref()
                && let Err(error) = keeping.save_index()
            {
                log::error!("recovery: the index is not saved at quit: {error}");
            }
            // A system end skips asking: unsaved tabs keep points.
            if host.core().tabs.iter().any(|tab| tab.dirty(cx)) {
                log::warn!("recovery: quit with unsaved tabs; points stay");
            } else if let Some(keeping) = host.core_mut().keeping.as_mut() {
                keeping.end();
            }
            async {}
        })
        .detach();
    }

    /// Ticks every ten seconds for the window's life.
    fn keep_points_every(cx: &mut Context<Self>) {
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            loop {
                cx.background_executor().timer(Duration::from_secs(2)).await;
                if this.update(cx, |host, cx| host.keep_points(cx)).is_err() {
                    break;
                }
                cx.background_executor().timer(Recovery::INTERVAL - Duration::from_secs(2)).await;
            }
        })
        .detach();
    }

    /// Reopens what a crash left, then writes due points.
    fn keep_points(&mut self, cx: &mut Context<Self>) {
        // An idle engine reports no error when it ends.
        crate::Stopping::check_engine(self, cx);
        if let Some(error) = self.core_mut().recovery_off.take() {
            self.toast_unlogged(text(cx, "office.recovery.off", &[("error", &error)]), Severity::Danger, cx);
        }
        if self.core().stopped.is_some() {
            return;
        }
        if self.core().keeping.as_ref().is_some_and(|keeping| keeping.behind.is_some()) {
            self.flush_index(cx);
        }
        if self.core().window.is_some() {
            self.reopen_crashed(cx);
        }
        let app = Self::KIND.app;
        let due: Vec<(u64, u64)> = self
            .core()
            .tabs
            .iter()
            .filter(|tab| tab.dirty(cx) && !tab.exporting && Self::Engine::settled(tab.view.read(cx)))
            .map(|tab| (tab.id, Self::Engine::edited(tab.view.read(cx))))
            .filter(|(id, edited)| {
                let slot = self.core().keeping.as_ref().and_then(|keeping| keeping.slots.get(id));
                slot.is_some_and(|slot| !slot.writing && slot.written != Some(*edited))
            })
            .collect();
        for (id, edited) in due {
            let Some(keeping) = self.core_mut().keeping.as_mut() else { return };
            let slot = keeping.slots.get_mut(&id).expect("a due tab has a slot");
            slot.writing = true;
            let kept = slot.kept.clone();
            let folder = keeping.folder(&kept.slot);
            let work = self.tab(id).expect("a due tab").work.join(format!("recovery.{}", kept.format));
            self.read_in(
                id,
                move |worker, doc| {
                    Self::Engine::snapshot(worker, doc, &work, &kept.format)?;
                    let bytes = std::fs::read(&work)?;
                    std::fs::remove_file(&work)?;
                    let limits = ely_platform::storage::limits(app)?;
                    let recovery = match kept.sealed {
                        true => Recovery::sensitive(folder, RecoveryKey::load().key),
                        false => Recovery::new(folder),
                    };
                    let mut recovery = recovery.with_limits(limits.keep, limits.megabytes * 1024 * 1024);
                    recovery.write_once(&bytes, None, SystemTime::now()).map(|_| ())
                },
                move |host, written, _, cx| {
                    let Some(keeping) = host.core_mut().keeping.as_mut() else { return };
                    let Some(slot) = keeping.slots.get_mut(&id) else { return };
                    slot.writing = false;
                    match written {
                        Ok(()) => {
                            slot.written = Some(edited);
                            slot.failed = None;
                            keeping.last = Some(SystemTime::now());
                            if keeping.behind.is_some() {
                                host.flush_index(cx);
                            }
                        }
                        Err(error) => {
                            log::error!("recovery: tab {id} point failed ({:?})", error.kind());
                            let Some(said) = slot.fail(edited, &error) else { return };
                            let shown = text(cx, "office.recovery.failed", &[("error", &said)]);
                            host.toast_unlogged(shown, Severity::Warning, cx);
                        }
                    }
                },
                cx,
            );
        }
    }

    /// Writes the index in the background, one at a time.
    fn flush_index(&mut self, cx: &mut Context<Self>) {
        let Some(keeping) = self.core_mut().keeping.as_mut() else { return };
        if keeping.flushing {
            return keeping.flush_again = true;
        }
        let (path, bytes) = match keeping.index() {
            Ok(index) => index,
            Err(error) => return self.index_failed(error.to_string(), cx),
        };
        keeping.flushing = true;
        let (turn, turns) = (keeping.turns.take(), keeping.turns.clone());
        let task = cx.background_spawn(async move { turns.write(turn, || write_index(&path, &bytes)) });
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let written = task.await;
            this.update(cx, |host, cx| {
                let Some(keeping) = host.core_mut().keeping.as_mut() else { return };
                keeping.flushing = false;
                match written {
                    // A later write, the quit's, holds the newest.
                    None => {}
                    Some(Ok(())) => keeping.behind = None,
                    Some(Err(error)) => host.index_failed(error.to_string(), cx),
                }
                let again = host.core_mut().keeping.as_mut().is_some_and(|keeping| std::mem::take(&mut keeping.flush_again));
                if again {
                    host.flush_index(cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// An index failure shows once and retries.
    fn index_failed(&mut self, error: String, cx: &mut Context<Self>) {
        let Some(keeping) = self.core_mut().keeping.as_mut() else { return };
        if keeping.behind.as_ref() == Some(&error) {
            return;
        }
        log::error!("recovery: the index is not saved: {error}");
        keeping.behind = Some(error.clone());
        self.toast_unlogged(text(cx, "office.recovery.index", &[("error", &error)]), Severity::Danger, cx);
    }

    /// Each tab gets its own folder of points once open.
    fn keep_tab(&mut self, id: u64, name: &str, sealed: bool, cx: &mut Context<Self>) {
        let format = name
            .rsplit_once('.')
            .map(|(_, ext)| ext.to_ascii_lowercase())
            .filter(|ext| Self::KIND.save_as.contains(&ext.as_str()) || Self::KIND.kept.contains(&ext.as_str()))
            .unwrap_or_else(|| Self::KIND.save_as[0].to_string());
        let Some(keeping) = self.core_mut().keeping.as_mut() else { return };
        let slot = format!("{}-{id}", std::process::id());
        let kept = Kept { slot, name: name.to_string(), format, sealed, held: false };
        keeping.slots.insert(id, Slot { kept, written: None, writing: false, failed: None });
        self.flush_index(cx);
    }

    /// A closed tab's points go: closing decided its fate.
    fn forget_tab(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(keeping) = self.core_mut().keeping.as_mut() else { return };
        let Some(slot) = keeping.slots.remove(&id) else { return };
        if let Err(error) = remove(&keeping.folder(&slot.kept.slot)) {
            log::error!("recovery: tab {id} points stay: {error}");
        }
        self.flush_index(cx);
    }

    /// Reopens each crashed slot's newest point as an unsaved copy.
    fn reopen_crashed(&mut self, cx: &mut Context<Self>) {
        let Some(keeping) = self.core_mut().keeping.as_mut() else { return };
        let crashed: Vec<(Kept, String)> = std::mem::take(&mut keeping.crashed).into_iter().map(|kept| { let name = kept.name.clone(); (kept, name) }).collect();
        if crashed.is_empty() {
            return;
        }
        reopen::restore_off(self, crashed, false, |host, done, cx| {
            for (name, restored) in done {
                if let Err(error) = restored {
                    log::error!("recovery: a crashed copy cannot open ({:?})", error.kind());
                    let shown = text(cx, "office.recovery.lost", &[("name", &name), ("error", &error.to_string())]);
                    host.toast_unlogged(shown, Severity::Danger, cx);
                }
            }
        }, cx);
    }

    /// Takes tab `id`'s slot; releasing it keeps the points.
    fn take_slot(&mut self, id: u64, name: String) -> Option<Taken> {
        let slot = self.core_mut().keeping.as_mut()?.slots.remove(&id)?;
        Some(Taken(slot.kept, name))
    }

    /// Reopens taken slots as copies; names any lost.
    fn reopen_taken(&mut self, taken: Vec<Taken>, mut lost: Vec<String>, cx: &mut Context<Self>) {
        let taken = taken.into_iter().map(|Taken(kept, name)| (kept, name)).collect();
        reopen::restore_off(self, taken, true, move |host, done, cx| {
            for (name, restored) in done {
                match restored {
                    Ok(true) => {}
                    Ok(false) => lost.push(name),
                    Err(error) => {
                        log::error!("office: a point does not open ({:?})", error.kind());
                        lost.push(format!("{name} ({error})"));
                    }
                }
            }
            log::info!("{}: the engine is back, {} tab(s) lost", Self::KIND.app.slug(), lost.len());
            if !lost.is_empty() {
                let shown = text(cx, "office.stopped.lost", &[("names", &lost.join(", "))]);
                host.toast_unlogged(shown, Severity::Danger, cx);
            }
        }, cx);
    }

    /// A reopened copy takes over its old points, unsaved.
    fn reopened(&mut self, id: u64, path: &Path, cx: &mut Context<Self>) {
        let Some(keeping) = self.core_mut().keeping.as_mut() else { return };
        let Some(n) = keeping.reopening.iter().position(|(copy, ..)| copy == path) else {
            return;
        };
        let (_, at, kept) = keeping.reopening.remove(n);
        let name = kept.name.clone();
        if let Some(slot) = keeping.slots.get_mut(&id) {
            let sealed = slot.kept.sealed || kept.sealed;
            slot.kept = Kept { sealed, held: false, ..kept };
        }
        // The point it took over is the latest kept.
        keeping.last = Some(keeping.last.map_or(at, |last| last.max(at)));
        self.flush_index(cx);
        let Some(tab) = self.tab_mut(id) else { return };
        tab.session.edit();
        let shown = text(cx, "office.recovery.back", &[("name", &name), ("at", &clock(at))]);
        self.toast_unlogged(shown, Severity::Warning, cx);
    }
}

impl<H: Host> Recovering for H {}

/// A recovery time as this machine's clock reads it.
pub(crate) fn clock(at: SystemTime) -> String {
    match jiff::Timestamp::try_from(at) {
        Ok(at) => at.to_zoned(jiff::tz::TimeZone::system()).strftime("%Y-%m-%d %H:%M").to_string(),
        Err(error) => error.to_string(),
    }
}

#[cfg(test)]
mod tests;
