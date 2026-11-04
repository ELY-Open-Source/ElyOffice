//! Compares the active document with an older file (DOC-13).

use std::path::PathBuf;

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
        let into = self.core.next;
        self.core.next += 1;
        let dir = match files::work_dir(&self.core.work, into) {
            Ok(dir) => dir,
            Err(error) => return self.failed_open(&name, &error, cx),
        };
        let copy = dir.join("compared.odt");
        let url = ely_lok::file_url(&copy);
        self.read_in(
            id,
            move |worker, doc| doc.save_as(worker, &url, "odt"),
            move |docs, saved, _, cx| match saved {
                Ok(()) => docs.compare_copy((id, into), name, copy, older, cx),
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
        older: PathBuf,
        cx: &mut Context<Self>,
    ) {
        log::info!("doc: comparing tab {id} with {}", file_name(&older));
        self.load_into(into, Source::Copy(copy), cx);
        let url = ely_lok::file_url(&older);
        self.core.waiting.push((
            into,
            Box::new(move |docs, _, cx| {
                let Some(tab) = docs.tab_mut(into) else {
                    return;
                };
                tab.name = Docs::text(cx, "doc.compare.name", &[("name", &name)]);
                let sent = tab
                    .view
                    .update(cx, |view, _| view.run("doc.compare", &[&url, "true"]));
                if let Err(error) = sent {
                    let text =
                        Docs::text(cx, "office.error.engine", &[("error", &error.to_string())]);
                    return docs.toast(text, Severity::Danger, cx);
                }
                docs.show_side(Side::Review, cx);
            }),
        ));
    }
}
