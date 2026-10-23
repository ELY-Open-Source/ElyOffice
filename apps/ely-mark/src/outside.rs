//! Changes made to a file outside ElyMark (MARK-08, §14.4).

use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use ely_files::{Fingerprint, Policy, PolicyEngine, SaveState};
use ely_gpui_component::{feedback::Banner, primitives::Severity};
use ely_office::prelude::*;
use futures::{StreamExt as _, channel::mpsc};
use gpui::Context;
use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};

use crate::desk::Desk;
use crate::text::Source;
use ely_ui::i18n::text;

/// Bursts, such as a save's rename, settle into one look.
const SETTLE: Duration = Duration::from_millis(200);
/// An atomic replace empties the path this long at most.
const GAP: Duration = Duration::from_millis(300);

/// What happened to a tab's file on disk.
#[derive(Default)]
pub enum Outside {
    #[default]
    Clear,
    /// Changed under unsaved text: the disk's version, as read.
    Changed(Source, Fingerprint),
    Deleted,
    /// Changes cannot be followed; why.
    Unread(String),
}

/// What ElyMark keeps per tab.
pub struct Extra {
    pub outside: Outside,
    /// The text as last loaded or saved: the merge base.
    pub base: String,
    pub policy: PolicyEngine,
    /// The text change autosave last heard.
    pub heard: u64,
    /// The policy revision an autosave writes.
    pub writing: Option<u64>,
    /// A look put off while a save runs.
    pub recheck: bool,
    /// Its recovery folder's name, once given.
    pub slot: String,
    /// The last recovery point's mark, and one in flight.
    pub pointed: Option<String>,
    pub pointing: bool,
    /// When a point last landed, and the failure last shown.
    pub pointed_at: Option<std::time::SystemTime>,
    pub point_failed: Option<std::io::ErrorKind>,
    /// A kept tab's mode, place and text came back once.
    pub restored: bool,
    /// Disk reads started; only the newest lands.
    pub reads: u64,
    /// The writing view reached its document.
    pub looked: bool,
    /// The text change last counted, and its count.
    pub counting: Option<u64>,
    pub counts: Option<crate::count::Counts>,
    /// The selection, and its words once counted.
    pub selection: Option<(std::sync::Arc<str>, Option<u64>)>,
}

impl Extra {
    /// Autosave from scratch; a write in flight no longer counts.
    pub fn restart_autosave(&mut self) {
        self.policy = PolicyEngine::new(Policy::MarkAutoSave, true);
        self.writing = None;
    }
}

impl Default for Extra {
    fn default() -> Self {
        Extra {
            outside: Outside::Clear,
            base: String::new(),
            policy: PolicyEngine::new(Policy::MarkAutoSave, false),
            heard: 0,
            writing: None,
            recheck: false,
            slot: String::new(),
            pointed: None,
            pointing: false,
            pointed_at: None,
            point_failed: None,
            restored: false,
            reads: 0,
            looked: false,
            counting: None,
            counts: None,
            selection: None,
        }
    }
}

/// The file's bytes and the fingerprint they match.
fn read_still(path: &Path) -> io::Result<(Fingerprint, Source)> {
    // A writer mid-way changes the fingerprint; read again.
    for _ in 0..5 {
        let before = Fingerprint::of(path)?;
        let bytes = std::fs::read(path)?;
        if Fingerprint::of(path)? == before {
            return Ok((before, Source::decode(bytes)?));
        }
    }
    Err(io::Error::other("the file kept changing while read"))
}

impl Desk {
    /// Watches every tab's folder; a change looks at the tabs.
    pub fn watch_folders(&mut self, cx: &mut Context<Self>) {
        let mut dirs: Vec<PathBuf> = self
            .core
            .tabs
            .iter()
            .filter_map(|tab| Some(tab.session.path()?.parent()?.to_path_buf()))
            .collect();
        dirs.sort();
        dirs.dedup();
        if dirs == self.watched {
            return;
        }
        self.watcher = None;
        self.watched = dirs.clone();
        if dirs.is_empty() {
            return;
        }
        match watch(&dirs, cx) {
            Ok(watcher) => self.watcher = Some(watcher),
            Err(error) => self.unwatched(error.to_string(), cx),
        }
    }

    /// Watching failed: every file says so and offers a look.
    fn unwatched(&mut self, why: String, cx: &mut Context<Self>) {
        log::error!("mark: watching files failed: {why}");
        let ids: Vec<u64> = self.core.tabs.iter().map(|tab| tab.id).collect();
        for id in ids {
            self.set_outside(id, Outside::Unread(why.clone()), cx);
        }
    }

    pub fn look_all(&mut self, cx: &mut Context<Self>) {
        // A write, ours or not, may change what Git sees.
        self.ask_git(cx);
        let ids: Vec<u64> = self.core.tabs.iter().map(|tab| tab.id).collect();
        for id in ids {
            self.look(id, cx);
        }
    }

    /// Compares tab `id`'s file with its last read or write.
    pub fn look(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(tab) = self.tab_mut(id) else {
            return;
        };
        let Some(path) = tab.session.path().map(Path::to_path_buf) else {
            return;
        };
        if tab.exporting || tab.session.state() == SaveState::Saving {
            tab.extra.recheck = true;
            return;
        }
        tab.extra.recheck = false;
        let ours = tab.session.fingerprint();
        match Fingerprint::of(&path) {
            Ok(now) if Some(now) == ours => {
                if !matches!(tab.extra.outside, Outside::Changed(..)) {
                    self.set_outside(id, Outside::Clear, cx);
                }
            }
            Ok(_) => self.read_disk(id, path, cx),
            Err(error) if error.kind() == io::ErrorKind::NotFound => self.missing(id, path, cx),
            Err(error) => self.set_outside(id, Outside::Unread(error.to_string()), cx),
        }
    }

    /// Gone: an atomic replace, a rename, or a delete.
    fn missing(&mut self, id: u64, path: PathBuf, cx: &mut Context<Self>) {
        cx.spawn(async move |desk, cx| {
            cx.background_executor().timer(GAP).await;
            desk.update(cx, |desk, cx| desk.still_missing(id, path, cx))
                .ok();
        })
        .detach();
    }

    fn still_missing(&mut self, id: u64, path: PathBuf, cx: &mut Context<Self>) {
        let Some(tab) = self.tab(id).filter(|tab| tab.session.path() == Some(&path)) else {
            return;
        };
        if path.exists() {
            return self.look(id, cx);
        }
        let found = match (tab.registry, path.parent()) {
            (Some(registry), Some(dir)) => self.core.sessions.renamed_in(registry, dir),
            _ => Ok(None),
        };
        match found {
            Ok(Some(to)) => {
                log::info!("mark: {} was renamed outside", path.display());
                self.follow(id, to, cx);
                self.look(id, cx);
            }
            Ok(None) => {
                log::warn!("mark: {} was deleted outside", path.display());
                self.set_outside(id, Outside::Deleted, cx);
            }
            Err(error) => self.set_outside(id, Outside::Unread(error.to_string()), cx),
        }
    }

    /// Changed: a clean tab follows, a dirty one asks.
    fn read_disk(&mut self, id: u64, path: PathBuf, cx: &mut Context<Self>) {
        let Some(tab) = self.tab_mut(id) else {
            return;
        };
        tab.extra.reads += 1;
        let number = tab.extra.reads;
        let reading = path.clone();
        cx.spawn(async move |desk, cx| {
            let read = cx
                .background_executor()
                .spawn(async move { read_still(&reading) })
                .await;
            desk.update(cx, |desk, cx| desk.disk_read(id, path, number, read, cx))
                .ok();
        })
        .detach();
    }

    fn disk_read(
        &mut self,
        id: u64,
        path: PathBuf,
        number: u64,
        read: io::Result<(Fingerprint, Source)>,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tab(id).filter(|tab| tab.session.path() == Some(&path)) else {
            return log::info!("mark: a read for a moved or closed tab was dropped");
        };
        if tab.extra.reads != number {
            return log::info!("mark: read {number} of tab {id} lapsed");
        }
        let (fingerprint, disk) = match read {
            Ok(read) => read,
            Err(error) => return self.set_outside(id, Outside::Unread(error.to_string()), cx),
        };
        // The disk moved on since the read: look again.
        if Fingerprint::of(&path).ok() != Some(fingerprint) {
            log::info!("mark: tab {id}'s file changed while read");
            return self.look(id, cx);
        }
        let current = tab.doc.lock().expect("the text lock").current.clone();
        let kept = disk.text == tab.extra.base;
        let dirty = tab.dirty(cx);
        log::info!("mark: {} changed outside; unsaved {dirty}", path.display());
        match (disk.text == current, dirty) {
            // The same text: only the file underneath is new.
            (true, _) => {
                self.adopt_disk(id, &disk, fingerprint, cx);
            }
            // The text as last saved: the edits stay unsaved.
            (false, true) if kept => self.keep_edits(id, disk, fingerprint, cx),
            (false, false) => self.take_disk(id, disk, fingerprint, cx),
            (false, true) => {
                let tab = self.tab_mut(id).expect("open");
                tab.session.external_modified();
                tab.extra.policy.external_modified();
                self.set_outside(id, Outside::Changed(disk, fingerprint), cx);
                // An open comparison shows the newest disk text.
                if self
                    .comparing
                    .as_ref()
                    .is_some_and(|comparing| comparing.id == id)
                {
                    self.compare(id, cx);
                }
            }
        }
    }

    /// The disk's text replaces the tab's; autosave starts afresh.
    pub fn take_disk(
        &mut self,
        id: u64,
        disk: Source,
        fingerprint: Fingerprint,
        cx: &mut Context<Self>,
    ) {
        if let Some(view) = self.adopt_disk(id, &disk, fingerprint, cx) {
            view.update(cx, |view, cx| view.refresh(disk, cx));
        }
    }

    /// A new file with the saved text; edits stay.
    fn keep_edits(
        &mut self,
        id: u64,
        disk: Source,
        fingerprint: Fingerprint,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tab_mut(id) else {
            return;
        };
        tab.session.rebase(fingerprint);
        tab.doc.lock().expect("the text lock").source = disk;
        // A conflict stopped autosave, maybe mid-write; it resumes.
        tab.extra.restart_autosave();
        let (registry, path) = (tab.registry, tab.session.path().map(Path::to_path_buf));
        if let (Some(registry), Some(path)) = (registry, path)
            && let Err(error) = self.core.sessions.track(registry, &path)
        {
            log::warn!("mark: the replaced file did not register: {error}");
        }
        self.set_outside(id, Outside::Clear, cx);
        self.save_after_pause(id, cx);
    }

    /// The tab and its view take the file as saved.
    pub(crate) fn adopt_disk(
        &mut self,
        id: u64,
        disk: &Source,
        fingerprint: Fingerprint,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Entity<crate::view::MarkView>> {
        let tab = self.tab_mut(id)?;
        if let Err(error) = tab.session.reload_at(fingerprint) {
            self.set_outside(id, Outside::Unread(error.to_string()), cx);
            return None;
        }
        tab.extra.base = disk.text.clone();
        // The file holds the disk's text: it is saved.
        let saved = crate::engine::mark(&disk.text);
        tab.mark = saved.clone();
        tab.extra.restart_autosave();
        // Its bytes (BOM, line ends) are what saves keep now.
        tab.doc.lock().expect("the text lock").source = disk.clone();
        let (view, registry) = (tab.view.clone(), tab.registry);
        let path = tab.session.path().map(Path::to_path_buf);
        // An atomic replace is a new file underneath.
        if let (Some(registry), Some(path)) = (registry, path)
            && let Err(error) = self.core.sessions.track(registry, &path)
        {
            log::warn!("mark: the replaced file did not register: {error}");
        }
        self.set_outside(id, Outside::Clear, cx);
        view.update(cx, |view, cx| view.saved(&saved, cx));
        Some(view)
    }

    pub fn set_outside(&mut self, id: u64, outside: Outside, cx: &mut Context<Self>) {
        if let Some(tab) = self.tab_mut(id) {
            tab.extra.outside = outside;
            cx.notify();
        }
    }

    /// The active file's trouble on disk, with its way out.
    pub fn outside_banner(&self, cx: &mut Context<Self>) -> Option<Banner> {
        let tab = self.active_tab()?;
        let (id, name) = (tab.id, tab.name.to_string());
        let this = cx.entity();
        let (key, why, action, severity) = match &tab.extra.outside {
            Outside::Clear => return None,
            Outside::Changed(..) => (
                "mark.outside.changed",
                None,
                "mark.outside.compare",
                Severity::Warning,
            ),
            Outside::Deleted => (
                "mark.outside.deleted",
                None,
                "mark.outside.save_as",
                Severity::Warning,
            ),
            Outside::Unread(why) => (
                "mark.outside.unread",
                Some(why.clone()),
                "mark.outside.refresh",
                Severity::Danger,
            ),
        };
        let why = why.unwrap_or_default();
        let said = text(cx, key, &[("name", &name), ("why", &why)]);
        let label = text(cx, action, &[]);
        let banner = Banner::new("mark-outside", severity, said);
        Some(banner.action(label, move |_, cx| {
            this.update(cx, |desk, cx| {
                match desk.tab(id).map(|tab| &tab.extra.outside) {
                    Some(Outside::Changed(..)) => desk.compare(id, cx),
                    Some(Outside::Deleted) => desk.save_as(id, None, cx),
                    _ => desk.look(id, cx),
                }
            })
        }))
    }
}

/// Watches `dirs`; each settled burst looks at the tabs.
fn watch(dirs: &[PathBuf], cx: &mut Context<Desk>) -> notify::Result<RecommendedWatcher> {
    let (sender, mut changes) = mpsc::unbounded();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        // Reads, ours included, are no change.
        if event
            .as_ref()
            .is_ok_and(|event| ely_ui::watching::only_read(&event.kind))
        {
            return;
        }
        let _ = sender.unbounded_send(event.map(|_| ()).map_err(|error| error.to_string()));
    })?;
    for dir in dirs {
        watcher.watch(dir, RecursiveMode::NonRecursive)?;
    }
    cx.spawn(async move |desk, cx| {
        while let Some(first) = changes.next().await {
            cx.background_executor().timer(SETTLE).await;
            let mut failed = first.err();
            while let Ok(more) = changes.try_recv() {
                failed = more.err().or(failed);
            }
            let looked = desk.update(cx, |desk, cx| match failed {
                Some(why) => desk.unwatched(why, cx),
                None => desk.look_all(cx),
            });
            if looked.is_err() {
                break;
            }
        }
    })
    .detach();
    log::info!("mark: watching {} folders", dirs.len());
    Ok(watcher)
}

#[cfg(test)]
pub(crate) mod tests;
