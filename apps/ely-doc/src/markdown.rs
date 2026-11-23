//! Export as Markdown, to read or edit in ElyMark.

use ely_gpui_component::primitives::Severity;
use ely_office::in_window;
use ely_office::prelude::*;
use gpui::{Context, TaskExt as _, Window, actions};

use crate::docs::Docs;

actions!(doc, [ExportMarkdown]);

impl Docs {
    pub fn on_export_markdown(&mut self, _: &ExportMarkdown, _: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab().map(|tab| tab.id) else {
            let text = Docs::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        if !self.settled(tab, cx) {
            return self.when_settled(tab, move |docs, window, cx| {
                docs.on_export_markdown(&ExportMarkdown, window, cx)
            });
        }
        let Some(source) = self.tab(tab) else {
            return;
        };
        let Some(dir) = ely_office::folder(source) else {
            return self.panel_failed(&ely_ui::i18n::text(cx, "error.no_documents", &[]), cx);
        };
        let stem = source
            .name
            .rsplit_once('.')
            .map_or(source.name.as_ref(), |(stem, _)| stem);
        let chosen = cx.prompt_for_new_path(&dir, Some(&format!("{stem}.md")));
        // The tab stays here until this ends.
        let busy = self.hold(tab);
        cx.spawn(async move |this, cx| {
            let _busy = busy;
            let chosen = match chosen.await {
                Ok(Ok(path)) => Ok(path),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            in_window(&this, cx, |docs, _, cx| match chosen {
                Ok(Some(path)) => docs.write_markdown(tab, path, cx),
                Ok(None) => log::info!("doc: the Markdown panel was cancelled"),
                Err(error) => docs.panel_failed(&error, cx),
            })
        })
        .detach_and_log_err(cx);
    }

    /// Writes the document as Markdown; the toast opens it in ElyMark.
    pub(crate) fn write_markdown(&mut self, tab: u64, path: std::path::PathBuf, cx: &mut Context<Self>) {
        if !self.export_target_free(&path, cx) {
            return;
        }
        let Some(name) = self.tab(tab).map(|tab| tab.name.clone()) else {
            return self.output_failed(&ely_ui::i18n::text(cx, "error.closed", &[]), cx);
        };
        log::info!("doc: writing Markdown of {name}");
        let (url, shown, opened) = (ely_lok::file_url(&path), ely_office::files::file_name(&path), path);
        self.read_in(
            tab,
            move |worker, doc| doc.save_as(worker, &url, "md"),
            move |docs, written, _, cx| match written {
                Ok(()) => {
                    let text = Docs::text(cx, "doc.output.exported", &[("name", &shown)]);
                    docs.toast_output(text, opened.clone(), cx);
                }
                Err(error) => docs.output_failed(&error.to_string(), cx),
            },
            cx,
        );
    }
}
