//! Linking empties a slide; the scores take its place.

use std::io;

use ely_lok::{Doc, Worker, show::OBJECTS};

/// A filled placeholder leaves an empty one behind.
const PASSES: usize = 3;

/// The Escape key, as the view sends it.
const ESCAPE: i32 = 1281;

/// Deletes every object on `parts`, then shows `back`.
pub(super) fn clear(worker: &mut Worker, doc: &Doc, parts: &[usize], back: usize) -> io::Result<()> {
    leave_text(worker, doc)?;
    let cleared = parts.iter().try_for_each(|&part| {
        doc.show_sheet(worker, part)?;
        empty(worker, doc)
    });
    let shown = doc.show_sheet(worker, back);
    cleared.and(shown)
}

/// Escapes a text edit; deleted mid-edit, text mode sticks.
fn leave_text(worker: &mut Worker, doc: &Doc) -> io::Result<()> {
    let now = worker.now(doc)?;
    let shown = ely_lok::show::objects(&worker.values(&now, OBJECTS)?)?;
    if !shown.selection.is_some_and(|selection| selection.editing) {
        return Ok(());
    }
    // The first ends the edit, the second unmarks.
    (0..2).try_for_each(|_| worker.post_key(doc, 0, ESCAPE))
}

/// Empties the shown slide, or says what stayed.
fn empty(worker: &mut Worker, doc: &Doc) -> io::Result<()> {
    let count = |worker: &mut Worker| -> io::Result<usize> {
        let now = worker.now(doc)?;
        Ok(ely_lok::show::objects(&worker.values(&now, OBJECTS)?)?.objects.len())
    };
    for _ in 0..PASSES {
        // Marks start over, in the order listed.
        let now = worker.now(doc)?;
        worker.run(&now, "object.deselect", &["false"])?;
        let found = count(worker)?;
        if found == 0 {
            return Ok(());
        }
        let picks: Vec<[String; 2]> = (0..found).map(|at| [at.to_string(), (at > 0).to_string()]).collect();
        let values: Vec<[&str; 2]> = picks.iter().map(|[at, add]| [at.as_str(), add.as_str()]).collect();
        let commands: Vec<(&str, &[&str])> = values.iter().map(|value| ("object.select", value.as_slice())).collect();
        let now = worker.now(doc)?;
        worker.run_all(&now, &commands)?;
        let now = worker.now(doc)?;
        worker.run(&now, "object.delete", &[])?;
    }
    match count(worker)? {
        0 => Ok(()),
        left => Err(io::Error::other(format!("{left} objects stayed on the slide"))),
    }
}
