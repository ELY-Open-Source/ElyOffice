//! Mail merge from a CSV (DOC-14); never sends mail.

mod run;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    data_display::{PropertyGrid, PropertyGroup},
    forms::NumberInput,
    layout::ScrollArea,
    primitives::Severity,
    theme::ControlSize,
};
use gpui::{
    Context, IntoElement, ParentElement as _, Render, Styled as _, WeakEntity, Window, actions, div,
};

use crate::docs::{Docs, Side};
use ely_office::prelude::*;
pub use run::{Table, marker, read_csv};

actions!(doc, [MailMerge]);

/// ElyDoc's Tools menu.
pub fn menu(cx: &gpui::App) -> gpui::Menu {
    let label = |key: &str| Docs::text(cx, key, &[]);
    gpui::Menu::new(label("menu.tools")).items([
        gpui::MenuItem::action(label("office.find.open"), ely_office::FindReplace),
        gpui::MenuItem::action(label("doc.merge.open"), MailMerge),
        gpui::MenuItem::separator(),
        gpui::MenuItem::action(label("doc.nav.open"), ely_ui::commands::ToggleLeftPanel),
        gpui::MenuItem::action(label("doc.nav.back"), crate::navigate::GoBack),
    ])
}

/// The merge panel: the data, the record shown, the format.
pub struct Merge {
    docs: WeakEntity<Docs>,
    pub table: Option<Table>,
    /// The template's tab, active when the data was chosen.
    pub source: Option<u64>,
    /// Tabs opened as previews; none is a template.
    pub previews: std::collections::HashSet<u64>,
    /// The record to preview, from 1.
    record: usize,
    pdf: bool,
}

impl Merge {
    pub fn new(docs: WeakEntity<Docs>) -> Self {
        Merge {
            docs,
            table: None,
            source: None,
            previews: std::collections::HashSet::new(),
            record: 1,
            pdf: false,
        }
    }
}

impl Docs {
    pub fn on_merge(&mut self, _: &MailMerge, _: &mut Window, cx: &mut Context<Self>) {
        self.show_side(Side::Merge, cx);
        if self.merge.read(cx).table.is_none() {
            self.choose_csv(cx);
        }
    }

    pub(crate) fn choose_csv(&mut self, cx: &mut Context<Self>) {
        let Some(source) = self.core.active else {
            let text = Docs::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        if self.merge.read(cx).previews.contains(&source) {
            let text = Docs::text(cx, "doc.merge.from_preview", &[]);
            return self.toast(text, Severity::Warning, cx);
        }
        self.pick(false, cx, move |docs, path, cx| match read_csv(&path) {
            Ok(table) => {
                log::info!("doc: merge data has {} rows", table.rows.len());
                docs.merge.update(cx, |merge, cx| {
                    merge.table = Some(table);
                    merge.source = Some(source);
                    merge.record = 1;
                    cx.notify();
                });
            }
            Err(error) => {
                let name = path.display().to_string();
                let error = error.to_string();
                let text = Docs::text(
                    cx,
                    "doc.merge.unreadable",
                    &[("name", &name), ("error", &error)],
                );
                docs.toast(text, Severity::Danger, cx);
            }
        });
    }

    /// The template's tab, while it is open.
    fn source(&self, cx: &mut Context<Self>) -> Option<u64> {
        let source = self.merge.read(cx).source.filter(|source| self.tab(*source).is_some());
        if source.is_none() {
            let text = Docs::text(cx, "doc.merge.gone", &[]);
            self.toast(text, Severity::Warning, cx);
        }
        source
    }

    /// Types a field's marker at the template's cursor.
    pub(crate) fn insert_field(&mut self, header: &str, cx: &mut Context<Self>) {
        let Some(tab) = self.source(cx).and_then(|source| self.tab(source)) else {
            return;
        };
        tab.view.update(cx, |view, _| view.insert(&marker(header)));
    }
}

impl Render for Merge {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (docs, this) = (self.docs.clone(), cx.entity());
        let text = |key: &str| Docs::text(cx, key, &[]);
        let choose = {
            let docs = docs.clone();
            Button::new("doc-merge-choose", text("doc.merge.choose"))
                .size(ControlSize::Sm)
                .on_click(move |_, _, cx| {
                    docs.update(cx, |docs, cx| docs.choose_csv(cx))
                        .expect("the documents live");
                })
        };
        let Some(table) = self.table.clone() else {
            let group =
                PropertyGroup::new(text("doc.merge.title")).row(text("doc.merge.data"), choose);
            return PropertyGrid::new("doc-merge")
                .group(group)
                .into_any_element();
        };
        let fields = table
            .headers
            .iter()
            .enumerate()
            .map(|(at, header)| {
                let (docs, header) = (docs.clone(), header.clone());
                Button::new(("doc-merge-field", at), header.clone())
                    .size(ControlSize::Sm)
                    .variant(ButtonVariant::Ghost)
                    .on_click(move |_, _, cx| {
                        docs.update(cx, |docs, cx| docs.insert_field(&header, cx))
                            .expect("the documents live");
                    })
            })
            .collect::<Vec<_>>();
        let record = {
            let this = this.clone();
            NumberInput::new("doc-merge-record", self.record as f64)
                .label(text("doc.merge.record"))
                .range(1.0, table.rows.len() as f64)
                .on_commit(move |value, _, cx| {
                    this.update(cx, |merge, cx| {
                        merge.record = value.round() as usize;
                        cx.notify();
                    })
                })
        };
        let preview = {
            let (docs, table, at) = (docs.clone(), table.clone(), self.record - 1);
            Button::new("doc-merge-preview", text("doc.merge.preview"))
                .size(ControlSize::Sm)
                .on_click(move |_, _, cx| {
                    docs.update(cx, |docs, cx| {
                        if let Some(source) = docs.source(cx) {
                            docs.preview_record(source, &table, at, cx)
                        }
                    })
                    .expect("the documents live");
                })
        };
        let format = {
            let this = this.clone();
            SegmentedControl::new("doc-merge-format", if self.pdf { "pdf" } else { "docx" })
                .size(ControlSize::Sm)
                .segment("docx", "DOCX", None)
                .segment("pdf", "PDF", None)
                .on_change(move |value, _, cx| {
                    this.update(cx, |merge, cx| {
                        merge.pdf = value.as_ref() == "pdf";
                        cx.notify();
                    })
                })
        };
        let generate = {
            let (pdf, table) = (self.pdf, table.clone());
            Button::new("doc-merge-generate", text("doc.merge.generate"))
                .size(ControlSize::Sm)
                .variant(ButtonVariant::Primary)
                .on_click(move |_, _, cx| {
                    let table = table.clone();
                    docs.update(cx, |docs, cx| {
                        let Some(source) = docs.source(cx) else {
                            return;
                        };
                        docs.pick(true, cx, move |docs, folder, cx| {
                            docs.generate(source, table, pdf, folder, cx)
                        })
                    })
                    .expect("the documents live");
                })
        };
        let row = || div().flex().flex_wrap().items_center().gap_1();
        let template = self.source.and_then(|source| {
            let docs = self.docs.upgrade()?;
            Some(docs.read(cx).tab(source)?.name.clone())
        });
        let group = PropertyGroup::new(table.name.clone())
            .row(text("doc.merge.template"), div().child(template.unwrap_or_else(|| text("doc.merge.gone"))))
            .row(text("doc.merge.data"), choose)
            .row(text("doc.merge.fields"), row().children(fields))
            .row(
                text("doc.merge.record"),
                row().child(div().w_full().max_w_24().child(record)).child(preview),
            )
            .row(
                text("doc.merge.output"),
                row().child(format).child(generate),
            );
        ScrollArea::new("doc-merge-scroll").vertical()
            .size_full()
            .child(PropertyGrid::new("doc-merge").group(group))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests;
