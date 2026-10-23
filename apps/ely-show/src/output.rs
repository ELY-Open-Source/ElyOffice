//! PDF, slide pictures and handouts, saved or printed (SHOW-13).

use std::{io, path::PathBuf};

use ely_gpui_component::primitives::Severity;
use ely_lok::{Doc, Pdf, Worker};
use ely_office::{in_window, prelude::*};
use ely_ui::commands::{Export, Print};
use gpui::{Context, TaskExt as _, Window};

use crate::deck::Deck;
use crate::dialog::Dialog;

mod dialog;
pub use dialog::output_dialog;

/// Slide pictures are this wide.
const PICTURE: u32 = 1920;

/// What goes out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Slides,
    Pictures,
    Handouts,
}

/// Which slides, by number from one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Range {
    All,
    Current,
    Slides(u32, u32),
}

/// The output dialog's choices.
#[derive(Clone, Debug)]
pub struct Output {
    pub tab: u64,
    pub print: bool,
    pub kind: Kind,
    pub range: Range,
    /// Slides on a handout page.
    pub per: u8,
    /// The shown slide, from one; none in master view.
    pub current: Option<u32>,
    /// Each slide, hidden or not.
    pub hidden: Vec<bool>,
}

impl Output {
    /// First and last slide; None is all.
    pub fn pages(&self) -> Option<(u32, u32)> {
        match self.range {
            Range::All => None,
            Range::Current => self.current.map(|at| (at, at)),
            Range::Slides(from, to) => Some((from.min(to), from.max(to))),
        }
    }
}

/// Runs `work` on a saved copy of `doc`.
fn on_copy<T>(
    worker: &mut Worker,
    doc: &Doc,
    (copy, scheme): (&std::path::Path, Option<&'static str>),
    work: impl FnOnce(&mut Worker, &Doc) -> io::Result<T>,
) -> io::Result<T> {
    let url = ely_lok::file_url(copy);
    doc.save_as(worker, &url, "odp")?;
    let done = loaded(worker, &url, scheme, work);
    std::fs::remove_file(copy)?;
    done
}

fn loaded<T>(
    worker: &mut Worker,
    url: &str,
    scheme: Option<&'static str>,
    work: impl FnOnce(&mut Worker, &Doc) -> io::Result<T>,
) -> io::Result<T> {
    let made = worker.load(url)?;
    let done = match scheme {
        Some(scheme) => worker.run(&made, "view.theme", &[scheme]).and_then(|()| work(worker, &made)),
        None => work(worker, &made),
    };
    let closed = worker.close_doc(made);
    let done = done?;
    closed?;
    Ok(done)
}

/// Writes slide `part` as a PNG of `size` pixels.
fn picture(worker: &mut Worker, doc: &Doc, part: usize, size: (u32, u32), to: std::fs::File) -> io::Result<()> {
    let mut pixels = doc.paint_part(worker, (part, false), size)?;
    // The kernel paints BGRA.
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    png(pixels, size, to)
}

/// Encodes RGBA `pixels` as PNG; the last write checked.
fn png(pixels: Vec<u8>, size: (u32, u32), to: impl io::Write) -> io::Result<()> {
    use image::ImageEncoder as _;
    if pixels.len() != size.0 as usize * size.1 as usize * 4 {
        return Err(io::Error::other("the slide picture has the wrong size"));
    }
    let mut out = io::BufWriter::new(to);
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&pixels, size.0, size.1, image::ExtendedColorType::Rgba8)
        .map_err(io::Error::other)?;
    io::Write::flush(&mut out)
}

/// The shown slides of `range` and their files.
fn targets(hidden: &[bool], range: Option<(u32, u32)>, (folder, stem): (&std::path::Path, &str)) -> Vec<(usize, PathBuf)> {
    let (from, to) = range.unwrap_or((1, hidden.len() as u32));
    let width = hidden.len().to_string().len();
    (from..=to)
        .map(|number| number as usize - 1)
        .filter(|part| hidden.get(*part) == Some(&false))
        .map(|part| (part, folder.join(format!("{stem}-{:0width$}.png", part + 1))))
        .collect()
}

impl Deck {
    pub fn on_export(&mut self, _: &Export, _: &mut Window, cx: &mut Context<Self>) {
        self.ask_output(false, cx);
    }

    pub fn on_print(&mut self, _: &Print, _: &mut Window, cx: &mut Context<Self>) {
        self.ask_output(true, cx);
    }

    /// Reads the slides, not masters, then asks what goes out.
    fn ask_output(&mut self, print: bool, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            let text = Deck::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        let id = tab.id;
        let current = (!tab.extra.masters)
            .then(|| u32::try_from(tab.extra.slides.current + 1).expect("a slide number"));
        let asked = self.ask_dialog();
        self.read_in(
            id,
            |worker, doc| ely_lok::show::slide_show(&worker.values(doc, ely_lok::show::SLIDE_SHOW)?),
            move |deck, read, _, cx| match read {
                Ok(slides) if deck.dialog_due(asked, id) => {
                    // Refused while another dialog is open; it says so.
                    let output = Output {
                        tab: id,
                        print,
                        kind: Kind::Slides,
                        range: Range::All,
                        per: 3,
                        current,
                        hidden: slides.iter().map(|slide| slide.hidden).collect(),
                    };
                    let _ = deck.open_dialog(Dialog::Output(output), cx);
                }
                Ok(_) => log::info!("show: the output dialog is no longer due"),
                Err(error) => deck.engine_failed(&format!("the slides: {error}"), cx),
            },
            cx,
        );
    }

    /// Writes what the dialog asks, then saves or prints it.
    pub(crate) fn output(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Output(output)) = self.dialog.take() else {
            return;
        };
        cx.notify();
        if !self.settled(output.tab, cx) {
            let tab = output.tab;
            self.dialog = Some(Dialog::Output(output));
            return self.when_settled(tab, |deck, _, cx| deck.output(cx));
        }
        let Some(source) = self.tab(output.tab) else {
            return;
        };
        let work = source.work.clone();
        if output.print {
            let path = work.join("print.pdf");
            let printing = path.clone();
            return self.write_pdf(&output, path, cx, move |_, cx| {
                ely_office::print_later(printing, cx)
            });
        }
        let Some(dir) = ely_office::folder(source) else {
            return self.panel_failed("no documents folder", cx);
        };
        let stem = (source.name.rsplit_once('.'))
            .map_or(source.name.to_string(), |(stem, _)| stem.to_string());
        if output.kind == Kind::Pictures {
            return self.pick(true, cx, move |deck, folder, cx| deck.write_pictures(&output, (folder, stem), cx));
        }
        let chosen = cx.prompt_for_new_path(&dir, Some(&format!("{stem}.pdf")));
        cx.spawn(async move |this, cx| {
            let chosen = match chosen.await {
                Ok(Ok(path)) => Ok(path),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            in_window(&this, cx, |deck, _, cx| match chosen {
                Ok(Some(path)) => deck.export_pdf(&output, path, cx),
                Ok(None) => log::info!("show: the export panel was cancelled"),
                Err(error) => deck.panel_failed(&error, cx),
            })
        })
        .detach_and_log_err(cx);
    }

    pub(crate) fn export_pdf(&mut self, output: &Output, path: PathBuf, cx: &mut Context<Self>) {
        match self.holder(&path) {
            Ok(None) => {}
            Ok(Some(_)) => {
                let name = ely_office::files::file_name(&path);
                let text = Deck::text(cx, "show.output.open", &[("name", &name)]);
                return self.toast(text, Severity::Danger, cx);
            }
            Err(error) => return self.output_failed(&error.to_string(), cx),
        }
        let (name, opened) = (ely_office::files::file_name(&path), path.clone());
        self.write_pdf(output, path, cx, move |deck, cx| {
            let text = Deck::text(cx, "show.output.done", &[("name", &name)]);
            deck.toast_output(text, opened.clone(), cx);
        });
    }

    /// Writes slides or handouts as a PDF, from a copy.
    fn write_pdf(
        &mut self,
        output: &Output,
        path: PathBuf,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, &mut Context<Self>) + 'static,
    ) {
        let Some(tab) = self.tab(output.tab) else {
            return;
        };
        let pdf = Pdf {
            pages: output.pages(),
            handouts: if output.kind == Kind::Handouts { output.per } else { 0 },
            ..Pdf::default()
        };
        log::info!("show: writes a PDF ({pdf:?})");
        let (url, copy) = (ely_lok::file_url(&path), tab.work.join("output.odp"));
        self.read_in(
            output.tab,
            move |worker, doc| on_copy(worker, doc, (&copy, None), |worker, made| made.export_pdf(worker, &url, &pdf)),
            move |deck, written, _, cx| match written {
                Ok(()) => then(deck, cx),
                Err(error) => deck.output_failed(&error.to_string(), cx),
            },
            cx,
        );
    }

    /// One PNG a shown slide, light, into `folder`.
    pub(crate) fn write_pictures(&mut self, output: &Output, (folder, stem): (PathBuf, String), cx: &mut Context<Self>) {
        let Some(tab) = self.tab(output.tab) else {
            return;
        };
        let range = output.pages();
        let (w, h) = tab.extra.size;
        let size = (PICTURE, (f64::from(PICTURE) * h as f64 / w.max(1) as f64).round() as u32);
        let (copy, light) = (tab.work.join("output.odp"), ely_lok::SCHEME);
        let (shown, opened) = (folder.display().to_string(), folder.clone());
        log::info!("show: writes slide pictures");
        self.read_in(
            output.tab,
            move |worker, doc| {
                // Which slides show is the copy's, as it is written.
                on_copy(worker, doc, (&copy, Some(light)), |worker, made| {
                    let slides = ely_lok::show::slide_show(&worker.values(made, ely_lok::show::SLIDE_SHOW)?)?;
                    let hidden: Vec<bool> = slides.iter().map(|slide| slide.hidden).collect();
                    let files = targets(&hidden, range, (&folder, &stem));
                    let paths: Vec<PathBuf> = files.iter().map(|(_, path)| path.clone()).collect();
                    let claimed = ely_files::claim_all(&paths)?;
                    let written = (files.iter().zip(claimed))
                        .try_for_each(|((part, _), file)| picture(worker, made, *part, size, file));
                    // A failure leaves none of this run's files.
                    if let Err(error) = &written
                        && let Err(left) = files.iter().try_for_each(|(_, path)| std::fs::remove_file(path))
                    {
                        return Err(io::Error::other(format!("{error}; then {left}")));
                    }
                    written.map(|()| files.len())
                })
            },
            move |deck, written, _, cx| match written {
                Ok(count) => {
                    let n = count.to_string();
                    let text = ely_ui::i18n::counted(cx, "show.output.pictures_done", count as u64, &[("n", &n), ("folder", &shown)]);
                    deck.toast_output(text, opened.clone(), cx);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    let text = Deck::text(cx, "show.output.taken", &[("name", &error.to_string())]);
                    deck.toast(text, Severity::Danger, cx);
                }
                Err(error) => deck.output_failed(&error.to_string(), cx),
            },
            cx,
        );
    }

    fn output_failed(&self, error: &str, cx: &mut Context<Self>) {
        let text = Deck::text(cx, "show.output.failed", &[("error", error)]);
        self.toast(text, Severity::Danger, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Takes `room` bytes, then the disk is full.
    struct Full {
        room: usize,
    }

    impl io::Write for Full {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            match self.room {
                0 => Err(io::Error::new(io::ErrorKind::StorageFull, "full")),
                room => {
                    let took = bytes.len().min(room);
                    self.room -= took;
                    Ok(took)
                }
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_full_disk_at_the_last_write_is_an_error() {
        let pixels = vec![200u8; 64 * 64 * 4];
        let mut whole = Vec::new();
        png(pixels.clone(), (64, 64), &mut whole).expect("encodes");
        let short = Full { room: whole.len() - 1 };
        let failed = png(pixels, (64, 64), short).expect_err("one byte short");
        assert_eq!(failed.kind(), io::ErrorKind::StorageFull);
    }

    #[test]
    fn targets_skip_hidden_slides_and_pad_numbers() {
        let hidden = [false, true, false, false, false, false, false, false, false, false];
        let out = std::path::Path::new("/out");
        let files = targets(&hidden, Some((1, 3)), (out, "deck"));
        let paths: Vec<&std::path::Path> = files.iter().map(|(_, p)| p.as_path()).collect();
        assert_eq!(paths, [out.join("deck-01.png"), out.join("deck-03.png")]);
    }
}
