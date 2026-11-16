//! Linked files: saved values until the user asks (LOK-11).

use std::{io, path::PathBuf, time::Duration};

use ely_files::Fingerprint;
use ely_gpui_component::{
    primitives::{IconName, Severity},
    shell::StatusBarItem,
};
use futures::{StreamExt as _, channel::mpsc};
use gpui::{AppContext as _, Context, Window, actions};
use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};

use crate::{Commands as _, Host, Tabs, engine::Engine, text};

actions!(office, [UpdateLinks]);

/// A save's bursts settle into one look.
const SETTLE: Duration = Duration::from_millis(200);

/// A file source as last looked at.
type Seen = (PathBuf, Result<Fingerprint, io::ErrorKind>);

/// What a document's links show.
#[derive(Default)]
pub struct Links {
    /// The sources the kernel names.
    sources: Vec<String>,
    state: Linked,
    /// The kernel reads the sources now.
    updating: bool,
    /// Each file source as it was when last read.
    seen: Vec<Seen>,
    watcher: Option<RecommendedWatcher>,
}

/// Where a document's links stand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Linked {
    /// The values saved in the file; the missing sources named.
    Saved(Vec<String>),
    /// Read; the failed sources named.
    Updated(Vec<String>),
    /// This source was saved after the update.
    Changed(String),
}

impl Default for Linked {
    fn default() -> Self {
        Self::Saved(Vec::new())
    }
}

impl Links {
    /// As opened; looks at the disk, so off the window.
    pub fn opened(sources: Vec<String>) -> Self {
        let missing = look(&sources)
            .into_iter()
            .filter(|(_, seen)| *seen == Err(io::ErrorKind::NotFound))
            .map(|(path, _)| path.display().to_string())
            .collect();
        Self { sources, state: Linked::Saved(missing), ..Self::default() }
    }

    /// The sources the kernel names after an edit.
    pub fn named(&mut self, sources: Vec<String>) {
        self.sources = sources;
    }

    pub fn sources(&self) -> &[String] {
        &self.sources
    }

    pub fn state(&self) -> &Linked {
        &self.state
    }
}

/// Each file source and its fingerprint now.
fn look(sources: &[String]) -> Vec<Seen> {
    sources
        .iter()
        .filter_map(|source| ely_lok::file_path(source).ok())
        .map(|path| {
            let seen = Fingerprint::of(&path).map_err(|error| error.kind());
            (path, seen)
        })
        .collect()
}

/// The folders of the sources found.
fn folders(seen: &[Seen]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = seen
        .iter()
        .filter(|(_, seen)| seen.is_ok())
        .filter_map(|(path, _)| path.parent().map(PathBuf::from))
        .collect();
    dirs.sort();
    dirs.dedup();
    dirs
}

/// A source as the user knows it: path or name.
fn shown(source: &str) -> String {
    ely_lok::file_path(source).map_or_else(|_| source.to_string(), |path| path.display().to_string())
}

pub trait Linking: Host<Engine = crate::Lok> {
    /// The menu's update, on the active tab.
    fn on_update_links(&mut self, _: &UpdateLinks, _: &mut Window, cx: &mut Context<Self>) {
        match self.core().active() {
            Some(id) => self.update_links(id, cx),
            None => self.toast(text(cx, "command.blocked.no_document", &[]), Severity::Warning, cx),
        }
    }

    /// Reads every link of tab `id` again; the user asked.
    fn update_links(&mut self, id: u64, cx: &mut Context<Self>) {
        let slug = Self::KIND.app.slug();
        let Some(tab) = self.tab_mut(id) else {
            return;
        };
        if tab.links.updating {
            return;
        }
        tab.links.updating = true;
        let sources = tab.links.sources.clone();
        log::info!("{slug}: tab {id} updates links to {} sources", sources.len());
        cx.notify();
        self.write_in(
            id,
            move |worker, doc| {
                // Seen first: a save during the read still shows.
                let seen = look(&sources);
                let dirs = folders(&seen);
                Ok((Self::Engine::update_links(worker, doc)?, seen, dirs))
            },
            move |host, read, _, cx| {
                let Some(tab) = host.tab_mut(id) else {
                    return;
                };
                tab.links.updating = false;
                cx.notify();
                let (failed, seen, dirs) = match read {
                    Ok(read) => read,
                    Err(error) => return host.engine_failed(&error.to_string(), cx),
                };
                log::info!("{slug}: tab {id} links updated, {} failed, {} folders watched", failed.len(), dirs.len());
                let watcher = watch(id, &dirs, host.core().shared.clone(), cx);
                let Some(tab) = host.tab_mut(id) else {
                    return;
                };
                let failed: Vec<String> = failed.iter().map(|source| shown(source)).collect();
                let said = (!failed.is_empty()).then(|| text(cx, "office.links.failed", &[("name", &tab.name), ("sources", &failed.join(", "))]));
                tab.links.state = Linked::Updated(failed);
                tab.links.seen = seen;
                match watcher {
                    Ok(watcher) => tab.links.watcher = Some(watcher),
                    Err(error) => host.unwatched(id, &error.to_string(), cx),
                }
                if let Some(said) = said {
                    host.toast_unlogged(said, Severity::Warning, cx);
                }
            },
            cx,
        );
    }

    /// Something moved in a source's folder; looks again.
    fn sources_moved(&mut self, id: u64, failed: Option<String>, cx: &mut Context<Self>) {
        if let Some(why) = failed {
            return self.unwatched(id, &why, cx);
        }
        let Some(tab) = self.tab(id) else {
            return;
        };
        let before = tab.links.seen.clone();
        let paths: Vec<String> = before.iter().map(|(path, _)| ely_lok::file_url(path)).collect();
        let now = cx.background_spawn(async move { look(&paths) });
        let shared = self.core().shared.clone();
        cx.spawn(async move |_, cx: &mut gpui::AsyncApp| {
            let now = now.await;
            // Tab state only: the window holding it now hears.
            cx.update(|cx| {
                let Some(host) = shared.hosts().into_iter().find(|host| host.read(cx).tab(id).is_some()) else {
                    return log::info!("{}: tab {id} closed before its sources were checked", Self::KIND.app.slug());
                };
                host.update(cx, |host, cx| {
                    let changed = now.iter().zip(&before).find(|(now, then)| now != then);
                    let (Some((source, _)), Some(tab)) = (changed, host.tab_mut(id)) else {
                        return;
                    };
                    if tab.links.seen != before || tab.links.updating {
                        return;
                    }
                    log::info!("{}: tab {id} has a linked source saved since its update", Self::KIND.app.slug());
                    tab.links.state = Linked::Changed(source.0.display().to_string());
                    tab.links.watcher = None;
                    cx.notify();
                })
            })
        })
        .detach();
    }

    /// Later saves of tab `id`'s sources go unseen.
    fn unwatched(&mut self, id: u64, why: &str, cx: &mut Context<Self>) {
        let Some(tab) = self.tab_mut(id) else {
            return;
        };
        tab.links.watcher = None;
        let said = text(cx, "office.links.unwatched", &[("name", &tab.name), ("why", why)]);
        self.toast(said, Severity::Danger, cx);
    }

    /// The active tab's links: as saved, failed or changed (LOK-11).
    fn links_item(&self, cx: &mut Context<Self>) -> Option<StatusBarItem> {
        let tab = self.active_tab()?;
        let links = &tab.links;
        if links.sources.is_empty() {
            return None;
        }
        let (key, named, icon) = match &links.state {
            Linked::Saved(missing) if missing.is_empty() => ("office.links.saved", String::new(), IconName::Link),
            Linked::Saved(missing) => ("office.links.missing", missing.join(", "), IconName::TriangleAlert),
            Linked::Updated(failed) if failed.is_empty() => return None,
            Linked::Updated(failed) => ("office.links.failed", failed.join(", "), IconName::TriangleAlert),
            Linked::Changed(source) => ("office.links.changed", source.clone(), IconName::Link),
        };
        let said = text(cx, key, &[("name", &tab.name), ("sources", &named)]);
        let (this, id) = (cx.entity(), tab.id);
        let item = StatusBarItem::new("office-links").icon(icon).label(text(cx, "office.links.update", &[])).tooltip(said);
        Some(item.on_click(move |_, _, cx| this.update(cx, |host, cx| host.update_links(id, cx))))
    }
}

impl<H: Host<Engine = crate::Lok>> Linking for H {}

/// Watches the folders of tab `id`'s sources.
fn watch<H: Linking>(id: u64, dirs: &[PathBuf], shared: crate::shared::Handle<H>, cx: &mut Context<H>) -> notify::Result<RecommendedWatcher> {
    let (sender, mut changes) = mpsc::unbounded();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        // Reads, the kernel's included, are no change.
        if event.as_ref().is_ok_and(|event| ely_ui::watching::only_read(&event.kind)) {
            return;
        }
        // The receiver ends with the tab.
        let _ = sender.unbounded_send(event.map(|_| ()).map_err(|error| error.to_string()));
    })?;
    for dir in dirs {
        watcher.watch(dir, RecursiveMode::NonRecursive)?;
    }
    cx.spawn(async move |_, cx: &mut gpui::AsyncApp| {
        while let Some(first) = changes.next().await {
            cx.background_executor().timer(SETTLE).await;
            let mut failed = first.err();
            while let Ok(more) = changes.try_recv() {
                failed = more.err().or(failed);
            }
            // The tab may have moved to another window.
            let told = cx.update(|cx| {
                let holder = shared.hosts().into_iter().find(|host| host.read(cx).tab(id).is_some());
                holder.map(|holder| holder.update(cx, |host, cx| host.sources_moved(id, failed, cx)))
            });
            if told.is_none() {
                log::info!("{}: tab {id}'s sources moved, but no window holds it", H::KIND.app.slug());
            }
        }
    })
    .detach();
    Ok(watcher)
}
