//! Removal and cleaning into new files (PDF-09, PDF-10).

use std::path::PathBuf;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::Checkbox,
    overlays::Dialog as Card,
    primitives::Severity,
    theme::ActiveTheme as _,
};
use ely_office::{Closing as _, Saving as _, Tabs as _, in_window};
use ely_pdf_engine::{Cleaning, Reach, Region};
use gpui::{AnyElement, App, Context, IntoElement, ParentElement as _, Styled as _, TaskExt as _, Window, div, px};

use crate::layout::Placed;
use crate::pages::rect_on;
use crate::view::PdfView;
use crate::viewer::Viewer;

/// A removal or cleaning, bound to its asked version.
pub enum Removal {
    /// The marks as confirmed.
    Redact { tab: u64, version: u64, regions: Vec<Region> },
    /// What cleaning reaches, once scanned, and what is chosen.
    Clean { tab: u64, version: u64, reach: Reach, what: Cleaning },
}

impl Removal {
    fn bound(&self) -> (u64, u64) {
        match self {
            Removal::Redact { tab, version, .. } | Removal::Clean { tab, version, .. } => (*tab, *version),
        }
    }
}

fn inside(rect: &[f32; 4], (x, y): (f32, f32)) -> bool {
    x >= rect[0] && x <= rect[2] && y >= rect[1] && y <= rect[3]
}

impl PdfView {
    /// Marks regions; a click on one unmarks it.
    pub fn mark_regions(&mut self, page: usize, boxes: Vec<[f32; 4]>, click: Option<(f32, f32)>, cx: &mut Context<Self>) {
        match click {
            Some(at) => self.redactions.retain(|r| !(r.page == page && inside(&r.rect, at))),
            None => self.redactions.extend(boxes.into_iter().map(|rect| Region { page, rect })),
        }
        log::info!("pdf: {} regions marked to remove", self.redactions.len());
        cx.notify();
    }

    /// The regions on `place`, drawn as they will come out.
    pub fn region_marks(&self, place: &Placed, scale: f32, cx: &App) -> Vec<AnyElement> {
        let colors = &cx.theme().colors;
        let size = self.sizes[place.page];
        (self.redactions.iter().filter(|r| r.page == place.page))
            .map(|r| {
                let (x, y, w, h) = rect_on(r.rect, size, self.rotate, scale);
                let box_ = div().absolute().left(px(x)).top(px(y)).w(px(w)).h(px(h));
                box_.bg(gpui::black().opacity(0.8)).border_1().border_color(colors.danger).into_any_element()
            })
            .collect()
    }
}

/// A name beside the open one, as `stem-tail.pdf`.
pub(crate) fn beside(name: &str, tail: &str) -> String {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    format!("{stem}-{tail}.pdf")
}

impl Viewer {
    pub fn ask_redact(&mut self, cx: &mut Context<Self>) {
        let (Some(tab), Some(view)) = (self.core.active(), self.active_view()) else {
            return self.no_document(cx);
        };
        let read = view.read(cx);
        if read.redactions.is_empty() {
            let text = Self::text(cx, "pdf.redact.none", &[]);
            return self.toast(text, Severity::Info, cx);
        }
        self.removal = Some(Removal::Redact { tab, version: read.version, regions: read.redactions.clone() });
        cx.notify();
    }

    pub fn clear_redactions(&mut self, cx: &mut Context<Self>) {
        self.on_view(|view, cx| {
            view.redactions.clear();
            cx.notify();
        }, cx);
    }

    pub fn ask_clean(&mut self, cx: &mut Context<Self>) {
        let (Some(open), Some(view)) = (self.active_tab(), self.active_view()) else {
            return self.no_document(cx);
        };
        let (tab, version, scratch) = (open.id, view.read(cx).version, open.work.join("scan.pdf"));
        self.ask_in(
            tab,
            move |worker, doc| worker.get()?.clean_scan(*doc, &scratch),
            move |viewer, read, _, cx| match read {
                // An edit since makes the counts stale.
                Ok(_) if viewer.tab(tab).is_none_or(|open| open.view.read(cx).version != version) => {
                    log::info!("pdf: a cleaning scan came back after an edit")
                }
                Ok(reach) => {
                    viewer.removal = Some(Removal::Clean { tab, version, reach, what: Cleaning::default() });
                    cx.notify();
                }
                Err(error) => viewer.failed(&error, cx),
            },
            cx,
        );
    }

    /// Asks where the new file goes, then writes it.
    fn choose_output(&mut self, cx: &mut Context<Self>) {
        let Some(removal) = self.removal.take() else {
            return;
        };
        cx.notify();
        let (tab, _) = removal.bound();
        let Some(open) = self.tab(tab) else {
            return;
        };
        let Some(dir) = ely_office::folder(open) else {
            return self.panel_failed(&ely_ui::i18n::text(cx, "error.no_documents", &[]), cx);
        };
        let tail = match removal {
            Removal::Redact { .. } => Self::text(cx, "pdf.redact.tail", &[]),
            Removal::Clean { .. } => Self::text(cx, "pdf.clean.tail", &[]),
        };
        let chosen = cx.prompt_for_new_path(&dir, Some(&beside(&open.name, &tail)));
        cx.spawn(async move |this, cx| {
            let chosen = match chosen.await {
                Ok(Ok(path)) => Ok(path),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            in_window(&this, cx, |viewer, _, cx| match chosen {
                Ok(Some(target)) => viewer.write_removal(removal, target, cx),
                Ok(None) => log::info!("pdf: the output panel was cancelled"),
                Err(error) => viewer.panel_failed(&error, cx),
            })
        })
        .detach_and_log_err(cx);
    }

    /// Writes, checks, then hands the new file over.
    pub(crate) fn write_removal(&mut self, removal: Removal, target: PathBuf, cx: &mut Context<Self>) {
        let (tab, version) = removal.bound();
        let (Some(open), Some(view)) = (self.tab(tab), self.tab(tab).map(|tab| tab.view.clone())) else {
            return;
        };
        // What was confirmed is what is written, or nothing.
        let read = view.read(cx);
        let changed = match &removal {
            Removal::Redact { regions, .. } => read.redactions != *regions,
            Removal::Clean { .. } => false,
        };
        if read.version != version || !read.settled() || changed {
            let text = Self::text(cx, "pdf.object.changed", &[]);
            return self.toast(text, Severity::Warning, cx);
        }
        let (shown, opened) = (ely_office::files::file_name(&target), target.clone());
        match self.holder(&target) {
            Ok(None) => {}
            Ok(Some(_)) => return self.toast(Self::text(cx, "pdf.extract.taken", &[("name", &shown)]), Severity::Danger, cx),
            Err(error) => return self.failed(&error, cx),
        }
        let (work, path, revision) = (open.work.join("removal.pdf"), open.session.path().map(PathBuf::from), open.session.revision());
        log::info!("pdf: writes a {} copy", if matches!(removal, Removal::Redact { .. }) { "redacted" } else { "cleaned" });
        self.read_in(
            tab,
            move |worker, doc| {
                let report = match removal {
                    Removal::Redact { regions, .. } => worker.get()?.redact(*doc, &regions, &work).map(|r| r.regions)?,
                    Removal::Clean { what, .. } => worker.get()?.clean(*doc, what, &work).map(|_| 0)?,
                };
                let bytes = std::fs::read(&work)?;
                // The work copy goes; a plain delete, not a wipe.
                std::fs::remove_file(&work)?;
                ely_files::export(&target, path.as_deref(), &bytes, revision)?;
                Ok(report)
            },
            move |viewer, done, _, cx| match done {
                Ok(0) => viewer.toast_output(Self::text(cx, "pdf.clean.done", &[("name", &shown)]), opened.clone(), cx),
                Ok(n) => viewer.toast_output(ely_ui::i18n::counted(cx, "pdf.redact.done", n as u64, &[("name", &shown), ("count", &n.to_string())]), opened.clone(), cx),
                Err(error) => viewer.failed(&error, cx),
            },
            cx,
        );
    }

    /// The removal or cleaning dialog, while open.
    pub fn removal_element(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let removal = self.removal.as_ref()?;
        let this = cx.entity();
        let name = self.tab(removal.bound().0)?.name.to_string();
        let (title, body) = match removal {
            Removal::Clean { reach, what, .. } => ("pdf.clean.title", self.clean_rows(reach, *what, &this, cx)),
            Removal::Redact { regions, .. } => {
                let count = ely_ui::i18n::counted(cx, "pdf.redact.count", regions.len() as u64, &[("count", &regions.len().to_string())]);
                ("pdf.redact.title", div().child(count).into_any_element())
            }
        };
        let title = Self::text(cx, title, &[("name", &name)]);
        let label = |key: &str| Self::text(cx, key, &[]);
        let closed = this.clone();
        let dismiss = move |_: &mut Window, cx: &mut App| {
            closed.update(cx, |viewer, cx| {
                viewer.removal = None;
                cx.notify();
            })
        };
        let ready = !matches!(removal, Removal::Clean { what, .. } if *what == Cleaning::default());
        let go = label("pdf.removal.go");
        let card = Card::new("pdf-removal", title, dismiss)
            .detail(label("pdf.removal.warning"))
            .child(body)
            .action(ely_office::cancel(cx))
            .action(move |_| {
                Button::new("pdf-removal-go", go)
                    .variant(ButtonVariant::Primary)
                    .disabled(!ready)
                    .on_click(move |_, _, cx| this.update(cx, |viewer, cx| viewer.choose_output(cx)))
            });
        Some(card.into_any_element())
    }

    fn clean_rows(&self, reach: &Reach, what: Cleaning, this: &gpui::Entity<Self>, cx: &mut Context<Self>) -> AnyElement {
        let row = |at: usize, key: &str, count: usize, on: bool, set: fn(&mut Cleaning, bool)| {
            let this = this.clone();
            Checkbox::new(("pdf-clean", at), on)
                .label(Self::text(cx, key, &[("count", &count.to_string())]))
                .disabled(count == 0)
                .on_change(move |value, _, cx| {
                    this.update(cx, |viewer, cx| {
                        if let Some(Removal::Clean { what, .. }) = viewer.removal.as_mut() {
                            set(what, value);
                        }
                        cx.notify();
                    })
                })
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(row(0, "pdf.clean.meta", reach.meta, what.meta, |w, on| w.meta = on))
            .child(row(1, "pdf.clean.annots", reach.annots, what.annots, |w, on| w.annots = on))
            .child(row(2, "pdf.clean.files", reach.files, what.files, |w, on| w.files = on))
            .child(row(3, "pdf.clean.hidden", reach.hidden + reach.scripts + reach.hidden_annots, what.hidden, |w, on| w.hidden = on))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_new_file_sits_beside_the_old_by_name() {
        assert_eq!(super::beside("report.pdf", "redacted"), "report-redacted.pdf");
        assert_eq!(super::beside("notes", "cleaned"), "notes-cleaned.pdf");
    }
}
