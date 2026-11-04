//! Crop, extract and split (PDF-02).

use std::{
    io::{self, Write as _},
    path::{Path, PathBuf},
};

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::NumberInput,
    overlays::Dialog as Card,
    primitives::Severity,
};
use ely_office::{Closing as _, Saving as _, Tabs as _, in_window};
use gpui::{AnyElement, App, Context, Entity, IntoElement, ParentElement as _, TaskExt as _, Window};

use crate::viewer::Viewer;

/// Points in a millimetre.
const MM: f32 = 72.0 / 25.4;

/// The viewer's own dialogs.
pub enum Ask {
    /// Pages to crop, margins in mm: top, right, bottom, left.
    Crop { tab: u64, pages: Vec<usize>, margins: [f32; 4] },
    /// Pages in each file.
    Split { tab: u64, every: usize },
}

/// Files of `every` pages from `count`: (first, last) each.
pub fn parts(count: usize, every: usize) -> Vec<(usize, usize)> {
    (0..count).step_by(every.max(1)).map(|first| (first, (first + every).min(count) - 1)).collect()
}

/// Margins as seen through a view turned `rotate` degrees, unturned.
pub fn unturn(seen: [f32; 4], rotate: i32) -> [f32; 4] {
    let turns = (rotate.rem_euclid(360) / 90) as usize;
    std::array::from_fn(|side| seen[(side + turns) % 4])
}

/// A file name's stem.
fn stem(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

impl Viewer {
    pub fn ask_crop(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let (Some(tab), Some(view)) = (self.core.active(), self.active_view()) else {
            return self.no_document(cx);
        };
        let pages = view.read(cx).chosen_pages();
        self.ask = Some(Ask::Crop { tab, pages, margins: [0.0; 4] });
        cx.notify();
    }

    pub fn ask_split(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active() else {
            return self.no_document(cx);
        };
        self.ask = Some(Ask::Split { tab, every: 1 });
        cx.notify();
    }

    pub(crate) fn crop(&mut self, cx: &mut Context<Self>) {
        let Some(Ask::Crop { tab, pages, margins }) = self.ask.take() else {
            return;
        };
        let Some(view) = self.tab(tab).map(|tab| tab.view.clone()) else {
            return;
        };
        let turned = unturn(margins, view.read(cx).rotate);
        let points = turned.map(|side| side * MM);
        view.update(cx, |view, cx| {
            view.edit_pages("pdf.doing.crop", move |worker, doc| worker.crop_pages(doc, &pages, points), cx)
        });
        cx.notify();
    }

    /// Asks where, then writes the chosen pages there.
    pub fn extract(&mut self, cx: &mut Context<Self>) {
        let (Some(tab), Some(view)) = (self.active_tab(), self.active_view()) else {
            return self.no_document(cx);
        };
        let (id, pages) = (tab.id, view.read(cx).chosen_pages());
        let Some(dir) = ely_office::folder(tab) else {
            return self.panel_failed(&ely_ui::i18n::text(cx, "error.no_documents", &[]), cx);
        };
        let name = format!("{}-{}.pdf", stem(&tab.name), pages[0] + 1);
        let chosen = cx.prompt_for_new_path(&dir, Some(&name));
        cx.spawn(async move |this, cx| {
            let chosen = match chosen.await {
                Ok(Ok(path)) => Ok(path),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            in_window(&this, cx, |viewer, _, cx| match chosen {
                Ok(Some(path)) => viewer.write_pages(id, pages, path, cx),
                Ok(None) => log::info!("pdf: the extract panel was cancelled"),
                Err(error) => viewer.panel_failed(&error, cx),
            })
        })
        .detach_and_log_err(cx);
    }

    /// Writes `pages` to `target` once tab `id` settles.
    pub(crate) fn write_pages(&mut self, id: u64, pages: Vec<usize>, target: PathBuf, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |viewer, _, cx| viewer.write_pages(id, pages, target, cx));
        }
        let Some(tab) = self.tab(id) else {
            return;
        };
        let (name, opened) = (ely_office::files::file_name(&target), target.clone());
        match self.holder(&target) {
            Ok(None) => {}
            Ok(Some(_)) => return self.toast(Self::text(cx, "pdf.extract.taken", &[("name", &name)]), Severity::Danger, cx),
            Err(error) => return self.failed_write(&name, &error, cx),
        }
        let (work, open, revision) = (tab.work.join("extract.pdf"), tab.session.path().map(PathBuf::from), tab.session.revision());
        log::info!("pdf: extracts {} pages", pages.len());
        self.read_in(
            id,
            move |worker, doc| {
                worker.get()?.extract(*doc, &pages, &work)?;
                let bytes = std::fs::read(&work)?;
                std::fs::remove_file(&work)?;
                ely_files::export(&target, open.as_deref(), &bytes, revision).map(|_| ())
            },
            move |viewer, done, _, cx| match done {
                Ok(()) => viewer.toast_output(Self::text(cx, "pdf.extract.done", &[("name", &name)]), opened.clone(), cx),
                Err(error) => viewer.failed_write(&name, &error, cx),
            },
            cx,
        );
    }

    fn failed_write(&self, name: &str, error: &io::Error, cx: &mut App) {
        if ely_pdf_engine::refused(error).is_some() {
            return self.failed(error, cx);
        }
        // Engine text may quote the document: the user sees it.
        log::error!("pdf: a write failed ({:?})", error.kind());
        let text = Self::text(cx, "office.error.save", &[("name", name), ("error", &ely_ui::i18n::error(cx, error))]);
        self.toast_unlogged(text, Severity::Danger, cx);
    }

    /// Asks for a folder, then writes the parts there.
    fn split(&mut self, cx: &mut Context<Self>) {
        let Some(Ask::Split { tab, every }) = self.ask.take() else {
            return;
        };
        cx.notify();
        self.pick(true, cx, move |viewer, folder, cx| viewer.write_parts(tab, every, folder, cx));
    }

    pub(crate) fn write_parts(&mut self, id: u64, every: usize, folder: PathBuf, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |viewer, _, cx| viewer.write_parts(id, every, folder, cx));
        }
        let (Some(tab), Some(view)) = (self.tab(id), self.tab(id).map(|tab| tab.view.clone())) else {
            return;
        };
        let parts = parts(view.read(cx).sizes.len(), every);
        let stem = stem(&tab.name).to_string();
        let targets: Vec<PathBuf> = (1..=parts.len()).map(|n| folder.join(format!("{stem}-{n}.pdf"))).collect();
        let opened = folder.clone();
        let work = tab.work.join("part.pdf");
        log::info!("pdf: splits into {} files", parts.len());
        self.read_in(
            id,
            move |worker, doc| {
                let claimed = ely_files::claim_all(&targets)?;
                let written = (parts.iter().zip(claimed)).try_for_each(|((first, last), file)| {
                    let pages: Vec<usize> = (*first..=*last).collect();
                    worker.get()?.extract(*doc, &pages, &work)?;
                    copy_into(&work, file)
                });
                // A failure leaves none of this run's files.
                written.map_err(|error| match targets.iter().try_for_each(std::fs::remove_file) {
                    Ok(()) => error,
                    Err(left) => ely_ipc::Worded::then(error, left),
                })?;
                Ok(targets.len())
            },
            move |viewer, done, _, cx| match done {
                Ok(count) => viewer.toast_output(ely_ui::i18n::counted(cx, "pdf.split.done", count as u64, &[("count", &count.to_string())]), opened.clone(), cx),
                Err(error) => viewer.failed_write(&stem, &error, cx),
            },
            cx,
        );
    }

    /// The open dialog of the viewer's own, if any.
    pub fn ask_element(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let this = cx.entity();
        let label = |key: &str| Self::text(cx, key, &[]);
        let dismiss = {
            let this = this.clone();
            move |_: &mut Window, cx: &mut App| {
                this.update(cx, |viewer, cx| {
                    viewer.ask = None;
                    cx.notify();
                })
            }
        };
        let card = match self.ask.as_ref()? {
            Ask::Crop { pages, margins, .. } => {
                let title = ely_ui::i18n::counted(cx, "pdf.crop.title", pages.len() as u64, &[("count", &pages.len().to_string())]);
                let sides = ["top", "right", "bottom", "left"];
                let rows = sides.iter().enumerate().fold(DescriptionList::new(), |rows, (at, side)| {
                    let name = label(&format!("pdf.crop.{side}"));
                    rows.item(name.clone(), margin(&this, at, margins[at], name))
                });
                let go = label("pdf.crop.go");
                Card::new("pdf-crop", title, dismiss).child(rows).action(ely_office::cancel(cx)).action(move |_| {
                    Button::new("pdf-crop-go", go)
                        .variant(ButtonVariant::Primary)
                        .on_click(move |_, _, cx| this.update(cx, |viewer, cx| viewer.crop(cx)))
                })
            }
            Ask::Split { every, .. } => {
                let changed = this.clone();
                let field = NumberInput::new("pdf-split-every", *every as f64)
                    .range(1.0, 100_000.0)
                    .label(label("pdf.split.every"))
                    .on_commit(move |value, _, cx| {
                        changed.update(cx, |viewer, _| {
                            if let Some(Ask::Split { every, .. }) = viewer.ask.as_mut() {
                                *every = value.round() as usize;
                            }
                        })
                    });
                let rows = DescriptionList::new().item(label("pdf.split.every"), field);
                let go = label("pdf.split.go");
                Card::new("pdf-split", label("pdf.split.title"), dismiss).child(rows).action(ely_office::cancel(cx)).action(move |_| {
                    Button::new("pdf-split-go", go)
                        .variant(ButtonVariant::Primary)
                        .on_click(move |_, _, cx| this.update(cx, |viewer, cx| viewer.split(cx)))
                })
            }
        };
        Some(card.into_any_element())
    }
}

/// A margin field in millimetres.
fn margin(this: &Entity<Viewer>, at: usize, value: f32, name: gpui::SharedString) -> NumberInput {
    let this = this.clone();
    NumberInput::new(("pdf-crop", at), f64::from(value))
        .range(0.0, 1000.0)
        .suffix("mm")
        .label(name)
        .on_commit(move |value, _, cx| {
            this.update(cx, |viewer, _| {
                if let Some(Ask::Crop { margins, .. }) = viewer.ask.as_mut() {
                    margins[at] = value as f32;
                }
            })
        })
}

/// Copies a written part into its claimed file.
pub(crate) fn copy_into(from: &Path, mut to: std::fs::File) -> io::Result<()> {
    to.write_all(&std::fs::read(from)?)?;
    to.sync_all()?;
    std::fs::remove_file(from)
}

#[cfg(test)]
mod tests {
    #[test]
    fn parts_cover_every_page_once() {
        assert_eq!(super::parts(5, 2), [(0, 1), (2, 3), (4, 4)]);
        assert_eq!(super::parts(3, 5), [(0, 2)]);
        assert_eq!(super::parts(2, 1), [(0, 0), (1, 1)]);
    }

    #[test]
    fn a_turned_view_crops_the_side_it_shows() {
        // Turned right, the page's left side shows on top.
        assert_eq!(super::unturn([1.0, 2.0, 3.0, 4.0], 90), [2.0, 3.0, 4.0, 1.0]);
        assert_eq!(super::unturn([1.0, 2.0, 3.0, 4.0], 0), [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(super::unturn([1.0, 2.0, 3.0, 4.0], 270), [4.0, 1.0, 2.0, 3.0]);
    }
}
