//! Opens, marks "Ely edit", saves, reopens fresh; prints JSON timings.

use std::{io, path::Path, time::Instant};

use ely_lok::{Doc, Worker};

const MARK: &str = "Ely edit";

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [input, output] = args.as_slice() else {
        return Err(io::Error::other("usage: office-roundtrip <in> <out>"));
    };
    let (input, output) = (std::path::absolute(input)?, std::path::absolute(output)?);
    let install = ely_lok::install_dir()?;
    let scratch = tempfile::tempdir()?;
    let started = Instant::now();
    let mut worker = Worker::spawn(&install, &scratch.path().join("first"))?;
    worker.ready()?;
    let up = started.elapsed();
    let loading = Instant::now();
    let doc = worker.load(&ely_lok::file_url(&input))?;
    let load = loading.elapsed();
    let parts = count(&mut worker, &doc)?;
    match (doc.is_sheet(), doc.is_deck()) {
        (true, _) => worker.run(&doc, "cell.set", &["Calc!A12", MARK])?,
        (false, deck) => {
            let find = if deck { "Point 1 " } else { "Detail 1.1" };
            let with = format!("{MARK} {find}");
            worker.run(&doc, "text.replace", &[find, "false", "3", "0", "0", "1", &with, "true"])?
        }
    }
    let format = output.extension().and_then(|ext| ext.to_str()).ok_or_else(|| io::Error::other("no extension"))?;
    let saving = Instant::now();
    doc.save_as(&mut worker, &ely_lok::file_url(&output), format)?;
    let save = saving.elapsed();
    drop(worker);
    let mut fresh = Worker::spawn(&install, &scratch.path().join("second"))?;
    fresh.ready()?;
    let again = fresh.load(&ely_lok::file_url(&output))?;
    let reopened = count(&mut fresh, &again)?;
    if reopened != parts {
        return Err(io::Error::other(format!("reopened with {reopened} parts, saved {parts}")));
    }
    let flat = match (again.is_sheet(), again.is_deck()) {
        (true, _) => "fods",
        (false, true) => "fodp",
        (false, false) => "fodt",
    };
    if !marked(&mut fresh, &again, &scratch.path().join(format!("again.{flat}")), flat)? {
        return Err(io::Error::other("the reopened copy lost the edit"));
    }
    println!(
        r#"{{"up_ms":{},"load_ms":{},"parts":{parts},"save_ms":{}}}"#,
        up.as_millis(),
        load.as_millis(),
        save.as_millis()
    );
    Ok(())
}

/// Pages, sheets or slides.
fn count(worker: &mut Worker, doc: &Doc) -> io::Result<usize> {
    Ok(match doc.is_sheet() || doc.is_deck() {
        true => doc.sheets(worker)?.list.len(),
        false => doc.pages(worker)?.len(),
    })
}

/// Whether a flat copy carries the mark.
fn marked(worker: &mut Worker, doc: &Doc, path: &Path, flat: &str) -> io::Result<bool> {
    doc.save_as(worker, &ely_lok::file_url(path), flat)?;
    Ok(std::fs::read_to_string(path)?.contains(MARK))
}
