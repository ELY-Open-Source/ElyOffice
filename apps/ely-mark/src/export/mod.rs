//! Export, print and rich copy (MARK-11).

use std::path::PathBuf;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    overlays::Dialog,
    primitives::Severity,
    theme::ControlSize,
};
use ely_office::prelude::*;
use ely_ui::{
    commands::{Export, Print},
    i18n::text,
};
use gpui::{
    AnyElement, Context, Div, InteractiveElement, IntoElement, ParentElement as _, TaskExt as _,
};

use crate::desk::Desk;
use crate::output::{self, Format, Page, Target};

mod copy;

pub use copy::{CopyHtml, CopyPlain, CopyRich};

const FORMATS: [(Format, &str, &str); 4] = [
    (Format::Page, "page", "html"),
    (Format::Linked, "linked", "html"),
    (Format::Pdf, "pdf", "pdf"),
    (Format::Text, "text", "txt"),
];

fn key(format: Format) -> &'static str {
    FORMATS
        .iter()
        .find(|(each, ..)| *each == format)
        .expect("listed")
        .1
}

impl Desk {
    pub fn output_actions(root: Div, cx: &mut Context<Self>) -> Div {
        let root = root
            .on_action(cx.listener(|desk, _: &Export, _, cx| desk.ask_export(cx)))
            .on_action(cx.listener(|desk, _: &Print, _, cx| desk.print(cx)));
        Self::copy_actions(root, cx)
    }

    pub(crate) fn no_document(&self, cx: &mut Context<Self>) {
        let said = text(cx, "command.blocked.no_document", &[]);
        self.toast(said, Severity::Warning, cx);
    }

    fn ask_export(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active() else {
            return self.no_document(cx);
        };
        self.exporting = Some((tab, Format::Page));
        cx.notify();
    }

    pub fn export_dialog(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (_, format) = self.exporting?;
        let (picking, going, closing) = (cx.entity(), cx.entity(), cx.entity());
        let formats = FORMATS
            .iter()
            .fold(
                SegmentedControl::new("mark-export-format", key(format)),
                |control, (_, value, _)| {
                    control.segment(*value, text(cx, &format!("mark.export.{value}"), &[]), None)
                },
            )
            .size(ControlSize::Sm)
            .on_change(move |value, _, cx| {
                let Some((chosen, ..)) =
                    FORMATS.iter().find(|(_, each, _)| *each == value.as_ref())
                else {
                    return;
                };
                let chosen = *chosen;
                picking.update(cx, |desk, cx| {
                    if let Some((_, format)) = &mut desk.exporting {
                        *format = chosen;
                    }
                    cx.notify();
                })
            });
        let go = text(cx, "mark.export.go", &[]);
        let card = Dialog::new("mark-export", text(cx, "mark.export", &[]), move |_, cx| {
            closing.update(cx, |desk, cx| {
                desk.exporting = None;
                cx.notify();
            })
        })
        .child(formats)
        .action(ely_office::cancel(cx))
        .action(move |_| {
            Button::new("mark-export-go", go)
                .variant(ButtonVariant::Primary)
                .on_click(move |_, _, cx| going.update(cx, |desk, cx| desk.export_asked(cx)))
        });
        Some(card.into_any_element())
    }

    /// Asks where, then writes the chosen format.
    fn export_asked(&mut self, cx: &mut Context<Self>) {
        let Some((tab, format)) = self.exporting.take() else {
            return;
        };
        cx.notify();
        let Some(source) = self.tab(tab) else {
            return;
        };
        let Some(dir) = ely_office::folder(source) else {
            return self.export_failed(&ely_ipc::Worded::error(std::io::ErrorKind::NotFound, "error.no_documents", &[]), cx);
        };
        let stem = source
            .name
            .rsplit_once('.')
            .map_or(source.name.as_ref(), |(stem, _)| stem);
        let extension = FORMATS
            .iter()
            .find(|(each, ..)| *each == format)
            .expect("listed")
            .2;
        let chosen = cx.prompt_for_new_path(&dir, Some(&format!("{stem}.{extension}")));
        cx.spawn(async move |desk, cx| {
            let chosen = match chosen.await {
                Ok(Ok(path)) => Ok(path),
                Ok(Err(error)) => Err(std::io::Error::other(error)),
                Err(error) => Err(std::io::Error::other(error)),
            };
            desk.update(cx, |desk, cx| match chosen {
                Ok(Some(path)) => desk.export_to(tab, path, format, cx),
                Ok(None) => log::info!("mark: the export panel was cancelled"),
                Err(error) => desk.export_failed(&error, cx),
            })
        })
        .detach_and_log_err(cx);
    }

    pub fn export_to(&mut self, tab: u64, path: PathBuf, format: Format, cx: &mut Context<Self>) {
        if !self.settled(tab, cx) {
            return self.when_settled(tab, move |desk, _, cx| {
                desk.export_to(tab, path, format, cx)
            });
        }
        let (Some(source), Some(markdown)) = (self.tab(tab), self.text_of(tab)) else {
            return;
        };
        match self.holder(&path) {
            Ok(None) => {}
            Ok(Some(_)) => return self.export_failed(&ely_ipc::Worded::error(std::io::ErrorKind::AlreadyExists, "error.open_there", &[]), cx),
            Err(error) => return self.export_failed(&error, cx),
        }
        let session = source.session.path().map(PathBuf::from);
        let beside = session
            .as_deref()
            .and_then(|path| path.parent())
            .map(PathBuf::from);
        let (revision, work) = (source.session.revision(), source.work.clone());
        let opened = path.clone();
        cx.spawn(async move |desk, cx| {
            let written = cx
                .background_executor()
                .spawn(async move {
                    let target = Target {
                        path: &path,
                        session: session.as_deref(),
                        revision,
                        work: &work,
                    };
                    output::write(format, &markdown, beside.as_deref(), &target)
                })
                .await;
            desk.update(cx, |desk, cx| match written {
                Ok(page) => {
                    let name = ely_office::files::file_name(&opened);
                    desk.toast_output(text(cx, "mark.export.done", &[("name", &name)]), opened, cx);
                    desk.left_out(&page, cx);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let said = text(
                        cx,
                        "mark.export.folder_taken",
                        &[("name", &error.to_string())],
                    );
                    desk.toast(said, Severity::Danger, cx);
                }
                Err(error) => desk.export_failed(&error, cx),
            })
        })
        .detach_and_log_err(cx);
    }

    /// Says what an export could not carry.
    fn left_out(&self, page: &Page, cx: &mut Context<Self>) {
        // Names and errors quote the document: not logged.
        if !page.missing.is_empty() {
            let said = text(
                cx,
                "mark.export.missing",
                &[("list", &page.missing.join(", "))],
            );
            self.toast_unlogged(said, Severity::Warning, cx);
        }
        if !page.diagrams.is_empty() {
            let lines: Vec<String> = page.diagrams.iter().map(usize::to_string).collect();
            let said = text(cx, "mark.export.diagrams", &[("list", &lines.join(", "))]);
            self.toast(said, Severity::Warning, cx);
        }
        if !page.formulas.is_empty() {
            let each = page.formulas.iter().map(|(line, error)| {
                text(
                    cx,
                    "mark.export.formula",
                    &[("line", &line.to_string()), ("error", error)],
                )
                .to_string()
            });
            let said = text(
                cx,
                "mark.export.formulas",
                &[("list", &each.collect::<Vec<_>>().join("; "))],
            );
            self.toast_unlogged(said, Severity::Warning, cx);
        }
    }

    /// Logs `error` itself; only the toast reads its words.
    fn export_failed(&self, error: &std::io::Error, cx: &mut Context<Self>) {
        log::warn!("mark: the export failed: {}", ely_ipc::Worded::logged(error));
        let said = text(cx, "mark.export.failed", &[("error", &ely_ui::i18n::error(cx, error))]);
        self.toast_unlogged(said, Severity::Danger, cx);
    }

    /// Prints the PDF export through the system panel.
    fn print(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active() else {
            return self.no_document(cx);
        };
        if !self.settled(tab, cx) {
            return self.when_settled(tab, |desk, _, cx| desk.print(cx));
        }
        let (Some(source), Some(markdown)) = (self.tab(tab), self.text_of(tab)) else {
            return;
        };
        let beside = source
            .session
            .path()
            .and_then(|path| path.parent())
            .map(PathBuf::from);
        let (revision, work) = (source.session.revision(), source.work.clone());
        let pdf = print_copy(&work);
        log::info!("mark: prints tab {tab}");
        cx.spawn(async move |desk, cx| {
            let written = cx
                .background_executor()
                .spawn(async move {
                    let target = Target {
                        path: &pdf,
                        session: None,
                        revision,
                        work: &work,
                    };
                    output::write(Format::Pdf, &markdown, beside.as_deref(), &target)
                        .map(|page| (page, pdf))
                })
                .await;
            desk.update(cx, |desk, cx| match written {
                Ok((page, pdf)) => {
                    desk.left_out(&page, cx);
                    ely_office::print_later(pdf, cx);
                }
                Err(error) => desk.print_failed(&error.to_string(), cx),
            })
        })
        .detach_and_log_err(cx);
    }
}

/// A file of its own for each print job.
fn print_copy(work: &std::path::Path) -> PathBuf {
    static JOBS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let job = JOBS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    work.join(format!("print-{job}.pdf"))
}

#[cfg(test)]
mod tests;
