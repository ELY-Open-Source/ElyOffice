//! Compares the active document with an older file (DOC-13).

use std::io;
use std::path::{Path, PathBuf};

use ely_gpui_component::primitives::Severity;
use gpui::{Context, Window, actions};

use ely_office::Source;
use ely_office::files::{self, file_name};
use ely_office::prelude::*;

use crate::docs::{Docs, Side};

actions!(doc, [CompareWith]);

impl Docs {
    pub fn on_compare(&mut self, _: &CompareWith, _: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.core.active() else {
            let text = Docs::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        self.pick(false, cx, move |docs, older, cx| {
            docs.compare(id, older, cx)
        });
    }

    /// A new tab holds this version, marked against `older`.
    pub(crate) fn compare(&mut self, id: u64, older: PathBuf, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |docs, _, cx| docs.compare(id, older, cx));
        }
        let Some(tab) = self.tab(id) else {
            return;
        };
        let name = tab.name.clone();
        // The copy's tab and folder, reserved now.
        let into = self.core.take_id();
        let dir = match files::work_dir(&self.core.work(), into) {
            Ok(dir) => dir,
            Err(error) => return self.failed_open(&name, &error, cx),
        };
        let copy = dir.join("compared.odt");
        // The older file is compared as a copy.
        let old = dir.join(format!("older-{}", file_name(&older)));
        let url = ely_lok::file_url(&copy);
        let taken = (older.clone(), old.clone());
        self.read_in(
            id,
            move |worker, doc| {
                std::fs::copy(&taken.0, &taken.1)?;
                doc.save_as(worker, &url, "odt")
            },
            move |docs, saved, _, cx| match saved {
                Ok(()) => docs.compare_copy((id, into), name, copy, (older, old), cx),
                Err(error) => docs.failed_open(&name, &error, cx),
            },
            cx,
        );
    }

    /// Opens the saved copy and marks it against `older`.
    fn compare_copy(
        &mut self,
        (id, into): (u64, u64),
        name: gpui::SharedString,
        copy: PathBuf,
        (older, old): (PathBuf, PathBuf),
        cx: &mut Context<Self>,
    ) {
        log::info!("doc: comparing tab {id} with {}", file_name(&older));
        self.load_into(into, Source::Copy(copy), cx);
        let url = ely_lok::file_url(&old);
        self.core.waiting.push((
            into,
            Box::new(move |docs, _, cx| {
                // Both sides' changes are accepted first: what is left differs.
                let compared = move |worker: &mut ely_lok::Worker, doc: &ely_lok::Doc| {
                    worker.run(doc, "doc.compare", &[&url, "true"])?;
                    Ok(ely_lok::review::changes(&worker.values(doc, ".uno:AcceptTrackedChanges")?)?.len())
                };
                docs.read_in(into, compared, move |docs, found, _, cx| docs.compared(into, name, &older, found, cx), cx);
            }),
        ));
    }

    /// Shows what the compare found, or says it found nothing.
    fn compared(&mut self, into: u64, name: gpui::SharedString, older: &Path, found: io::Result<usize>, cx: &mut Context<Self>) {
        match found {
            Ok(0) => {
                log::info!("doc: tab {into} matches {}", file_name(older));
                self.close_tab(into, cx);
                let text = Docs::text(cx, "doc.compare.same", &[("name", &file_name(older))]);
                self.toast(text, Severity::Info, cx);
            }
            Ok(found) => {
                log::info!("doc: tab {into} differs in {found} places");
                if let Some(tab) = self.tab_mut(into) {
                    tab.name = Docs::text(cx, "doc.compare.name", &[("name", &name)]);
                }
                self.show_side(Side::Review, cx);
            }
            Err(error) => {
                log::error!("doc: comparing tab {into} failed: {error}");
                self.close_tab(into, cx);
                let text = Docs::text(cx, "office.error.engine", &[("error", &error.to_string())]);
                self.toast(text, Severity::Danger, cx);
            }
        }
    }
}
