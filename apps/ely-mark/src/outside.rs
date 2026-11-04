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
use gpui::{AppContext as _, Context};
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
    /// Bumped by each outside state; older checks lapse.
    pub stamp: u64,
    /// The path the recent list last took from it.
    pub tracked: Option<std::path::PathBuf>,
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
            stamp: 0,
            tracked: None,
            looked: false,
            counting: None,
            counts: None,
            selection: None,
        }
    }
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
        self.look_at(ids, cx);
    }

    /// Compares tab `id`'s file with its last read or write.
    pub fn look(&mut self, id: u64, cx: &mut Context<Self>) {
        self.look_at(vec![id], cx);
    }

    /// The tabs' files, read off the window at once.
    fn look_at(&mut self, ids: Vec<u64>, cx: &mut Context<Self>) {
        let mut asked = Vec::new();
        for id in ids {
            let Some(tab) = self.tab_mut(id) else { continue };
            let Some(path) = tab.session.path().map(Path::to_path_buf) else { continue };
            if tab.exporting || tab.session.state() == SaveState::Saving {
                tab.extra.recheck = true;
                continue;
            }
            tab.extra.recheck = false;
            asked.push((id, path, tab.session.fingerprint(), tab.extra.stamp));
        }
        if asked.is_empty() {
            return;
        }
        cx.spawn(async move |desk, cx| {
            let seen = cx.background_spawn(async move {
                let seen = asked.into_iter().map(|(id, path, ours, stamp)| {
                    let now = Fingerprint::of(&path);
                    (id, path, (ours, stamp), now)
                });
                seen.collect::<Vec<_>>()
            });
            let seen = seen.await;
            desk.update(cx, |desk, cx| {
                for (id, path, began, now) in seen {
                    desk.compared(id, path, began, now, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// The disk as read against the tab now.
    fn compared(
        &mut self,
        id: u64,
        path: PathBuf,
        (ours, stamp): (Option<Fingerprint>, u64),
        now: io::Result<Fingerprint>,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.checked(id, &path, stamp) else {
            return log::info!("mark: a look at tab {id} lapsed");
        };
        // A save began or landed meanwhile: look again.
        if tab.session.fingerprint() != ours || tab.exporting || tab.session.state() == SaveState::Saving {
            return self.look(id, cx);
        }
        match now {
            Ok(now) if Some(now) == ours => {
                if matches!(tab.extra.outside, Outside::Deleted | Outside::Unread(_)) {
                    self.set_outside(id, Outside::Clear, cx);
                }
            }
            Ok(_) => self.read_disk(id, path, cx),
            Err(error) if error.kind() == io::ErrorKind::NotFound => self.missing(id, path, ours, cx),
            Err(error) => self.set_outside(id, Outside::Unread(ely_ui::i18n::error(cx, &error).to_string()), cx),
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
        if let Some(registry) = tab.registry {
            self.core.sessions.replaced(registry, fingerprint.id);
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
            self.set_outside(id, Outside::Unread(ely_ui::i18n::error(cx, &error).to_string()), cx);
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
        // An atomic replace is a new file underneath.
        if let Some(registry) = registry {
            self.core.sessions.replaced(registry, fingerprint.id);
        }
        self.set_outside(id, Outside::Clear, cx);
        view.update(cx, |view, cx| view.saved(&saved, cx));
        Some(view)
    }

    /// Tab `id`'s file state; checks begun before it lapse.
    pub fn set_outside(&mut self, id: u64, outside: Outside, cx: &mut Context<Self>) {
        if let Some(tab) = self.tab_mut(id) {
            tab.extra.outside = outside;
            tab.extra.stamp += 1;
            cx.notify();
        }
    }

    /// Tab `id`, if a check begun at `stamp` holds.
    fn checked(&self, id: u64, path: &Path, stamp: u64) -> Option<&ely_office::Tab<Extra, crate::engine::Mark>> {
        let tab = self.tab(id).filter(|tab| tab.session.path() == Some(path))?;
        (tab.extra.stamp == stamp).then_some(tab)
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

mod disk;

#[cfg(test)]
pub(crate) mod tests;
