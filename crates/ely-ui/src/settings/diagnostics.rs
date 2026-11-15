//! Diagnostics export: what goes in is shown first (§18.2).

use ely_gpui_component::{
    buttons::Button,
    forms::Switch,
    primitives::Severity,
    settings::{SettingsRow, SettingsSection},
    theme::{ActiveTheme as _, TextSize},
};
use ely_platform::diagnostics::{self, Part, Redact};
use gpui::{
    AnyElement, App, Context, InteractiveElement as _, IntoElement, ParentElement as _, Role, SharedString,
    StatefulInteractiveElement as _, Styled as _,
    WeakEntity, div,
};

use super::window::SettingsView;
use crate::i18n::text;

/// What the bundle holds; how the last export went.
#[derive(Default)]
pub(crate) struct Diagnostics {
    parts: Option<Result<Vec<Part>, SharedString>>,
    /// Paths and names go out only when asked.
    unredacted: bool,
    status: Option<(Severity, SharedString)>,
}

impl SettingsView {
    /// Lists the parts off the window thread.
    pub(super) fn read_diagnostics(&mut self, cx: &mut Context<Self>) {
        let app = self.app;
        let read = cx.background_executor().spawn(async move { diagnostics::gather(app) });
        cx.spawn(async move |this, cx| {
            let parts = read.await.map_err(|error| SharedString::from(error.to_string()));
            this.update(cx, |this, cx| {
                this.diagnostics.parts = Some(parts);
                cx.notify();
            })
        })
        .detach();
    }

    pub(super) fn diagnostics_page(&self, this: WeakEntity<Self>, cx: &App) -> AnyElement {
        let found = match &self.diagnostics.parts {
            None => Vec::new(),
            Some(Ok(parts)) if parts.is_empty() => vec![text(cx, "settings.diagnostics.none", &[])],
            Some(Ok(parts)) => parts.iter().map(|part| line(part, cx)).collect(),
            Some(Err(error)) => vec![text(cx, "settings.diagnostics.failed", &[("error", error)])],
        };
        let some = matches!(&self.diagnostics.parts, Some(Ok(parts)) if !parts.is_empty());
        let (toggle, export) = (this.clone(), this);
        let mut section = SettingsSection::new(text(cx, "settings.diagnostics", &[]))
            .row(
                SettingsRow::new(text(cx, "settings.diagnostics.included", &[]))
                    .control(
                        div()
                            .id("settings-parts")
                            .role(Role::List)
                            .aria_label(text(cx, "settings.diagnostics.included", &[]))
                            .flex()
                            .flex_col()
                            .text_size(cx.theme().text_size(TextSize::Sm))
                            .children(found.into_iter().enumerate().map(|(at, said)| {
                                div().id(("settings-part", at)).role(Role::ListItem).aria_label(said.clone()).child(said)
                            })),
                    ),
            )
            .row(
                SettingsRow::new(text(cx, "settings.diagnostics.redact", &[])).control(
                    Switch::new("settings-redact", !self.diagnostics.unredacted)
                        .name(text(cx, "settings.diagnostics.redact", &[]))
                        .on_change(move |on, _, cx| {
                            toggle
                                .update(cx, |this, cx| {
                                    this.diagnostics.unredacted = !on;
                                    cx.notify();
                                })
                                .ok();
                        }),
                ),
            )
            .row(
                SettingsRow::new(text(cx, "settings.diagnostics.export", &[])).control(
                    Button::new("settings-export", text(cx, "settings.diagnostics.export", &[]))
                        .disabled(!some)
                        .on_click(move |_, _, cx| {
                            export.update(cx, |this, cx| this.export_diagnostics(cx)).ok();
                        }),
                ),
            );
        if let Some((severity, said)) = self.diagnostics.status.clone() {
            section = section.row(super::said::line("settings-export-status", severity, said, cx));
        }
        section.into_any_element()
    }

    /// Asks where, then writes the bundle in the background.
    fn export_diagnostics(&mut self, cx: &mut Context<Self>) {
        let Some(Ok(parts)) = self.diagnostics.parts.clone() else {
            return;
        };
        let (app, redact) = (self.app, !self.diagnostics.unredacted);
        let folder = directories::UserDirs::new()
            .and_then(|dirs| dirs.document_dir().map(std::path::Path::to_path_buf))
            .unwrap_or_else(std::env::temp_dir);
        let name = format!("{}-diagnostics.txt", app.name());
        let chosen = cx.prompt_for_new_path(&folder, Some(&name));
        cx.spawn(async move |this, cx| {
            let path = match chosen.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) => return log::info!("settings: diagnostics export cancelled"),
                Ok(Err(error)) => return log::error!("settings: the save panel failed: {error}"),
                Err(error) => return log::error!("settings: the save panel failed: {error}"),
            };
            let target = path.clone();
            let written = cx
                .background_executor()
                .spawn(async move {
                    // Names that cannot be read stop the export.
                    let redact = redact.then(Redact::of_account).transpose()?;
                    let bundle = diagnostics::bundle(app, &parts, redact.as_ref())?;
                    std::fs::write(&target, bundle)
                })
                .await;
            this.update(cx, |this, cx| {
                let shown = path.display().to_string();
                this.diagnostics.status = Some(match written {
                    Ok(()) => {
                        log::info!("settings: diagnostics exported, redacted {redact}");
                        (Severity::Success, text(cx, "settings.diagnostics.saved", &[("path", &shown)]))
                    }
                    Err(error) => (Severity::Danger, text(cx, "settings.diagnostics.failed", &[("error", &error.to_string())])),
                });
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

/// `ely.log · 120 KB` or a crash report's file name.
fn line(part: &Part, cx: &App) -> SharedString {
    let file = part.path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let name = match file.as_str() {
        "ely.log" => text(cx, "settings.diagnostics.log", &[]).to_string(),
        _ => text(cx, "settings.diagnostics.crash", &[("name", &file)]).to_string(),
    };
    let size = format!("{} KB", part.bytes.div_ceil(1024));
    text(cx, "settings.diagnostics.size", &[("name", &name), ("size", &size)])
}
