//! Edits one at a time; the view rereads after (PDF-02).

use std::{collections::BTreeSet, io};

use ely_pdf_engine::{Bookmark, Doc, Worker};
use gpui::Context;

use crate::engine::MEASURE;
use crate::view::{PdfView, Reported, sizes};

/// One edit: it answers with the history mark after it.
pub type Op = Box<dyn FnOnce(&mut Worker, Doc) -> io::Result<String> + Send>;

/// A queued edit: name, pages picked after, state seen.
pub struct Queued {
    what: &'static str,
    op: Op,
    pick: Option<BTreeSet<usize>>,
    doc: Doc,
    /// The edit may add, drop, move or reshape pages.
    pages: bool,
    /// Aimed at a point: runs on the state it meets.
    follow: bool,
}

/// The document as an edit left it.
pub struct Fresh {
    doc: Doc,
    mark: String,
    pages: usize,
    first: Vec<[f32; 4]>,
    marks: Vec<Bookmark>,
}

impl PdfView {
    /// Queues `op`; `what` names it if it fails.
    pub fn edit(
        &mut self,
        what: &'static str,
        op: impl FnOnce(&mut Worker, Doc) -> io::Result<String> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        self.queue_edit(what, None, false, op, cx);
    }

    /// Queues a point-aimed edit; only moving pages refuse it.
    pub fn edit_at(
        &mut self,
        what: &'static str,
        op: impl FnOnce(&mut Worker, Doc) -> io::Result<String> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        if self.moving || self.queue.iter().any(|queued| queued.pages) {
            let text = crate::viewer::Viewer::text(cx, "pdf.pages.changed", &[]);
            cx.emit(crate::view::Failed(text.to_string()));
            return cx.notify();
        }
        log::info!("pdf: queues {what}");
        self.queue.push_back(Queued { what, op: Box::new(op), pick: None, doc: self.doc, pages: false, follow: true });
        self.next_edit(cx);
    }

    /// Queues an edit to the pages themselves.
    pub fn edit_pages(
        &mut self,
        what: &'static str,
        op: impl FnOnce(&mut Worker, Doc) -> io::Result<String> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        self.queue_edit(what, None, true, op, cx);
    }

    /// Queues `op`; once it lands, `pick` is picked.
    pub fn queue_edit(
        &mut self,
        what: &'static str,
        pick: Option<BTreeSet<usize>>,
        pages: bool,
        op: impl FnOnce(&mut Worker, Doc) -> io::Result<String> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        log::info!("pdf: queues {what}");
        // Aimed at what is shown now; later states refuse it.
        self.queue.push_back(Queued { what, op: Box::new(op), pick, doc: self.doc, pages, follow: false });
        self.next_edit(cx);
    }

    fn next_edit(&mut self, cx: &mut Context<Self>) {
        if self.editing {
            return;
        }
        let Some(Queued { what, op, pick, doc, pages, follow }) = self.queue.pop_front() else {
            return;
        };
        // Earlier edits have landed; a point still means the same.
        let doc = if follow { self.doc } else { doc };
        (self.editing, self.moving) = (true, pages);
        self.off(
            move |worker| {
                let mark = op(worker, doc)?;
                let doc = doc.at(&mark)?;
                let pages = worker.pages(doc)?;
                let first = worker.bounds(doc, 0, pages.min(MEASURE))?;
                Ok(Fresh { doc, mark, pages, first, marks: worker.outline(doc)? })
            },
            move |view, fresh, cx| {
                (view.editing, view.moving) = (false, false);
                match fresh {
                    Ok(fresh) => {
                        view.refresh(fresh, pages, cx);
                        if let Some(pick) = pick {
                            view.anchor = pick.first().copied();
                            view.picked = pick;
                        }
                    }
                    Err(error) => {
                        view.fail(what, &error, cx);
                        // Waiting saves and closes learn the queue moved.
                        cx.emit(Reported);
                    }
                }
                view.next_edit(cx);
                cx.notify();
            },
            cx,
        );
    }

    /// Everything read before the edit goes.
    fn refresh(&mut self, fresh: Fresh, pages: bool, cx: &mut Context<Self>) {
        self.version += 1;
        self.edited += 1;
        (self.doc, self.mark) = (fresh.doc, fresh.mark);
        let measured = sizes(fresh.pages, &fresh.first);
        if pages || fresh.pages != self.sizes.len() {
            // A page may now stand where another stood.
            self.retire_tiles();
            self.whole.clear();
            self.sizes = measured;
            self.thumbs_stale.clear();
            for (_, image) in self.thumbs.drain() {
                self.retired.push(image);
            }
        } else {
            // Same pages: their last whole draws stand in.
            self.retire_tiles();
            self.thumbs_stale.extend(self.thumbs.keys().copied());
            let read = fresh.first.len();
            self.sizes[..read].copy_from_slice(&measured[..read]);
        }
        self.marks = fresh.marks;
        self.failed.clear();
        self.unread.clear();
        self.text.clear();
        self.asking_text.clear();
        self.thumbs_failed.clear();
        // Indices shift with any edit; aims start over.
        (self.layers, self.aim, self.drag) = (Default::default(), None, None);
        (self.notes, self.sketch, self.marking, self.form_names) = (Default::default(), None, None, None);
        self.links.clear();
        (self.files, self.mark_pick) = (None, None);
        self.sign_at = None;
        // Page numbers may have moved under the marks.
        self.redactions.clear();
        (self.found, self.chosen, self.selection) = (Vec::new(), None, None);
        // Page numbers changed; a range pick starts over.
        self.picked.retain(|page| *page < fresh.pages);
        self.anchor = None;
        self.current = self.current.min(fresh.pages - 1);
        self.measure(fresh.first.len(), cx);
        cx.emit(Reported);
    }

    /// The file now holds history `mark`.
    pub fn saved(&mut self, mark: &str, cx: &mut Context<Self>) {
        self.saved = mark.to_string();
        self.saves += 1;
        // The file changed; read it again.
        self.signatures = None;
        self.unread.retain(|unread| !matches!(unread, crate::view::Unread::Signatures(_)));
        cx.emit(Reported);
    }

    /// Changed since the load or the last save.
    pub fn modified(&self) -> bool {
        self.mark != self.saved
    }

    /// Every edit asked for has landed.
    pub fn settled(&self) -> bool {
        self.queue.is_empty() && !self.editing
    }

    pub fn undo(&mut self, redo: bool, cx: &mut Context<Self>) {
        let what = if redo { "pdf.doing.redo" } else { "pdf.doing.undo" };
        self.edit_pages(what, move |worker, doc| worker.undo(doc, redo), cx);
    }
}
