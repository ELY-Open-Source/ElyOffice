//! Linking empties a slide; the scores take its place.

use std::io;

use ely_lok::{Doc, Worker};

/// Empties `parts` but their comments, as one undo step.
pub(super) fn clear(worker: &mut Worker, doc: &Doc, parts: &[usize]) -> io::Result<()> {
    let slides: Vec<String> = parts.iter().map(usize::to_string).collect();
    let now = worker.now(doc)?;
    worker.run(&now, "slide.clear", &[&slides.join(";")])
}
