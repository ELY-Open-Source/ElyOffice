//! Save, save as, and conflict outcomes (FILE-04, FILE-05).

use std::{
    io,
    path::{Path, PathBuf},
};

use ely_files::{Open, SaveError, Saved};
use ely_gpui_component::primitives::Severity;
use gpui::{Context, TaskExt as _, Window};

use crate::files::{self, file_name};
use crate::{After, Closing, Dialog, Engine, Host, Opening, Tabs, in_window, text};

mod gone;
mod landing;
mod paths;
pub use paths::folder;
use landing::Target;
use paths::identity;

pub trait Saving: Host {
    /// The tab whose source file is `target`.
    fn holder(&self, target: &Path) -> io::Result<Option<u64>> {
        // Target errors stand; a tab's gone volume does not.
        let target = identity(target)?;
        for tab in &self.core().tabs {
            // A running Save As holds its target already.
            for open in tab.session.path().into_iter().chain(tab.session.writing()) {
                match ely_files::same_file(open, &target) {
                    Ok(true) => return Ok(Some(tab.id)),
                    Ok(false) => {}
                    Err(error) => log::warn!("office: tab {} has no file to compare: {error}", tab.id),
                }
            }
        }
        Ok(None)
    }

    /// A save as refused unwritten; a waiting close stops.
    fn refuse_save(&mut self, then: &Option<After>, words: gpui::SharedString, cx: &mut Context<Self>) {
        self.toast(words, Severity::Danger, cx);
        if then.is_some() {
            self.close_stopped(cx);
        }
    }

    /// Tab `id` went before its save; its close goes on.
    fn closed_first(&mut self, id: u64, then: Option<After>, window: &mut Window, cx: &mut Context<Self>) {
        log::info!("office: tab {id} closed before its save");
        if let Some(after) = then {
            self.proceed(after, window, cx);
        }
    }

    /// Saves in place in its own format.
    fn save(&mut self, id: u64, then: Option<After>, window: &mut Window, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |host, window, cx| host.save(id, then, window, cx));
        }
        if self.exporting(id) {
            return self.after_export(id, move |host, window, cx| host.save(id, then, window, cx));
        }
        let Some(tab) = self.tab(id) else {
            return self.closed_first(id, then, window, cx);
        };
        let Some(path) = tab
            .session
            .path()
            .filter(|_| tab.risk.is_none())
            .map(PathBuf::from)
        else {
            return self.save_as(id, then, cx);
        };
        let format = match files::kept_format(&Self::KIND, &path) {
            Ok(format) => format,
            Err(error) => {
                return self.finish(id, Err(SaveError::Io(error)), None, then, window, cx);
            }
        };
        landing::export(self, id, format, then, Target::Own, cx);
    }

    /// Whether tab `id` has an export running.
    fn exporting(&self, id: u64) -> bool {
        self.tab(id).is_some_and(|tab| tab.exporting)
    }

    /// Runs `then` once tab `id`'s running export ends.
    fn after_export(
        &mut self,
        id: u64,
        then: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) {
        log::info!("office: a save of tab {id} waits for the running one");
        self.core_mut().queued_saves.push((id, Box::new(then)));
    }

    /// Asks where, then writes the default format there.
    fn save_as(&mut self, id: u64, then: Option<After>, cx: &mut Context<Self>) {
        let Some(tab) = self.tab(id) else {
            return;
        };
        let Some(dir) = folder(tab) else {
            return self.panel_failed(&ely_ui::i18n::text(cx, "error.no_documents", &[]), cx);
        };
        let stem = tab
            .name
            .rsplit_once('.')
            .map_or(tab.name.as_ref(), |(stem, _)| stem);
        log::info!("office: asks where to save tab {id}");
        let chosen =
            cx.prompt_for_new_path(&dir, Some(&format!("{stem}.{}", Self::KIND.save_as[0])));
        cx.spawn(async move |this, cx| {
            let chosen = match chosen.await {
                Ok(Ok(path)) => Ok(path),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            in_window(&this, cx, |host, window, cx| {
                match chosen {
                    Ok(Some(path)) => return host.write_as(id, path, then, window, cx),
                    Ok(None) => log::info!("office: the save panel was cancelled"),
                    Err(error) => host.panel_failed(&error, cx),
                }
                // No save: a close that waited on it stops.
                if then.is_some() {
                    host.close_stopped(cx);
                }
            })
        })
        .detach_and_log_err(cx);
    }

    /// Writes `id` to `path`; conversions need new files.
    fn write_as(
        &mut self,
        id: u64,
        path: PathBuf,
        then: Option<After>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |host, window, cx| {
                host.write_as(id, path, then, window, cx)
            });
        }
        if self.exporting(id) {
            return self.after_export(id, move |host, window, cx| {
                host.write_as(id, path, then, window, cx)
            });
        }
        // A gone tab's close goes on, whatever the target.
        let Some(tab) = self.tab(id) else {
            return self.closed_first(id, then, window, cx);
        };
        let name = tab.name.clone();
        let unsaved = |error: io::Error, cx: &mut Context<Self>| {
            text(cx, "office.error.save", &[("name", &name), ("error", &error.to_string())])
        };
        let target = match identity(&path) {
            Ok(target) => target,
            Err(error) => return self.refuse_save(&then, unsaved(error, cx), cx),
        };
        let Some(format) = files::save_as_format(&Self::KIND, &target) else {
            let formats = Self::KIND.save_as.join(", .");
            let text = text(cx, "office.error.format", &[("formats", &formats)]);
            return self.refuse_save(&then, text, cx);
        };
        let holder = match self.holder(&target) {
            Ok(holder) => holder,
            Err(error) => return self.refuse_save(&then, unsaved(error, cx), cx),
        };
        if holder.is_some_and(|other| other != id) {
            let text = text(cx, "office.error.taken", &[("name", &file_name(&target))]);
            return self.refuse_save(&then, text, cx);
        }
        let own_format = tab
            .session
            .path()
            .and_then(|open| files::kept_format(&Self::KIND, open).ok());
        if holder == Some(id) && (tab.risk.is_some() || own_format != Some(format)) {
            let text = text(cx, "office.error.convert", &[]);
            return self.refuse_save(&then, text, cx);
        }
        // The written file differs from the open one; reopen it.
        let reopen =
            matches!(tab.risk, Some(crate::Risk::Signed | crate::Risk::Repaired)) && then.is_none();
        landing::export(self, id, format, then, Target::As(target, reopen), cx);
    }

    /// The tab now names the saved file: registry, title, risk.
    fn moved(&mut self, id: u64, done: &Saved, cx: &mut Context<Self>) {
        let tab = self.tab(id).expect("the tab is open");
        if let Some(old) = tab.registry {
            self.core_mut().sessions.close(old);
        }
        let registry = match self.core_mut().sessions.open(&done.path, false) {
            Ok(Open::Fresh(registry) | Open::View(registry) | Open::Focus(registry)) => {
                Some(registry)
            }
            Err(error) => {
                self.report(id, Err(SaveError::Io(error)), cx);
                None
            }
        };
        // Ely's writes carry no signatures; macros may stay.
        let risk = match files::macro_risk(&Self::KIND, &done.path) {
            Ok(macros) => macros.then_some(crate::Risk::Macros),
            Err(error) => {
                self.report(id, Err(SaveError::Io(error)), cx);
                Some(crate::Risk::Macros)
            }
        };
        let tab = self.tab_mut(id).expect("the tab is open");
        tab.registry = registry;
        tab.name = file_name(&done.path);
        tab.risk = risk;
    }

    /// Keep Mine over the disk now; later changes conflict.
    fn keep_mine(
        &mut self,
        id: u64,
        then: Option<After>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |host, window, cx| {
                host.keep_mine(id, then, window, cx)
            });
        }
        if self.exporting(id) {
            return self.after_export(id, move |host, window, cx| {
                host.keep_mine(id, then, window, cx)
            });
        }
        let Some(tab) = self.tab(id) else {
            return self.closed_first(id, then, window, cx);
        };
        let format = tab
            .session
            .path()
            .ok_or_else(|| io::Error::other("no path"))
            .and_then(|path| files::kept_format(&Self::KIND, path));
        let format = match format {
            Ok(format) => format,
            Err(error) => {
                return self.finish(id, Err(SaveError::Io(error)), None, then, window, cx);
            }
        };
        let seen = tab.session.path().map(ely_files::Fingerprint::of);
        match seen {
            Some(Ok(seen)) => {
                self.tab_mut(id).expect("open").session.rebase(seen);
                landing::export(self, id, format, then, Target::Own, cx);
            }
            // Gone from disk: nothing there to lose.
            _ => landing::export(self, id, format, then, Target::Over, cx),
        }
    }

    /// Reopens the tab from its file; the disk version wins.
    fn reload(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(path) = self
            .tab(id)
            .and_then(|tab| tab.session.path().map(PathBuf::from))
        else {
            return;
        };
        self.close_tab(id, cx);
        self.open_file(path, cx);
    }

    /// Reopens a flagged tab once nothing in memory is unsaved.
    fn reopen_if_saved(&mut self, id: u64, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            in_window(&this, cx, |host, _, cx| {
                let Some(tab) = host.tab(id).filter(|tab| tab.reopen) else {
                    return;
                };
                match !tab.dirty(cx) && !tab.exporting {
                    true => {
                        log::info!("office: tab {id} reopens the file it wrote");
                        host.reload(id, cx);
                    }
                    false => log::info!(
                        "office: tab {id} has edits since; it reopens after its next save"
                    ),
                }
            })
        })
        .detach_and_log_err(cx);
    }

    /// `history` is the kernel's undo history read with the export.
    fn finish(
        &mut self,
        id: u64,
        saved: Result<Saved, SaveError>,
        history: Option<String>,
        then: Option<After>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ended = match &saved {
            Ok(done) => Ok(done.path.as_path()),
            Err(SaveError::Conflict) => Err("the file changed on disk since it was opened".to_string()),
            Err(SaveError::Refused(state)) => Err(format!("the save was refused: {state:?}")),
            Err(SaveError::Io(error)) => Err(error.to_string()),
        };
        self.core_mut().answer_saves(id, ended);
        match saved {
            Ok(done) => {
                log::info!("office: saved {}", done.path.display());
                // A save may replace the file; the registry follows it.
                let registry = self.tab(id).and_then(|tab| tab.registry);
                if let Some(registry) = registry {
                    self.core_mut().sessions.saved(registry, &done);
                }
                if let (Some(history), Some(tab)) = (history, self.tab_mut(id)) {
                    Self::Engine::saved(&tab.view, &history, cx);
                    tab.mark = history;
                }
                self.reopen_if_saved(id, cx);
                cx.notify();
                if let Some(after) = then {
                    self.close_tab(id, cx);
                    self.proceed(after, window, cx);
                }
            }
            Err(SaveError::Conflict) => {
                // Saves no close waits on may go to the app.
                if then.is_none() && self.conflicted(id, cx) {
                    return;
                }
                self.core_mut().dialog = Some(Dialog::Conflict { tab: id, then });
                cx.notify();
            }
            failed => {
                self.report(id, failed, cx);
                if then.is_some() {
                    self.close_stopped(cx);
                }
            }
        }
    }

    /// A failed save as a toast; edits stay.
    fn report(&mut self, id: u64, saved: Result<Saved, SaveError>, cx: &mut Context<Self>) {
        let name = self.tab(id).map(|tab| tab.name.clone()).unwrap_or_default();
        let error = match saved {
            Ok(_) => return,
            Err(SaveError::Refused(state)) => state.to_string(),
            Err(SaveError::Io(error)) => error.to_string(),
            Err(SaveError::Conflict) => {
                let text = text(cx, "office.error.conflict", &[("name", &name)]);
                return self.toast(text, Severity::Danger, cx);
            }
        };
        if let Some(tab) = self.tab_mut(id) {
            tab.session.save_failed();
        }
        let text = text(
            cx,
            "office.error.save",
            &[("name", &name), ("error", &error)],
        );
        self.toast(text, Severity::Danger, cx);
        cx.notify();
    }
}

impl<H: Host> Saving for H {}

#[cfg(all(test, windows))]
mod tests;
