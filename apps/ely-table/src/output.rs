//! Export as PDF, CSV or TSV, and print (TABLE-13, 14).

use std::path::PathBuf;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    overlays::Dialog as Card,
    primitives::Severity,
    theme::ControlSize,
};
use ely_lok::Pdf;
use ely_office::{in_window, prelude::*};
use ely_ui::commands::{Export, Print};
use gpui::{App, Context, Entity, ParentElement as _, TaskExt as _, Window, div};

use crate::book::Book;
use crate::dialog::{Dialog, edit};

/// What an export writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Pdf,
    Csv,
    Tsv,
}

const FORMATS: [(Format, &str, &str); 3] = [
    (Format::Pdf, "pdf", "PDF"),
    (Format::Csv, "csv", "CSV"),
    (Format::Tsv, "tsv", "TSV"),
];

impl Format {
    fn extension(self) -> &'static str {
        FORMATS.iter().find(|(each, ..)| *each == self).expect("listed").1
    }
}

/// Every sheet as Page Setup lays it out.
const WHOLE: Pdf = Pdf {
    pages: None,
    comments: false,
    changes: false,
    handouts: 0,
};

impl Book {
    pub(crate) fn on_export(&mut self, _: &Export, _: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active() else {
            return self.warn_key("command.blocked.no_document", cx);
        };
        self.open_dialog(Dialog::Export {
            tab,
            format: Format::Pdf,
        });
        cx.notify();
    }

    /// Asks where, then writes the chosen format.
    pub(crate) fn export_asked(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Export { tab, format }) = self.dialog.take() else {
            return;
        };
        cx.notify();
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
        let chosen = cx.prompt_for_new_path(&dir, Some(&format!("{stem}.{}", format.extension())));
        cx.spawn(async move |this, cx| {
            let chosen = match chosen.await {
                Ok(Ok(path)) => Ok(path),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            in_window(&this, cx, |book, _, cx| match chosen {
                Ok(Some(path)) => book.export_to(tab, path, format, cx),
                Ok(None) => log::info!("table: the export panel was cancelled"),
                Err(error) => book.panel_failed(&error, cx),
            })
        })
        .detach_and_log_err(cx);
    }

    pub(crate) fn export_to(&mut self, tab: u64, path: PathBuf, format: Format, cx: &mut Context<Self>) {
        if !self.settled(tab, cx) {
            return self.when_settled(tab, move |book, _, cx| book.export_to(tab, path, format, cx));
        }
        match self.holder(&path) {
            Ok(None) => {}
            Ok(Some(_)) => {
                let name = ely_office::files::file_name(&path);
                let text = Book::text(cx, "table.export.open", &[("name", &name)]);
                return self.toast(text, Severity::Danger, cx);
            }
            Err(error) => return self.output_failed(&error.to_string(), cx),
        }
        log::info!("table: exports tab {tab} as {format:?}");
        let (url, name) = (ely_lok::file_url(&path), ely_office::files::file_name(&path));
        let opened = path.clone();
        self.read_in(
            tab,
            move |worker, doc| match format {
                Format::Pdf => doc.export_pdf(worker, &url, &WHOLE),
                Format::Csv => doc.export_text(worker, &url, ','),
                Format::Tsv => doc.export_text(worker, &url, '\t'),
            },
            move |book, written, _, cx| match written {
                Ok(()) => {
                    let text = Book::text(cx, "table.export.done", &[("name", &name)]);
                    book.toast_output(text, opened.clone(), cx);
                }
                Err(error) => book.output_failed(&error.to_string(), cx),
            },
            cx,
        );
    }

    /// Prints a PDF of every sheet through the system panel.
    pub(crate) fn on_print(&mut self, _: &Print, _: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active() else {
            return self.warn_key("command.blocked.no_document", cx);
        };
        self.print(tab, cx);
    }

    fn print(&mut self, tab: u64, cx: &mut Context<Self>) {
        if !self.settled(tab, cx) {
            return self.when_settled(tab, move |book, _, cx| book.print(tab, cx));
        }
        let Some(source) = self.tab(tab) else {
            return;
        };
        let path = source.work.join("print.pdf");
        let url = ely_lok::file_url(&path);
        log::info!("table: prints tab {tab}");
        self.read_in(
            tab,
            move |worker, doc| doc.export_pdf(worker, &url, &WHOLE),
            move |book, written, _, cx| match written {
                Ok(()) => ely_office::print_later(path, cx),
                Err(error) => book.print_failed(&error.to_string(), cx),
            },
            cx,
        );
    }

    fn output_failed(&self, error: &str, cx: &mut Context<Self>) {
        let text = Book::text(cx, "table.export.failed", &[("error", error)]);
        self.toast(text, Severity::Danger, cx);
    }
}

/// The export dialog; text formats say what they drop.
pub fn export_dialog(
    this: Entity<Book>,
    (format, sheet): (Format, &str),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let picking = this.clone();
    let formats = FORMATS
        .iter()
        .fold(
            SegmentedControl::new("table-export-format", format.extension()),
            |control, (_, value, label)| control.segment(*value, *label, None),
        )
        .size(ControlSize::Sm)
        .on_change(move |value, _, cx| {
            let Some((chosen, ..)) = FORMATS.iter().find(|(_, each, _)| *each == value.as_ref())
            else {
                return;
            };
            let chosen = *chosen;
            edit(&picking, cx, |dialog, _, _| {
                if let Dialog::Export { format, .. } = dialog {
                    *format = chosen;
                }
            })
        });
    let loss = (format != Format::Pdf)
        .then(|| div().child(Book::text(cx, "table.export.loss", &[("sheet", sheet)])));
    let go = Book::text(cx, "table.export.go", &[]);
    Card::new("table-export", Book::text(cx, "table.export.title", &[]), dismiss)
        .child(formats)
        .children(loss)
        .action(ely_office::cancel(cx))
        .action(move |_| {
            Button::new("table-export-go", go)
                .variant(ButtonVariant::Primary)
                .on_click(move |_, _, cx| this.update(cx, |book, cx| book.export_asked(cx)))
        })
}
