//! The suite's data folder: picked here, moved at start (§18.1).

use std::path::PathBuf;

use ely_gpui_component::{buttons::Button, feedback::Banner, primitives::Severity, settings::SettingsRow};
use ely_platform::folder::{self, State};
use gpui::{AnyElement, App, Context, IntoElement, PathPromptOptions, SharedString, WeakEntity};

use super::window::SettingsView;
use crate::i18n::text;

impl SettingsView {
    /// The data folder's row, then its failed move.
    pub(super) fn folder_rows(&self, this: WeakEntity<Self>, cx: &App) -> Vec<AnyElement> {
        let state = match &self.storage.folder {
            None => return Vec::new(),
            Some(Err(error)) => return vec![Banner::new("storage-folder-unread", Severity::Danger, error.clone()).into_any_element()],
            Some(Ok(state)) => state,
        };
        let choose = Button::new("storage-folder-choose", text(cx, "settings.storage.folder_choose", &[])).on_click(move |_, _, cx| {
            this.update(cx, |this, cx| this.pick_folder(cx)).ok();
        });
        let row = SettingsRow::new(text(cx, "settings.storage.folder", &[])).description(shown(state, cx)).control(choose);
        let mut rows = vec![row.into_any_element()];
        if let Some(failed) = failed(state, cx) {
            rows.push(Banner::new("storage-folder-failed", Severity::Danger, failed).into_any_element());
        }
        rows
    }

    fn pick_folder(&mut self, cx: &mut Context<Self>) {
        let options = PathPromptOptions { files: false, directories: true, multiple: false, prompt: None };
        let picked = cx.prompt_for_paths(options);
        cx.spawn(async move |this, cx| {
            let parent = match picked.await {
                Ok(Ok(Some(mut paths))) => paths.remove(0),
                Ok(Ok(None)) => return log::info!("settings: choosing a data folder was cancelled"),
                Ok(Err(error)) => return log::error!("settings: the folder panel failed: {error}"),
                Err(error) => return log::error!("settings: the folder panel went away: {error}"),
            };
            this.update(cx, |this, cx| this.choose_folder(parent, cx)).ok();
        })
        .detach();
    }

    fn choose_folder(&mut self, parent: PathBuf, cx: &mut Context<Self>) {
        let chosen = cx.background_executor().spawn(async move { folder::choose(&parent) });
        cx.spawn(async move |this, cx| {
            let done = chosen.await;
            this.update(cx, |this, cx| {
                match done {
                    Ok(state) => {
                        this.storage.folder = Some(Ok(state));
                        this.storage.failed = None;
                    }
                    Err(error) => this.storage.failed = Some(text(cx, "settings.storage.failed", &[("error", &error.to_string())])),
                }
                cx.notify();
            })
        })
        .detach();
    }
}

/// Where the data is, and where it goes next.
fn shown(state: &State, cx: &App) -> SharedString {
    let at = state.at.display().to_string();
    match &state.next {
        None => at.into(),
        Some(next) => text(cx, "settings.storage.folder_next", &[("at", &at), ("next", &next.display().to_string())]),
    }
}

fn failed(state: &State, cx: &App) -> Option<SharedString> {
    let why = state.failed.as_ref()?;
    Some(text(cx, "settings.storage.folder_failed", &[("error", why)]))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ely_platform::folder::State;
    use gpui::TestAppContext;

    #[gpui::test]
    fn the_row_names_the_folder_and_any_move_or_failure(cx: &mut TestAppContext) {
        cx.update(|cx| {
            ely_gpui_component::init_for_tests(cx);
            crate::settings::install_defaults(cx);
            crate::i18n::install(cx);
        });
        let at = PathBuf::from(if cfg!(windows) { r"C:\data\ElyOffice" } else { "/data/ElyOffice" });
        let next = PathBuf::from(if cfg!(windows) { r"D:\keep\ElyOffice" } else { "/keep/ElyOffice" });
        let mut state = State { at: at.clone(), next: None, failed: None };
        cx.update(|cx| {
            assert_eq!(super::shown(&state, cx).as_ref(), at.display().to_string());
            assert_eq!(super::failed(&state, cx), None);
            state.next = Some(next.clone());
            let shown = super::shown(&state, cx);
            assert!(shown.starts_with(&at.display().to_string()) && shown.contains(&next.display().to_string()), "{shown}");
            state.failed = Some("disk full".into());
            assert_eq!(super::failed(&state, cx).as_deref(), Some("The data folder did not move: disk full"));
        });
    }
}
