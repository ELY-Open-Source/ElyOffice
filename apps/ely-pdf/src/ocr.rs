//! OCR: asked, then read page by page (PDF-08).

use std::path::PathBuf;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    data_display::DescriptionList,
    forms::{CheckboxGroup, Choice, NumberInput, RadioGroup},
    motion::ProgressBar,
    overlays::Dialog as Card,
    primitives::Severity,
};
use ely_office::{Closing as _, Tabs as _};
use ely_pdf_engine::OcrLine;
use gpui::{AnyElement, App, Context, IntoElement, ParentElement as _, SharedString, Window, div};

use crate::viewer::Viewer;

/// Pixels an inch Tesseract reads at.
const DPI: f32 = 300.0;
/// The languages, by Tesseract name and label key.
const LANGS: [(&str, &str); 3] = [("eng", "pdf.ocr.eng"), ("chi_sim", "pdf.ocr.sim"), ("chi_tra", "pdf.ocr.tra")];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    All,
    Current,
    Range,
}

/// What was asked, and the reading under way.
pub struct Ocr {
    pub tab: u64,
    pub scope: Scope,
    /// Pages from one.
    pub from: usize,
    pub to: usize,
    pub langs: Vec<SharedString>,
    /// Read pages with text again, dropping Ely's layer.
    pub replace: bool,
    pub run: Option<Run>,
}

pub struct Run {
    /// Counts runs; a page back from another is dropped.
    id: u64,
    version: u64,
    pages: Vec<usize>,
    at: usize,
    read: Vec<(usize, Vec<OcrLine>)>,
    skipped: usize,
    data: PathBuf,
}

/// The pages `scope` names, from zero.
pub fn pages(scope: Scope, current: usize, (from, to): (usize, usize), count: usize) -> Vec<usize> {
    match scope {
        Scope::All => (0..count).collect(),
        Scope::Current => vec![current],
        Scope::Range => (from.max(1) - 1..to.min(count)).collect(),
    }
}

impl Viewer {
    pub fn ask_ocr(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let (Some(tab), Some(view)) = (self.core.active(), self.active_view()) else {
            return self.no_document(cx);
        };
        let count = view.read(cx).sizes.len();
        let langs = vec!["eng".into(), "chi_sim".into()];
        self.ocr = Some(Ocr { tab, scope: Scope::All, from: 1, to: count, langs, replace: false, run: None });
        cx.notify();
    }

    pub(crate) fn start_ocr(&mut self, cx: &mut Context<Self>) {
        let Some(ocr) = self.ocr.as_mut() else {
            return;
        };
        let Some(view) = self.core.tabs.iter().find(|tab| tab.id == ocr.tab).map(|tab| tab.view.clone()) else {
            return;
        };
        let data = match ely_pdf_engine::tessdata() {
            Ok(data) => data,
            Err(error) => return self.failed(&error, cx),
        };
        let read = view.read(cx);
        let pages = pages(ocr.scope, read.current, (ocr.from, ocr.to), read.sizes.len());
        if pages.is_empty() || ocr.langs.is_empty() {
            return;
        }
        log::info!("pdf: OCR of {} pages in {} from {}", pages.len(), ocr.langs.join("+"), data.display());
        self.ocr_runs += 1;
        ocr.run = Some(Run { id: self.ocr_runs, version: read.version, pages, at: 0, read: Vec::new(), skipped: 0, data });
        self.ocr_step(cx);
    }

    /// Reads the next page, or lays what was read.
    fn ocr_step(&mut self, cx: &mut Context<Self>) {
        let Some(Ocr { tab, langs, replace, run: Some(run), .. }) = self.ocr.as_ref() else {
            return;
        };
        let Some(view) = self.tab(*tab).map(|tab| tab.view.clone()) else {
            self.ocr = None;
            return cx.notify();
        };
        if view.read(cx).version != run.version {
            self.ocr = None;
            let text = Self::text(cx, "pdf.object.changed", &[]);
            return self.toast(text, Severity::Warning, cx);
        }
        let Some(page) = run.pages.get(run.at).copied() else {
            return self.lay_text(cx);
        };
        let (langs, replace, data, tab, id) = (langs.join("+"), *replace, run.data.clone(), *tab, run.id);
        let viewer = cx.weak_entity();
        view.update(cx, |view, cx| {
            let doc = view.doc;
            view.off(
                move |worker| match !replace && !worker.text(doc, page)?.is_empty() {
                    true => Ok(None),
                    false => worker.ocr_page(doc, page, &langs, &data, DPI).map(Some),
                },
                move |_, read, cx| {
                    cx.defer(move |cx| {
                        if let Some(viewer) = viewer.upgrade() {
                            viewer.update(cx, |viewer, cx| viewer.ocr_read((tab, id, page), read, cx));
                        }
                    })
                },
                cx,
            );
        });
        cx.notify();
    }

    /// One page back; a cancelled or other run drops it.
    pub(crate) fn ocr_read(&mut self, (tab, id, page): (u64, u64, usize), read: std::io::Result<Option<Vec<OcrLine>>>, cx: &mut Context<Self>) {
        let Some(run) = self.ocr.as_mut().filter(|ocr| ocr.tab == tab).and_then(|ocr| ocr.run.as_mut()) else {
            return log::info!("pdf: an OCR page came back after its run");
        };
        if run.id != id || run.pages.get(run.at) != Some(&page) {
            return log::info!("pdf: an OCR page came back to another run");
        }
        match read {
            Ok(Some(lines)) => run.read.push((page, lines)),
            Ok(None) => run.skipped += 1,
            Err(error) => {
                self.ocr = None;
                return self.failed(&error, cx);
            }
        }
        run.at += 1;
        self.ocr_step(cx);
    }

    /// Lays every page read in hidden text, one step.
    fn lay_text(&mut self, cx: &mut Context<Self>) {
        let Some(Ocr { tab, replace, run: Some(run), .. }) = self.ocr.take() else {
            return;
        };
        cx.notify();
        let pages: Vec<(usize, Vec<OcrLine>)> = run.read.into_iter().filter(|(_, lines)| replace || !lines.is_empty()).collect();
        let (read, skipped) = (pages.len().to_string(), run.skipped.to_string());
        let summary = ely_ui::i18n::counted(cx, "pdf.ocr.done", pages.len() as u64, &[("read", &read), ("skipped", &skipped)]);
        let Some(view) = self.tab(tab).map(|tab| tab.view.clone()).filter(|_| !pages.is_empty()) else {
            return self.toast(summary, Severity::Info, cx);
        };
        let laid = self.on_same(&view, run.version, cx, |view, cx| {
            view.edit("pdf.doing.add_ocr", move |worker, doc| worker.text_layer(doc, &pages, replace), cx)
        });
        if laid {
            self.toast(summary, Severity::Success, cx);
        }
    }

    /// The OCR dialog: the ask, then the progress.
    pub fn ocr_element(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let ocr = self.ocr.as_ref()?;
        let this = cx.entity();
        let label = |key: &str| Self::text(cx, key, &[]);
        let closed = this.clone();
        let dismiss = move |_: &mut Window, cx: &mut App| {
            closed.update(cx, |viewer, cx| {
                viewer.ocr = None;
                cx.notify();
            })
        };
        let card = Card::new("pdf-ocr", label("pdf.ocr.title"), dismiss);
        if let Some(run) = &ocr.run {
            let (n, count) = ((run.at + 1).min(run.pages.len()).to_string(), run.pages.len().to_string());
            let progress = ProgressBar::new("pdf-ocr-progress", run.at as f32 / run.pages.len() as f32);
            let card = card.detail(Self::text(cx, "pdf.ocr.reading", &[("n", &n), ("count", &count)])).child(progress);
            return Some(card.action(ely_office::cancel(cx)).into_any_element());
        }
        let go = label("pdf.ocr.go");
        let ready = !ocr.langs.is_empty();
        Some(
            card.child(self.ocr_rows(ocr, &this, cx))
                .action(ely_office::cancel(cx))
                .action(move |_| {
                    Button::new("pdf-ocr-go", go)
                        .variant(ButtonVariant::Primary)
                        .disabled(!ready)
                        .on_click(move |_, _, cx| this.update(cx, |viewer, cx| viewer.start_ocr(cx)))
                })
                .into_any_element(),
        )
    }

    fn ocr_rows(&self, ocr: &Ocr, this: &gpui::Entity<Self>, cx: &mut Context<Self>) -> AnyElement {
        let label = |key: &str| Self::text(cx, key, &[]);
        let change = |this: &gpui::Entity<Self>, change: fn(&mut Ocr, &str)| {
            let this = this.clone();
            move |value: &SharedString, _: &mut Window, cx: &mut App| {
                this.update(cx, |viewer, cx| {
                    if let Some(ocr) = viewer.ocr.as_mut() {
                        change(ocr, value);
                    }
                    cx.notify();
                })
            }
        };
        let scope = match ocr.scope {
            Scope::All => "all",
            Scope::Current => "current",
            Scope::Range => "range",
        };
        let scopes = SegmentedControl::new("pdf-ocr-scope", scope)
            .segment("all", label("pdf.ocr.all"), None)
            .segment("current", label("pdf.ocr.current"), None)
            .segment("range", label("pdf.ocr.range"), None)
            .on_change(change(this, |ocr, value| {
                ocr.scope = match value {
                    "current" => Scope::Current,
                    "range" => Scope::Range,
                    _ => Scope::All,
                }
            }));
        let langs = CheckboxGroup::new("pdf-ocr-langs", LANGS.map(|(value, key)| Choice::new(value, label(key))))
            .selected(ocr.langs.clone())
            .on_change({
                let this = this.clone();
                move |values: &[SharedString], _: &mut Window, cx: &mut App| {
                    this.update(cx, |viewer, cx| {
                        if let Some(ocr) = viewer.ocr.as_mut() {
                            ocr.langs = values.to_vec();
                        }
                        cx.notify();
                    })
                }
            });
        let policy = RadioGroup::new("pdf-ocr-policy", [Choice::new("skip", label("pdf.ocr.skip")), Choice::new("replace", label("pdf.ocr.replace"))])
            .selected(if ocr.replace { "replace" } else { "skip" })
            .on_change(change(this, |ocr, value| ocr.replace = value == "replace"));
        let mut rows = DescriptionList::new().item(label("pdf.ocr.pages"), scopes);
        if ocr.scope == Scope::Range {
            let bound = |id: &'static str, value: usize, set: fn(&mut Ocr, usize)| {
                let this = this.clone();
                NumberInput::new(id, value as f64).range(1.0, 100_000.0).on_change(move |value, _, cx| {
                    this.update(cx, |viewer, cx| {
                        if let Some(ocr) = viewer.ocr.as_mut() {
                            set(ocr, value.round() as usize);
                        }
                        cx.notify();
                    })
                })
            };
            let from = bound("pdf-ocr-from", ocr.from, |ocr, value| ocr.from = value).label(label("pdf.ocr.from"));
            let to = bound("pdf-ocr-to", ocr.to, |ocr, value| ocr.to = value).label(label("pdf.ocr.to"));
            rows = rows.item(label("pdf.ocr.from"), from).item(label("pdf.ocr.to"), to);
        }
        let rows = rows.item(label("pdf.ocr.langs"), langs).item(label("pdf.ocr.policy"), policy);
        div().child(rows).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scope_names_its_pages() {
        assert_eq!(pages(Scope::All, 1, (1, 1), 3), [0, 1, 2]);
        assert_eq!(pages(Scope::Current, 1, (1, 1), 3), [1]);
        assert_eq!(pages(Scope::Range, 0, (2, 9), 4), [1, 2, 3], "the end stops at the last page");
        assert!(pages(Scope::Range, 0, (3, 2), 4).is_empty());
    }
}
