//! Folders, files, the recent list, going to a line.

use std::path::PathBuf;

use ely_gpui_component::primitives::Severity;
use ely_office::prelude::*;
use ely_ui::i18n::text;
use gpui::{Context, Window};

use crate::desk::Desk;
use crate::folder::{Folder, markdown_under};

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
    pub fn remember(&mut self, path: &std::path::Path, cx: &mut Context<Self>) {
        // One spelling per file: its real path.
        let remembered = dunce::canonicalize(path)
            .and_then(|path| ely_files::recent::remember(&self.recent_file, &path));
        match remembered {
            Ok(list) => self.recent = list,
            Err(error) => {
                log::error!("mark: the recent list did not save: {error}");
                self.toast(
                    text(cx, "office.error.engine", &[("error", &error.to_string())]),
                    Severity::Danger,
                    cx,
                );
            }
        }
    }

    /// Tab `id` shows `path`; the recent list follows.
    pub fn track_recent(&mut self, id: u64, path: &std::path::Path, cx: &mut Context<Self>) {
        let Ok(real) = dunce::canonicalize(path) else {
            return log::warn!("mark: {} is gone; the recent list keeps it", path.display());
        };
        if self.remembered.get(&id) == Some(&real) {
            return;
        }
        if let Some(old) = self.remembered.insert(id, real)
            && !old.exists()
        {
            self.forget(&old, cx);
        }
        self.remember(path, cx);
    }

    /// Takes a path out of the recent list.
    pub fn forget(&mut self, path: &std::path::Path, cx: &mut Context<Self>) {
        match ely_files::recent::forget(&self.recent_file, path) {
            Ok(()) => {
                self.recent.retain(|known| known != path);
                log::info!("mark: a file left the recent list");
                cx.notify();
            }
            Err(error) => {
                log::error!("mark: the recent list did not save: {error}");
                let said = text(cx, "office.error.engine", &[("error", &error.to_string())]);
                self.toast(said, Severity::Danger, cx);
            }
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
