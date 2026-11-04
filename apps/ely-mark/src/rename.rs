//! Renaming the open file in its folder (MARK-01).

use std::{io, path::PathBuf};

use ely_files::SaveState;
use ely_gpui_component::{forms::TextInput, overlays::PromptDialog, primitives::Severity};
use ely_office::{Host as _, files::file_name, prelude::*};
use ely_ui::{commands, i18n::text};
use gpui::{AnyElement, AppContext as _, Context, IntoElement as _, Window};

use crate::desk::Desk;

/// The text key saying why `name` is refused.
fn refused(name: &str) -> Option<&'static str> {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', '\0']) {
        return Some("mark.rename.bad");
    }
    let extension = std::path::Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase();
    (!Desk::KIND.kept.contains(&extension.as_str())).then_some("mark.rename.format")
}

impl Desk {
    /// Asks a new name for the active file.
    pub fn on_rename(&mut self, _: &commands::Rename, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let (id, name) = (tab.id, tab.name.clone());
        let field = cx.new(|cx| {
            let mut field = TextInput::new(window, cx);
            field.set_text(name, cx);
            field
        });
        self.naming = Some((id, field));
        cx.notify();
    }

    pub fn naming_element(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (id, field) = self.naming.as_ref()?;
        let (closed, sent, id) = (cx.entity(), cx.entity(), *id);
        let formats = Desk::KIND.kept.join(", .");
        let bad = text(cx, "mark.rename.bad", &[]);
        let format = text(cx, "mark.rename.format", &[("formats", &formats)]);
        let prompt = PromptDialog::new(
            "mark-rename",
            text(cx, "mark.rename", &[]),
            field,
            move |_, cx| {
                closed.update(cx, |desk, cx| {
                    desk.naming = None;
                    cx.notify();
                })
            },
        )
        .label(text(cx, "mark.rename.name", &[]))
        .submit(text(cx, "mark.rename.apply", &[]))
        .check(move |name| match refused(name) {
            None => Ok(()),
            Some("mark.rename.bad") => Err(bad.clone()),
            Some(_) => Err(format.clone()),
        })
        .on_submit(move |name, _, cx| {
            sent.update(cx, |desk, cx| {
                desk.naming = None;
                desk.rename(id, name, cx);
            })
        });
        Some(prompt.into_any_element())
    }

    /// Renames tab `id`'s file; an existing name is refused.
    fn rename(&mut self, id: u64, name: &str, cx: &mut Context<Self>) {
        let Some(tab) = self.tab(id) else {
            return;
        };
        let Some(from) = tab.session.path().map(|path| path.to_path_buf()) else {
            return self.toast(text(cx, "mark.rename.untitled", &[]), Severity::Warning, cx);
        };
        if tab.exporting || tab.session.state() == SaveState::Saving {
            return self.toast(text(cx, "mark.rename.saving", &[]), Severity::Warning, cx);
        }
        let to = from.with_file_name(name.trim());
        let (source, target) = (from.clone(), to.clone());
        cx.spawn(async move |this, cx| {
            let done = cx
                .background_executor()
                .spawn(async move { ely_files::rename_new(&source, &target).map(|()| ely_files::Sessions::key(&target)) })
                .await;
            this.update(cx, |desk, cx| desk.renamed(id, from, to, done, cx))
                .ok();
        })
        .detach();
    }

    /// The tab follows only if it holds the old path.
    fn renamed(
        &mut self,
        id: u64,
        from: PathBuf,
        to: PathBuf,
        done: io::Result<io::Result<ely_files::Key>>,
        cx: &mut Context<Self>,
    ) {
        let key = match done {
            Ok(Ok(key)) => key,
            // Renamed but unread: the watcher follows it.
            Ok(Err(error)) => return log::warn!("mark: {} was renamed, then unread: {error}", to.display()),
            Err(error) => {
                log::error!("mark: {} did not rename: {error}", from.display());
                let (name, error) = (file_name(&from), error.to_string());
                let said = text(cx, "mark.rename.failed", &[("name", &name), ("error", &error)]);
                return self.toast(said, Severity::Danger, cx);
            }
        };
        log::info!("mark: renamed {} to {}", from.display(), to.display());
        match self.tab(id).and_then(|tab| tab.session.path()) {
            Some(path) if path == from => {
                self.follow(id, to, key, cx);
            }
            // The watcher saw the rename first.
            Some(path) if path == to => log::info!("mark: the tab already follows"),
            _ => log::info!("mark: the renamed file's tab is gone"),
        }
    }

    /// Follows `id` to `to`; a refusal marks it deleted.
    pub fn follow(&mut self, id: u64, to: PathBuf, key: ely_files::Key, cx: &mut Context<Self>) -> bool {
        let Some(registry) = self.tab(id).map(|tab| tab.registry) else {
            return false;
        };
        let lost = match self.claim(id, registry, key) {
            Ok(None) => None,
            Ok(Some(other)) => {
                log::warn!("mark: tab {id}'s file went onto {other}");
                Some(crate::outside::Outside::Deleted)
            }
            Err(error) => Some(crate::outside::Outside::Unread(ely_ui::i18n::error(cx, &error).to_string())),
        };
        if let Some(lost) = lost {
            self.set_outside(id, lost, cx);
            return false;
        }
        if let Some(tab) = self.tab_mut(id) {
            tab.session.renamed(to.clone());
            tab.name = file_name(&to);
        }
        if let Some(view) = self.tab(id).map(|tab| tab.view.clone()) {
            self.observed(id, &view, cx);
        }
        // The tree and folder search see the new name.
        self.relist(cx);
        cx.notify();
        true
    }

    /// Who else holds `key`'s file; else tab `id` takes it.
    fn claim(&mut self, id: u64, registry: Option<usize>, key: ely_files::Key) -> io::Result<Option<String>> {
        for tab in self.core.tabs.iter().filter(|tab| tab.id != id) {
            if let Some(writing) = tab.session.writing()
                && ely_files::same_name(writing, key.path())?
            {
                return Ok(Some(format!("tab {}'s Save As", tab.id)));
            }
        }
        let Some(registry) = registry else { return Ok(None) };
        let taken = self.core_mut().sessions.track(registry, key)?;
        Ok(taken.err().map(|other| format!("session {other}'s file")))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn names_need_no_slashes_and_a_markdown_end() {
        for bad in ["", " ", "..", "a/b.md", "a\\b.md"] {
            assert_eq!(super::refused(bad), Some("mark.rename.bad"), "{bad:?}");
        }
        assert_eq!(super::refused("notes.txt"), Some("mark.rename.format"));
        assert_eq!(super::refused("Notes.MD"), None);
        assert_eq!(super::refused("a.markdown"), None);
    }
}
