//! Export to PDF and print (DOC-15).

use std::path::PathBuf;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    data_display::DescriptionList,
    forms::{NumberInput, Switch},
    overlays::Dialog as Card,
    primitives::Severity,
    theme::ControlSize,
};
use ely_lok::Pdf;
use ely_ui::commands::{Export, Print};
use gpui::{App, Context, Entity, ParentElement as _, Styled as _, TaskExt as _, Window, div};

use crate::actions::Dialog;
use ely_office::in_window;
use ely_office::prelude::*;

use crate::docs::Docs;

impl Docs {
    pub fn on_export(&mut self, _: &Export, _: &mut Window, cx: &mut Context<Self>) {
        self.ask_output(false, cx);
    }

    pub fn on_print(&mut self, _: &Print, _: &mut Window, cx: &mut Context<Self>) {
        self.ask_output(true, cx);
    }

    pub(crate) fn ask_output(&mut self, print: bool, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            let text = Docs::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        let last = u32::try_from(tab.view.read(cx).pages().len()).expect("a page count");
        self.dialog = Some(Dialog::Output {
            tab: tab.id,
            print,
            all: true,
            pdf: Pdf {
                pages: Some((1, last)),
                comments: false,
                changes: true,
                handouts: 0,
            },
        });
        cx.notify();
    }

    /// Writes the asked PDF, then saves or prints it.
    pub(crate) fn output(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Output {
            tab,
            print,
            all,
            pdf,
        }) = self.dialog.take()
        else {
            return;
        };
        cx.notify();
        let pdf = Pdf {
            pages: pdf.pages.filter(|_| !all),
            ..pdf
        };
        if !self.settled(tab, cx) {
            self.dialog = Some(Dialog::Output {
                tab,
                print,
                all,
                pdf,
            });
            return self.when_settled(tab, |docs, _, cx| docs.output(cx));
        }
        let Some(source) = self.tab(tab) else {
            return;
        };
        if print {
            let path = source.work.join("print.pdf");
            let printing = path.clone();
            return self.write_pdf(tab, path, pdf, cx, move |docs, written, cx| match written {
                Ok(()) => ely_office::print_later(printing, cx),
                Err(error) => docs.print_failed(&error.to_string(), cx),
            });
        }
        let Some(dir) = ely_office::folder(source) else {
            return self.panel_failed("no documents folder", cx);
        };
        let stem = source
            .name
            .rsplit_once('.')
            .map_or(source.name.as_ref(), |(stem, _)| stem);
        let chosen = cx.prompt_for_new_path(&dir, Some(&format!("{stem}.pdf")));
        cx.spawn(async move |this, cx| {
            let chosen = match chosen.await {
                Ok(Ok(path)) => Ok(path),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            in_window(&this, cx, |docs, _, cx| match chosen {
                Ok(Some(path)) => docs.export_to(tab, path, pdf, cx),
                Ok(None) => log::info!("doc: the export panel was cancelled"),
                Err(error) => docs.panel_failed(&error, cx),
            })
        })
        .detach_and_log_err(cx);
    }

    pub(crate) fn export_to(&mut self, tab: u64, path: PathBuf, pdf: Pdf, cx: &mut Context<Self>) {
        match self.holder(&path) {
            Ok(None) => {}
            Ok(Some(_)) => {
                log::info!("doc: the export target is an open document");
                let name = ely_office::files::file_name(&path);
                let text = Docs::text(cx, "doc.output.open", &[("name", &name)]);
                return self.toast(text, Severity::Danger, cx);
            }
            Err(error) => return self.output_failed(&error.to_string(), cx),
        }
        let (name, opened) = (ely_office::files::file_name(&path), path.clone());
        self.write_pdf(tab, path, pdf, cx, move |docs, written, cx| match written {
            Ok(()) => {
                let text = Docs::text(cx, "doc.output.exported", &[("name", &name)]);
                docs.toast_output(text, opened.clone(), cx);
            }
            Err(error) => docs.output_failed(&error.to_string(), cx),
        });
    }

    /// Writes the PDF in the background; `then` takes the outcome.
    fn write_pdf(
        &mut self,
        tab: u64,
        path: PathBuf,
        pdf: Pdf,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, std::io::Result<()>, &mut Context<Self>) + 'static,
    ) {
        let Some(name) = self.tab(tab).map(|tab| tab.name.clone()) else {
            return self.output_failed("the document closed", cx);
        };
        log::info!("doc: writing a PDF of {name} ({pdf:?})");
        let url = ely_lok::file_url(&path);
        self.read_in(
            tab,
            move |worker, doc| doc.export_pdf(worker, &url, &pdf),
            move |docs, written, _, cx| then(docs, written, cx),
            cx,
        );
    }

    fn output_failed(&self, error: &str, cx: &mut Context<Self>) {
        let text = Docs::text(cx, "doc.output.failed", &[("error", error)]);
        self.toast(text, Severity::Danger, cx);
    }
}

/// Edits the open output dialog.
fn change(this: &Entity<Docs>, cx: &mut App, edit: impl FnOnce(&mut bool, &mut Pdf)) {
    this.update(cx, |docs, cx| {
        if let Some(Dialog::Output { all, pdf, .. }) = &mut docs.dialog {
            edit(all, pdf);
        }
        cx.notify();
    })
}

/// The dialog before a PDF goes out: pages, comments, changes.
pub fn output_dialog(
    this: Entity<Docs>,
    (print, all, pdf, last): (bool, bool, Pdf, u32),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let text = |key: &str| Docs::text(cx, key, &[]);
    let (from, to) = pdf.pages.unwrap_or((1, last));
    let pages = {
        let this = this.clone();
        SegmentedControl::new("doc-output-pages", if all { "all" } else { "range" })
            .size(ControlSize::Sm)
            .segment("all", text("doc.output.all"), None)
            .segment("range", text("doc.output.range"), None)
            .on_change(move |value, _, cx| {
                change(&this, cx, |all, _| *all = value.as_ref() == "all")
            })
    };
    let bound = |id: &'static str, value: u32, first: bool| {
        let this = this.clone();
        NumberInput::new(id, f64::from(value))
            .label(text(if first { "doc.output.from" } else { "doc.output.to" }))
            .range(1.0, f64::from(last))
            .on_commit(move |value, _, cx| {
                let value = value.round() as u32;
                change(&this, cx, |_, pdf| {
                    let (from, to) = pdf.pages.unwrap_or((1, value));
                    pdf.pages = Some(if first {
                        (value, to.max(value))
                    } else {
                        (from.min(value), value)
                    });
                })
            })
    };
    let range = (!all).then(|| {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().w_full().max_w_24().child(bound("doc-output-from", from, true)))
            .child("–")
            .child(div().w_full().max_w_24().child(bound("doc-output-to", to, false)))
    });
    let toggle = |id: &'static str, key: &str, on: bool, set: fn(&mut Pdf, bool)| {
        let this = this.clone();
        Switch::new(id, on)
            .label(text(key))
            .on_change(move |on, _, cx| change(&this, cx, |_, pdf| set(pdf, on)))
    };
    let fields = DescriptionList::new()
        .item(
            text("doc.output.pages"),
            div().flex().flex_col().gap_2().child(pages).children(range),
        )
        .item(
            text("doc.output.marks"),
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(toggle(
                    "doc-output-comments",
                    "doc.output.comments",
                    pdf.comments,
                    |pdf, on| pdf.comments = on,
                ))
                .child(toggle(
                    "doc-output-changes",
                    "doc.output.changes",
                    pdf.changes,
                    |pdf, on| pdf.changes = on,
                )),
        );
    let (title, go) = match print {
        true => ("doc.output.print_title", "doc.output.print"),
        false => ("doc.output.export_title", "doc.output.export"),
    };
    let label = text(go);
    Card::new("doc-output", text(title), dismiss)
        .child(fields)
        .action(ely_office::cancel(cx))
        .action(move |_| {
            Button::new("doc-output-go", label)
                .variant(ButtonVariant::Primary)
                .on_click(move |_, _, cx| this.update(cx, |docs, cx| docs.output(cx)))
        })
}
