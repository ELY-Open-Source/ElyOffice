//! The Original view: a read-only copy, every change rejected (#126).

use std::{io, path::Path};

use ely_lok::{Doc, Feed, Layout, Waiter, Worker};

/// The copy, laid out, ready for its own view.
pub(super) struct Prepared {
    pub doc: Doc,
    pub laid: Layout,
    pub feed: Feed,
    pub waiter: Waiter,
}

/// Copies the tab, rejects all, locks the copy.
pub(super) fn prepare(worker: &mut Worker, doc: &Doc, work: &Path) -> io::Result<Prepared> {
    let copy = work.join("original.odt");
    doc.save_as(worker, &ely_lok::file_url(&copy), "odt")?;
    let original = worker.load(&ely_lok::file_url(&copy))?;
    let ready = reject_and_lock(worker, &original).and_then(|()| ely_lok::layout(worker, &original));
    match ready {
        Ok(laid) => {
            log::info!("doc: the original view is ready");
            Ok(Prepared { doc: original, laid, feed: worker.feed(), waiter: worker.waiter() })
        }
        Err(error) => {
            worker.close_doc(original)?;
            Err(error)
        }
    }
}

/// The copy's changes rejected; its view takes no edits.
fn reject_and_lock(worker: &mut Worker, original: &Doc) -> io::Result<()> {
    // Rejecting none leaves no undo step: the check refuses it.
    let json = worker.values(original, ".uno:AcceptTrackedChanges")?;
    if !ely_lok::review::changes(&json)?.is_empty() {
        worker.run(original, "track.reject_all", &[])?;
    }
    original.set_read_only(worker, true)
}
