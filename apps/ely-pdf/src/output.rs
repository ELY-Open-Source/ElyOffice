//! Export, print, and the repaired copy's report (PDF-13, PDF-14).

use std::{io, path::PathBuf};

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    data_display::DescriptionList,
    feedback::Banner,
    forms::{Checkbox, NumberInput},
    overlays::Dialog as Card,
    primitives::Severity,
    theme::ControlSize,
};
use ely_office::{Chrome as _, Closing as _, Saving as _, Stopping as _, Tabs as _, in_window};
use ely_pdf_engine::Optimizing;
use ely_ui::commands::{Export, Print};
use gpui::{AnyElement, App, Context, IntoElement, ParentElement as _, Styled as _, TaskExt as _, Window, div};

use crate::view::{PdfView, Reported};
use crate::viewer::Viewer;

impl PdfView {
    /// A draw met damage; read what MuPDF has said.
    pub fn found_damage(&mut self, cx: &mut Context<Self>) {
        let doc = self.doc;
        self.off(
            move |worker| worker.repair_report(doc),
            |view, read, cx| match read {
                Ok(report) => {
                    (view.repair, view.damage_seen) = (report, true);
                    cx.emit(Reported);
                }
                Err(error) => view.fail("reading the damage report", &error, cx),
            },
            cx,
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Images,
    Text,
    Optimized,
}

/// What the export dialog asks for.
pub struct Exporting {
    tab: u64,
    pub kind: Kind,
    dpi: u32,
    /// Every page, or the picked ones.
    all: bool,
    pub how: Optimizing,
}

/// `stem-N.png` for each page, numbers padded alike.
pub fn image_names(stem: &str, pages: &[usize], count: usize) -> Vec<String> {
    let width = count.to_string().len();
    pages.iter().map(|page| format!("{stem}-{:0width$}.png", page + 1)).collect()
}

fn stem(name: &str) -> String {
    name.rsplit_once('.').map_or(name, |(stem, _)| stem).to_string()
}

impl Viewer {
    pub fn on_export(&mut self, _: &Export, _: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active else {
            return self.no_document(cx);
        };
        let how = Optimizing { compress: true, from: 300, to: 150 };
        self.exporting = Some(Exporting { tab, kind: Kind::Images, dpi: 150, all: true, how });
        cx.notify();
    }

    pub fn on_print(&mut self, _: &Print, _: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active else {
            return self.no_document(cx);
        };
        self.print(tab, cx);
    }

    fn print(&mut self, id: u64, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |viewer, _, cx| viewer.print(id, cx));
        }
        let Some(tab) = self.tab(id) else {
            return;
        };
        let printing = tab.work.join("print.pdf");
        let out = printing.clone();
        log::info!("pdf: prints");
        self.read_in(
            id,
            move |worker, doc| worker.get()?.print_copy(*doc, &out),
            move |viewer, done, _, cx| match done {
                Ok(()) => ely_office::print_later(printing, cx),
                Err(error) => viewer.failed(&error, cx),
            },
            cx,
        );
    }

    /// Asks where, then writes what the dialog asks for.
    fn choose_export(&mut self, cx: &mut Context<Self>) {
        let Some(asked) = self.exporting.take() else {
            return;
        };
        cx.notify();
        let (Some(tab), Some(view)) = (self.tab(asked.tab), self.tab(asked.tab).map(|tab| tab.view.clone())) else {
            return;
        };
        let Some(dir) = ely_office::folder(tab) else {
            return self.panel_failed("no documents folder", cx);
        };
        let (id, stem) = (asked.tab, stem(&tab.name));
        if asked.kind == Kind::Images {
            let read = view.read(cx);
            let pages = if asked.all { (0..read.sizes.len()).collect() } else { read.chosen_pages() };
            let names = image_names(&stem, &pages, read.sizes.len());
            let (dpi, version) = (asked.dpi as f32, read.version);
            return self.pick(true, cx, move |viewer, folder, cx| {
                let targets = names.iter().map(|name| folder.join(name)).collect();
                viewer.write_images(id, (pages, version), dpi, targets, cx)
            });
        }
        let name = match asked.kind {
            Kind::Text => format!("{stem}.txt"),
            _ => format!("{stem}-{}.pdf", Self::text(cx, "pdf.export.tail", &[])),
        };
        let chosen = cx.prompt_for_new_path(&dir, Some(&name));
        cx.spawn(async move |this, cx| {
            let chosen = match chosen.await {
                Ok(Ok(path)) => Ok(path),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            in_window(&this, cx, |viewer, _, cx| match chosen {
                Ok(Some(target)) => viewer.write_export(id, asked.kind, asked.how, target, cx),
                Ok(None) => log::info!("pdf: the export panel was cancelled"),
                Err(error) => viewer.panel_failed(&error, cx),
            })
        })
        .detach_and_log_err(cx);
    }

    /// Page numbers hold for `version` only; an edit since refuses.
    pub(crate) fn write_images(&mut self, id: u64, (pages, version): (Vec<usize>, u64), dpi: f32, targets: Vec<PathBuf>, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |viewer, _, cx| viewer.write_images(id, (pages, version), dpi, targets, cx));
        }
        let Some(tab) = self.tab(id) else {
            return;
        };
        if tab.view.read(cx).version != version {
            return self.toast(Self::text(cx, "pdf.object.changed", &[]), Severity::Warning, cx);
        }
        let work = tab.work.join("page.png");
        let folder = targets.first().and_then(|target| target.parent()).map(PathBuf::from);
        log::info!("pdf: exports {} page images", pages.len());
        self.read_in(
            id,
            move |worker, doc| {
                let claimed = ely_files::claim_all(&targets)?;
                let written = pages.iter().zip(claimed).try_for_each(|(page, file)| {
                    worker.get()?.export_image(*doc, *page, dpi, &work)?;
                    crate::ask::copy_into(&work, file)
                });
                // A failure leaves none of this run's files.
                if let Err(error) = &written
                    && let Err(left) = targets.iter().try_for_each(std::fs::remove_file)
                {
                    return Err(io::Error::other(format!("{error}; then {left}")));
                }
                written.map(|()| targets.len())
            },
            move |viewer, done, _, cx| match done {
                Ok(count) => {
                    let text = ely_ui::i18n::counted(cx, "pdf.export.images_done", count as u64, &[("count", &count.to_string())]);
                    match folder.clone() {
                        Some(folder) => viewer.toast_output(text, folder, cx),
                        None => viewer.toast(text, Severity::Success, cx),
                    }
                }
                Err(error) => viewer.failed(&error, cx),
            },
            cx,
        );
    }

    pub(crate) fn write_export(&mut self, id: u64, kind: Kind, how: Optimizing, target: PathBuf, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |viewer, _, cx| viewer.write_export(id, kind, how, target, cx));
        }
        let Some(tab) = self.tab(id) else {
            return;
        };
        let shown = ely_office::files::file_name(&target);
        match self.holder(&target) {
            Ok(None) => {}
            Ok(Some(_)) => return self.toast(Self::text(cx, "pdf.extract.taken", &[("name", &shown)]), Severity::Danger, cx),
            Err(error) => return self.failed(&error, cx),
        }
        let (work, path, revision) = (tab.work.join("export.out"), tab.session.path().map(PathBuf::from), tab.session.revision());
        // Unsaved edits: the file on disk is not "before".
        let compared = path.clone().filter(|_| !tab.view.read(cx).modified());
        let opened = target.clone();
        log::info!("pdf: exports {}", if kind == Kind::Text { "text" } else { "an optimized copy" });
        self.read_in(
            id,
            move |worker, doc| {
                match kind {
                    Kind::Text => worker.get()?.export_text(*doc, &work)?,
                    _ => worker.get()?.optimize(*doc, &work, how)?,
                }
                let bytes = std::fs::read(&work)?;
                std::fs::remove_file(&work)?;
                let before = compared.as_deref().map(std::fs::metadata).transpose()?.map(|meta| meta.len());
                ely_files::export(&target, path.as_deref(), &bytes, revision)?;
                Ok((before, bytes.len() as u64))
            },
            move |viewer, done, _, cx| match done {
                Ok((Some(before), after)) if kind == Kind::Optimized => {
                    let sizes = [("name", shown.as_ref()), ("before", &crate::attach::size(before)), ("after", &crate::attach::size(after))];
                    viewer.toast_output(Self::text(cx, "pdf.export.optimized_done", &sizes), opened.clone(), cx)
                }
                Ok(_) => viewer.toast_output(Self::text(cx, "pdf.export.done", &[("name", &shown)]), opened.clone(), cx),
                Err(error) => viewer.failed(&error, cx),
            },
            cx,
        );
    }

    /// The shell's warning, with its details a press away.
    pub fn file_banner(&self, cx: &mut Context<Self>) -> Option<Banner> {
        if let Some(stopped) = self.stopped_banner(cx) {
            return Some(stopped);
        }
        let banner = self.risk_banner(cx)?;
        let (key, action): (&str, Box<dyn gpui::Action>) = match self.active_tab()?.risk? {
            ely_office::Risk::Signed => ("pdf.sign.show", Box::new(crate::document::ShowSignatures)),
            ely_office::Risk::Repaired => ("pdf.repair.show", Box::new(crate::document::ShowRepairReport)),
            ely_office::Risk::Macros | ely_office::Risk::Sections => return Some(banner),
        };
        let label = Self::text(cx, key, &[]);
        Some(banner.action(label, move |window, cx| window.dispatch_action(action.boxed_clone(), cx)))
    }

    pub fn show_repair(&mut self, open: bool, cx: &mut Context<Self>) {
        self.repair_open = open;
        cx.notify();
    }

    /// MuPDF's words on what it found broken.
    pub fn repair_element(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let report = self.active_view().filter(|_| self.repair_open)?.read(cx).repair.clone()?;
        let name = self.active_tab()?.name.clone();
        let this = cx.entity();
        let dismiss = move |_: &mut Window, cx: &mut App| this.update(cx, |viewer, cx| viewer.show_repair(false, cx));
        let lines = report.lines().map(|line| div().child(line.to_string())).collect::<Vec<_>>();
        let done = Self::text(cx, "pdf.sign.close", &[]);
        let close = move |close: ely_gpui_component::overlays::Close| {
            Button::new("pdf-repair-close", done).variant(ButtonVariant::Primary).on_click(move |_, window, cx| close(window, cx))
        };
        let card = Card::new("pdf-repair", Self::text(cx, "pdf.repair.title", &[("name", &name)]), dismiss)
            .detail(Self::text(cx, "pdf.repair.detail", &[]))
            .child(div().text_sm().max_h(gpui::px(320.0)).overflow_hidden().children(lines))
            .action(close);
        Some(card.into_any_element())
    }

    fn set_exporting(&mut self, cx: &mut Context<Self>, change: impl FnOnce(&mut Exporting)) {
        if let Some(exporting) = self.exporting.as_mut() {
            change(exporting);
        }
        cx.notify();
    }

    /// The export dialog, while open.
    pub fn export_element(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let asked = self.exporting.as_ref()?;
        let optimized = (asked.kind == Kind::Optimized).then(|| self.optimize_rows(asked.how, cx));
        let this = cx.entity();
        let label = |key: &str| Self::text(cx, key, &[]);
        let segments = |id: &'static str, selected: &str, choices: &[(&str, &str)], set: fn(&mut Exporting, &str)| {
            let changed = this.clone();
            choices
                .iter()
                .fold(SegmentedControl::new(id, selected.to_string()).size(ControlSize::Sm), |control, (value, key)| {
                    control.segment(value.to_string(), Self::text(cx, key, &[]), None)
                })
                .on_change(move |value, _, cx| {
                    let value = value.to_string();
                    changed.update(cx, |viewer, cx| viewer.set_exporting(cx, |asked| set(asked, &value)))
                })
        };
        let kind = match asked.kind {
            Kind::Images => "images",
            Kind::Text => "text",
            Kind::Optimized => "optimized",
        };
        let kinds = [("images", "pdf.export.images"), ("text", "pdf.export.text"), ("optimized", "pdf.export.optimized")];
        let mut body = div().flex().flex_col().gap_3().child(segments("pdf-export-kind", kind, &kinds, |asked, value| {
            asked.kind = match value {
                "text" => Kind::Text,
                "optimized" => Kind::Optimized,
                _ => Kind::Images,
            }
        }));
        match asked.kind {
            Kind::Images => {
                let dpi = asked.dpi.to_string();
                let dpis = [("72", "pdf.export.dpi72"), ("150", "pdf.export.dpi150"), ("300", "pdf.export.dpi300")];
                let pages = if asked.all { "all" } else { "picked" };
                let rows = DescriptionList::new()
                    .item(label("pdf.export.resolution"), segments("pdf-export-dpi", &dpi, &dpis, |asked, value| asked.dpi = value.parse().expect("a listed dpi")))
                    .item(
                        label("pdf.export.pages"),
                        segments("pdf-export-pages", pages, &[("all", "pdf.export.all"), ("picked", "pdf.export.picked")], |asked, value| asked.all = value == "all"),
                    );
                body = body.child(rows);
            }
            Kind::Text => {}
            Kind::Optimized => body = body.children(optimized),
        }
        let closed = this.clone();
        let dismiss = move |_: &mut Window, cx: &mut App| {
            closed.update(cx, |viewer, cx| {
                viewer.exporting = None;
                cx.notify();
            })
        };
        let go = label("pdf.export.go");
        // MuPDF only shrinks pictures; `to` must stay below `from`.
        let ready = !(asked.kind == Kind::Optimized && asked.how.to > 0 && asked.how.from <= asked.how.to);
        let card = Card::new("pdf-export", label("pdf.export.title"), dismiss).child(body).action(ely_office::cancel(cx)).action(move |_| {
            Button::new("pdf-export-go", go)
                .variant(ButtonVariant::Primary)
                .disabled(!ready)
                .on_click(move |_, _, cx| this.update(cx, |viewer, cx| viewer.choose_export(cx)))
        });
        Some(card.into_any_element())
    }

    fn optimize_rows(&self, how: Optimizing, cx: &mut Context<Self>) -> AnyElement {
        let this = cx.entity();
        let set = |change: fn(&mut Optimizing, f64)| {
            let this = this.clone();
            move |value: f64, _: &mut Window, cx: &mut App| this.update(cx, |viewer, cx| viewer.set_exporting(cx, |asked| change(&mut asked.how, value)))
        };
        let toggled = |change: fn(&mut Optimizing, bool)| {
            let this = this.clone();
            move |on: bool, _: &mut Window, cx: &mut App| this.update(cx, |viewer, cx| viewer.set_exporting(cx, |asked| change(&mut asked.how, on)))
        };
        let shrink = how.to > 0;
        let mut rows = div().flex().flex_col().gap_2().child(
            Checkbox::new("pdf-export-compress", how.compress).label(Self::text(cx, "pdf.export.compress", &[])).on_change(toggled(|how, on| how.compress = on)),
        );
        rows = rows.child(
            Checkbox::new("pdf-export-shrink", shrink)
                .label(Self::text(cx, "pdf.export.shrink", &[]))
                .on_change(toggled(|how, on| (how.from, how.to) = if on { (300, 150) } else { (300, 0) })),
        );
        if shrink {
            let from = NumberInput::new("pdf-export-from", f64::from(how.from)).range(72.0, 2400.0).suffix("dpi").label(Self::text(cx, "pdf.export.above", &[]));
            let to = NumberInput::new("pdf-export-to", f64::from(how.to)).range(36.0, 1200.0).suffix("dpi").label(Self::text(cx, "pdf.export.to", &[]));
            let list = DescriptionList::new()
                .item(Self::text(cx, "pdf.export.above", &[]), from.on_change(set(|how, value| how.from = value.round() as u32)))
                .item(Self::text(cx, "pdf.export.to", &[]), to.on_change(set(|how, value| how.to = value.round() as u32)));
            rows = rows.child(list);
        }
        rows.into_any_element()
    }
}
