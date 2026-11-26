//! Export a copy, and print (DOC-15).

use std::path::PathBuf;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    data_display::DescriptionList,
    forms::{Choice, NumberInput, Select, Switch},
    overlays::Dialog as Card,
    primitives::Severity,
    theme::ControlSize,
};
use ely_lok::{Doc, Pdf, Worker};
use ely_ui::commands::{Export, Print};
use gpui::{App, Context, Entity, ParentElement as _, Styled as _, TaskExt as _, Window, div};

use crate::actions::Dialog;
use ely_office::in_window;
use ely_office::prelude::*;

use crate::docs::Docs;

const PDF_FAILED: &str = "doc.output.failed";
const COPY_FAILED: &str = "doc.output.copy_failed";

/// What an export writes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Format {
    #[default]
    Pdf,
    Docx,
    Odt,
    Rtf,
    Txt,
    Epub,
    Markdown,
}

/// The dialog's order.
const FORMATS: [Format; 7] =
    [Format::Pdf, Format::Docx, Format::Odt, Format::Rtf, Format::Txt, Format::Epub, Format::Markdown];

impl Format {
    /// The extension, also the kernel's name for it.
    pub fn ext(self) -> &'static str {
        match self {
            Format::Pdf => "pdf",
            Format::Docx => "docx",
            Format::Odt => "odt",
            Format::Rtf => "rtf",
            Format::Txt => "txt",
            Format::Epub => "epub",
            Format::Markdown => "md",
        }
    }

    /// Writes a non-PDF copy of `doc` to `url`.
    pub(crate) fn write(self, worker: &mut Worker, doc: &Doc, url: &str) -> std::io::Result<()> {
        match self {
            Format::Txt => doc.export_plain(worker, url),
            _ => doc.save_as(worker, url, self.ext()),
        }
    }

    pub(crate) fn of(ext: &str) -> Option<Self> {
        FORMATS.into_iter().find(|format| format.ext().eq_ignore_ascii_case(ext))
    }
}

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
            format: Format::Pdf,
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

    /// Writes the asked copy, then saves or prints it.
    pub(crate) fn output(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Output {
            tab,
            print,
            format,
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
                format,
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
            return self.panel_failed(&ely_ui::i18n::text(cx, "error.no_documents", &[]), cx);
        };
        let named = suggested(&source.name, format, |stem| {
            Docs::text(cx, "doc.output.copy_name", &[("name", stem)]).to_string()
        });
        let chosen = cx.prompt_for_new_path(&dir, Some(&named));
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
                Ok(Some(path)) => match format {
                    Format::Pdf => docs.export_to(tab, path, pdf, cx),
                    Format::Markdown => docs.write_markdown(tab, path, cx),
                    _ => docs.write_copy(tab, path, format, cx),
                },
                Ok(None) => log::info!("doc: the export panel was cancelled"),
                Err(error) => docs.panel_failed(&error, cx),
            })
        })
        .detach_and_log_err(cx);
    }

    /// Whether `path` is free to write; else toasts `failed`.
    pub(crate) fn export_target_free(&mut self, path: &std::path::Path, failed: &str, cx: &mut Context<Self>) -> bool {
        match self.holder(path) {
            Ok(None) => true,
            Ok(Some(_)) => {
                log::info!("doc: the export target is an open document");
                self.refuse_open(path, cx);
                false
            }
            Err(error) => {
                self.output_failed(failed, &error.to_string(), cx);
                false
            }
        }
    }

    /// Tells why an open file is not overwritten.
    pub(crate) fn refuse_open(&mut self, path: &std::path::Path, cx: &mut Context<Self>) {
        let name = ely_office::files::file_name(path);
        let text = Docs::text(cx, "doc.output.open", &[("name", &name)]);
        self.toast(text, Severity::Danger, cx);
    }

    pub(crate) fn export_to(&mut self, tab: u64, path: PathBuf, pdf: Pdf, cx: &mut Context<Self>) {
        if !self.export_target_free(&path, PDF_FAILED, cx) {
            return;
        }
        let (name, opened) = (ely_office::files::file_name(&path), path.clone());
        self.write_pdf(tab, path, pdf, cx, move |docs, written, cx| match written {
            Ok(()) => {
                let text = Docs::text(cx, "doc.output.exported", &[("name", &name)]);
                docs.toast_output(text, opened.clone(), cx);
            }
            Err(error) => docs.output_failed(PDF_FAILED, &error.to_string(), cx),
        });
    }

    /// Writes `format` aside, then to `path`; the tab stays.
    fn write_copy(&mut self, tab: u64, path: PathBuf, format: Format, cx: &mut Context<Self>) {
        if !self.export_target_free(&path, COPY_FAILED, cx) {
            return;
        }
        let Some(work) = self.tab(tab).map(|tab| tab.work.clone()) else {
            return self.output_failed(COPY_FAILED, &ely_ui::i18n::text(cx, "error.closed", &[]), cx);
        };
        log::info!("doc: writing a {} copy", format.ext());
        let (shown, opened) = (ely_office::files::file_name(&path), path.clone());
        let busy = self.hold(tab);
        self.read_in(
            tab,
            move |worker, doc| {
                ely_office::remote::export_to(&work, &path, |copy| {
                    format.write(worker, doc, &ely_lok::file_url(copy))
                })
            },
            move |docs, written, _, cx| {
                let _busy = busy;
                match written {
                    Ok(()) => {
                        let text = Docs::text(cx, "doc.output.exported", &[("name", &shown)]);
                        docs.toast_output(text, opened.clone(), cx);
                    }
                    Err(error) => docs.output_failed(COPY_FAILED, &error.to_string(), cx),
                }
            },
            cx,
        );
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
            return self.output_failed(PDF_FAILED, &ely_ui::i18n::text(cx, "error.closed", &[]), cx);
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

    /// Toasts the `key` message with `error` in it.
    pub(crate) fn output_failed(&self, key: &str, error: &str, cx: &mut Context<Self>) {
        let text = Docs::text(cx, key, &[("error", error)]);
        self.toast(text, Severity::Danger, cx);
    }
}

/// The offered name; `copy` keeps off the open file's.
pub(crate) fn suggested(source: &str, format: Format, copy: impl FnOnce(&str) -> String) -> String {
    let stem = source.rsplit_once('.').map_or(source, |(stem, _)| stem);
    let named = format!("{stem}.{}", format.ext());
    match named.eq_ignore_ascii_case(source) {
        true => format!("{}.{}", copy(stem), format.ext()),
        false => named,
    }
}

/// Sets the open dialog's format.
fn choose(this: &Entity<Docs>, cx: &mut App, chosen: Format) {
    this.update(cx, |docs, cx| {
        if let Some(Dialog::Output { format, .. }) = &mut docs.dialog {
            *format = chosen;
        }
        cx.notify();
    })
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

/// The export or print dialog.
pub fn output_dialog(
    this: Entity<Docs>,
    (print, format, all, pdf, last): (bool, Format, bool, Pdf, u32),
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
    let formats = (!print).then(|| {
        let this = this.clone();
        let choices = FORMATS.map(|format| Choice::new(format.ext(), text(&format!("doc.output.format.{}", format.ext()))));
        Select::new("doc-output-format", choices)
            .label(text("doc.output.format"))
            .selected(format.ext())
            .on_change(move |value, _, cx| {
                let Some(chosen) = Format::of(value) else {
                    return log::error!("doc: no export format {value}");
                };
                choose(&this, cx, chosen)
            })
    });
    let mut fields = DescriptionList::new();
    if let Some(formats) = formats {
        fields = fields.item(text("doc.output.format"), formats);
    }
    if format == Format::Pdf {
        fields = fields
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
    }
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

#[cfg(test)]
mod tests;
