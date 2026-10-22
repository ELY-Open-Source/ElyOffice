//! MuPDF as the shell's engine (PDF-01, PERF-01).

use std::{
    convert::Infallible,
    io,
    path::Path,
    sync::{Arc, Mutex},
};

use ely_office::{Engine, Host, Load};
use ely_pdf_engine::{Bookmark, Doc, Worker};
use gpui::{App, AppContext as _, Context, Entity, FocusHandle, Window};

use crate::view::{Failed, PdfView, Reported};

/// Pages measured with the load.
pub const MEASURE: usize = 100;

/// The worker, started on the first open.
#[derive(Default)]
pub struct Mupdf(Option<Worker>);

impl Mupdf {
    /// A holder around a running worker, for tests.
    #[cfg(test)]
    pub fn with(worker: Worker) -> Self {
        Mupdf(Some(worker))
    }

    #[cfg(test)]
    pub fn started(&self) -> bool {
        self.0.is_some()
    }

    /// The worker, started now if need be.
    pub fn get(&mut self) -> io::Result<&mut Worker> {
        if self.0.is_none() {
            log::info!("pdf: starts the worker");
            self.0 = Some(Worker::spawn()?);
        }
        Ok(self.0.as_mut().expect("just started"))
    }
}

/// A load: the page count, first sizes, bookmarks, history.
pub struct Loaded {
    pub doc: Doc,
    pub pages: usize,
    pub first: Vec<[f32; 4]>,
    pub marks: Vec<Bookmark>,
    pub mark: String,
    /// `proto::may` bits.
    pub allowed: u32,
    /// Fields holding a signature.
    pub signed: usize,
    /// MuPDF's words when it had to repair the file.
    pub repaired: Option<String>,
}

/// The MuPDF engine.
pub struct Pdf;

/// What a tab needs; locked or empty files fail.
fn read(worker: &mut Worker, doc: Doc, locked: bool, password: &str) -> io::Result<Loaded> {
    if locked && !worker.unlock(doc, password)? {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "a password is needed"));
    }
    let pages = worker.pages(doc)?;
    if pages == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "no pages to show"));
    }
    let first = worker.bounds(doc, 0, pages.min(MEASURE))?;
    let (marks, mark, allowed) = (worker.outline(doc)?, worker.history(doc)?, worker.permissions(doc)?);
    let (signed, repaired) = (worker.signed(doc)?, worker.repair_report(doc)?);
    Ok(Loaded { doc, pages, first, marks, mark, allowed, signed, repaired })
}

impl Engine for Pdf {
    type Worker = Mupdf;
    type Doc = Doc;
    type View = PdfView;
    type Loaded = Loaded;
    type Import = Infallible;

    fn load(worker: &mut Mupdf, from: Load<Infallible>) -> io::Result<Loaded> {
        let (path, password) = match from {
            Load::File(path, password) => (path, password),
            Load::Import(_, never) => match *never {},
        };
        let worker = worker.get()?;
        let opened = worker.open(path)?;
        read(worker, opened.doc, opened.locked, password).inspect_err(|_| {
            // A file not shown leaves nothing open.
            if let Err(error) = worker.close(opened.doc) {
                log::error!("pdf: closing a file not shown: {error}");
            }
        })
    }

    fn open(
        worker: &Arc<Mutex<Mupdf>>,
        loaded: Loaded,
        _: &mut Window,
        cx: &mut App,
    ) -> io::Result<(Doc, Entity<PdfView>, String)> {
        let (doc, mark) = (loaded.doc, loaded.mark.clone());
        let zoom = ely_ui::settings::current(cx).doc_zoom;
        let view = cx.new(|cx| {
            let mut view = PdfView::new(Arc::clone(worker), loaded, cx);
            // Automatic keeps the page width.
            if let ely_platform::settings::DocZoom::Percent(percent) = zoom {
                view.zoom = crate::layout::Zoom::Scale(percent as f32 / 100.0 * crate::layout::ACTUAL);
                log::info!("pdf: opens at {percent}%");
            }
            view
        });
        Ok((doc, view, mark))
    }

    fn risk(loaded: &Loaded) -> Option<ely_office::Risk> {
        match (&loaded.repaired, loaded.signed) {
            (Some(_), _) => Some(ely_office::Risk::Repaired),
            (None, 0) => None,
            (None, _) => Some(ely_office::Risk::Signed),
        }
    }

    fn unsound(worker: &mut Mupdf, doc: &Doc) -> io::Result<Option<ely_office::Risk>> {
        Ok(worker.get()?.check_pictures(*doc)?.then_some(ely_office::Risk::Repaired))
    }

    fn history(worker: &mut Mupdf, doc: &Doc) -> io::Result<String> {
        worker.get()?.history(*doc)
    }

    fn write(worker: &mut Mupdf, doc: &Doc, path: &Path, format: &str) -> io::Result<()> {
        match format {
            "pdf" => worker.get()?.save(*doc, path),
            other => Err(io::Error::new(io::ErrorKind::Unsupported, format!("writing .{other}"))),
        }
    }

    /// MuPDF's save writes a snapshot; the document is untouched.
    fn snapshot(worker: &mut Mupdf, doc: &Doc, path: &Path, format: &str) -> io::Result<()> {
        Self::write(worker, doc, path, format)
    }

    fn close(worker: &mut Mupdf, doc: Doc) -> io::Result<()> {
        worker.get()?.close(doc)
    }

    fn retire(view: &Entity<PdfView>, cx: &mut App) -> ely_gpui_component::primitives::Retired {
        view.update(cx, |view, _| view.retire_all())
    }

    fn ended(worker: &mut Mupdf) -> Option<String> {
        worker.0.as_mut()?.ended()
    }

    /// The next load starts a new worker.
    fn restart(_: ely_platform::AppId, worker: &mut Mupdf) -> io::Result<()> {
        worker.0 = None;
        Ok(())
    }

    fn settled(view: &PdfView) -> bool {
        view.settled()
    }

    fn edited(view: &PdfView) -> u64 {
        view.edited
    }

    fn edit_pending(view: &PdfView) -> bool {
        !view.settled()
    }

    fn focus(view: &PdfView) -> FocusHandle {
        view.focus.clone()
    }

    fn saved(view: &Entity<PdfView>, mark: &str, cx: &mut App) {
        view.update(cx, |view, cx| view.saved(mark, cx));
    }

    fn watch<H: Host<Engine = Self>>(
        view: &Entity<PdfView>,
        cx: &mut Context<H>,
        reported: impl Fn(&mut H, &Entity<PdfView>, bool, &mut Context<H>) + 'static,
        failed: impl Fn(&mut H, &str, &mut Context<H>) + 'static,
    ) {
        cx.subscribe(view, move |host, _, error: &Failed, cx| failed(host, &error.0, cx))
            .detach();
        cx.subscribe(view, move |host, view, _: &Reported, cx| {
            let modified = view.read(cx).modified();
            reported(host, &view, modified, cx)
        })
        .detach();
    }
}
