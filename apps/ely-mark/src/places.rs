//! Folders, files, the recent list, going to a line.

use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use ely_gpui_component::primitives::Severity;
use ely_office::prelude::*;
use ely_ui::i18n::text;
use gpui::{AppContext as _, Context, Window};

use crate::desk::Desk;
use crate::folder::{Folder, markdown_under};

/// How long a failed recent-list entry waits to retry.
pub(crate) const RETRY: Duration = Duration::from_secs(30);

/// Puts `path` first; drops `old` once gone. Off the window.
fn relist(file: &Path, path: &Path, old: Option<PathBuf>) -> io::Result<(PathBuf, Option<Vec<PathBuf>>)> {
    let real = dunce::canonicalize(path)?;
    if old.as_ref() == Some(&real) {
        return Ok((real, None));
    }
    let mut list = ely_files::recent::remember(file, &real)?;
    if let Some(old) = old
        && !old.try_exists()?
    {
        ely_files::recent::forget(file, &old)?;
        list.retain(|known| *known != old);
    }
    Ok((real, Some(list)))
}

impl Desk {
    /// A folder lists beside; a file opens in a tab.
    pub fn open_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if path.is_dir() {
            return self.open_folder(path, cx);
        }
        // A shown file only takes focus; the list still learns.
        let shown = self.core.sessions.is_open(&path).unwrap_or(false);
        self.open_file(path.clone(), cx);
        if shown {
            self.remember(&path, cx);
        }
    }

    /// Lists `root`'s Markdown files off the window (MARK-01).
    pub fn open_folder(&mut self, root: PathBuf, cx: &mut Context<Self>) {
        log::info!("mark: opens folder {}", root.display());
        cx.spawn(async move |desk, cx| {
            let listed = root.clone();
            // The real root: listed files compare with tabs' paths.
            let files = cx
                .background_executor()
                .spawn(async move {
                    let real = dunce::canonicalize(&listed)?;
                    Ok::<_, std::io::Error>((markdown_under(&real)?, real))
                })
                .await;
            desk.update(cx, |desk, cx| match files {
                Ok((files, real)) => {
                    desk.remember(&real, cx);
                    desk.folder = Some(Folder { root: real, files });
                    desk.listings += 1;
                    cx.notify();
                }
                Err(error) => {
                    log::error!("mark: {} did not list: {error}", root.display());
                    let (name, error) = (root.display().to_string(), error.to_string());
                    let said = text(
                        cx,
                        "office.error.open",
                        &[("name", &name), ("error", &error)],
                    );
                    desk.toast(said, Severity::Danger, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// The open folder again, as the disk has it now.
    pub fn relist(&mut self, cx: &mut Context<Self>) {
        if let Some(root) = self.folder.as_ref().map(|folder| folder.root.clone()) {
            self.open_folder(root, cx);
        }
    }

    /// Puts `path` first in the recent list.
    pub fn remember(&mut self, path: &Path, cx: &mut Context<Self>) {
        // One spelling per file: its real path.
        let remembered = dunce::canonicalize(path)
            .and_then(|path| ely_files::recent::remember(&self.recent_file, &path));
        match remembered {
            Ok(list) => self.recent = list,
            Err(error) => self.unlisted(&error, cx),
        }
    }

    fn unlisted(&mut self, error: &io::Error, cx: &mut Context<Self>) {
        log::error!("mark: the recent list did not save: {error}");
        let said = text(cx, "office.error.engine", &[("error", &error.to_string())]);
        self.toast(said, Severity::Danger, cx);
    }

    /// Tab `id` shows `path`; the list follows off the window.
    pub fn track_recent(&mut self, id: u64, path: &Path, cx: &mut Context<Self>) {
        let old = self.remembered.get(&id).cloned();
        let Some(tab) = self.tab_mut(id).filter(|tab| tab.extra.tracked.as_deref() != Some(path)) else {
            return;
        };
        // One attempt at a time; a failure waits `RETRY`.
        tab.extra.tracked = Some(path.to_path_buf());
        let (file, path) = (self.recent_file.clone(), path.to_path_buf());
        let seeking = path.clone();
        cx.spawn(async move |desk, cx| {
            let listed = cx.background_spawn(async move { relist(&file, &seeking, old) }).await;
            desk.update(cx, |desk, cx| desk.listed(id, path, listed, cx)).ok();
        })
        .detach();
    }

    fn listed(&mut self, id: u64, path: PathBuf, listed: io::Result<(PathBuf, Option<Vec<PathBuf>>)>, cx: &mut Context<Self>) {
        if self.tab(id).is_none_or(|tab| tab.extra.tracked.as_ref() != Some(&path)) {
            return log::info!("mark: tab {id} moved on; its recent entry lapsed");
        }
        match listed {
            Ok((real, list)) => {
                self.remembered.insert(id, real);
                if let Some(list) = list {
                    self.recent = list;
                    cx.notify();
                }
                return;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                log::warn!("mark: {} is gone; the recent list waits", path.display());
            }
            Err(error) => self.unlisted(&error, cx),
        }
        cx.spawn(async move |desk, cx| {
            cx.background_executor().timer(RETRY).await;
            desk.update(cx, |desk, _| {
                if let Some(tab) = desk.tab_mut(id).filter(|tab| tab.extra.tracked.as_ref() == Some(&path)) {
                    tab.extra.tracked = None;
                }
            })
            .ok();
        })
        .detach();
    }

    /// Takes a path out of the recent list.
    pub fn forget(&mut self, path: &std::path::Path, cx: &mut Context<Self>) {
        match ely_files::recent::forget(&self.recent_file, path) {
            Ok(()) => {
                self.recent.retain(|known| known != path);
                log::info!("mark: a file left the recent list");
                cx.notify();
            }
            Err(error) => self.unlisted(&error, cx),
        }
    }

    /// Empties the recent list.
    pub fn clear_recent(&mut self, cx: &mut Context<Self>) {
        match ely_files::recent::clear(&self.recent_file) {
            Ok(()) => {
                log::info!("mark: the recent list is cleared");
                self.recent.clear();
                cx.notify();
            }
            Err(error) => {
                log::error!("mark: the recent list did not clear: {error}");
                let said = text(cx, "office.error.engine", &[("error", &error.to_string())]);
                self.toast(said, Severity::Danger, cx);
            }
        }
    }

    /// Line and column `at` in a tab or file.
    pub fn open_at(
        &mut self,
        tab: Option<u64>,
        file: PathBuf,
        at: (usize, usize),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let held = tab.or_else(|| self.tab_of(&file));
        if let Some(id) = held.filter(|id| self.tab(*id).is_some()) {
            self.core.active = Some(id);
            if let Some(view) = self.tab(id).map(|open| open.view.clone()) {
                // After the list's own key handling, which keeps focus.
                window.defer(cx, move |window, cx| {
                    view.update(cx, |view, cx| view.reveal(at, window, cx))
                });
            }
            return cx.notify();
        }
        // A file gone is reported by the open itself.
        if let Ok(real) = dunce::canonicalize(&file) {
            self.going = Some((real, at));
        }
        self.open_file(file, cx);
    }

    /// The tab showing `path`, by real path.
    pub fn tab_of(&self, path: &std::path::Path) -> Option<u64> {
        let real = dunce::canonicalize(path).ok()?;
        let held = |tab: &&ely_office::Tab<crate::outside::Extra, crate::engine::Mark>| {
            tab.session
                .path()
                .and_then(|open| dunce::canonicalize(open).ok())
                == Some(real.clone())
        };
        self.core.tabs.iter().find(held).map(|tab| tab.id)
    }

    /// Tab `id`'s text now, unsaved edits included.
    pub fn text_of(&self, id: u64) -> Option<String> {
        Some(
            self.tab(id)?
                .doc
                .lock()
                .expect("the text lock")
                .current
                .clone(),
        )
    }
}
