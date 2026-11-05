//! LOK-11: sources read for links stay free to replace.

use std::path::Path;

use super::analysis::{cells, skip};
use super::linked::{includes, rates, update};
use super::ole_links::worksheet;
use super::*;

fn fresh() -> (tempfile::TempDir, Worker) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let install = install().expect("the pinned LOK build");
    let worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    (dir, worker)
}

/// Renames a copy over `source`, as editors save.
fn replace(source: &Path, when: &str) {
    let copy = source.with_extension("copy");
    std::fs::copy(source, &copy).expect("copies the source");
    if let Err(error) = std::fs::rename(&copy, source) {
        panic!("{when}: the source is held: {error}");
    }
}

#[test]
fn a_source_a_book_read_is_free_to_replace() {
    if skip() {
        return;
    }
    let (dir, mut worker) = fresh();
    let source = dir.path().join("source.xlsx");
    worksheet(dir.path(), &mut worker, &source, 1);
    let flat = dir.path().join("reader.fods");
    rates(&flat, 0);
    let reader = worker.load(&crate::file_url(&flat)).expect("loads");
    let formula = format!("='{}/[source.xlsx]S1'!A1", dir.path().display());
    worker.run(&reader, "cell.goto", &["H1"]).expect("H1");
    worker.run(&reader, "cell.enter", &[&formula]).expect("the link");
    assert_eq!(cells(&mut worker, &reader, "H1"), "1");
    replace(&source, "after a formula read it");
    let saved = dir.path().join("reader.xlsx");
    reader.save_as(&mut worker, &crate::file_url(&saved), "xlsx").expect("saves");
    worker.close_doc(reader).expect("closes");
    worksheet(dir.path(), &mut worker, &source, 2);
    let book = worker.load(&crate::file_url(&saved)).expect("reloads");
    assert_eq!(update(&mut worker, &book), Vec::<String>::new());
    assert_eq!(cells(&mut worker, &book, "H1"), "2");
    replace(&source, "after an update read it");
    // Saved over in place, as ElyTable saves it.
    worksheet(dir.path(), &mut worker, &source, 3);
    assert_eq!(update(&mut worker, &book), Vec::<String>::new());
    assert_eq!(cells(&mut worker, &book, "H1"), "3", "the next update reads the new file");
}

#[test]
fn a_source_a_document_read_is_free_to_replace() {
    if skip() {
        return;
    }
    let (dir, mut worker) = fresh();
    let (main, include) = includes(&dir, &mut worker, "<text:p>One</text:p>");
    let doc = worker.load(&crate::file_url(&main)).expect("loads");
    assert_eq!(update(&mut worker, &doc), Vec::<String>::new());
    replace(&include, "after an update read the INCLUDETEXT source");
    worker.close_doc(doc).expect("closes");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office/ole-link.docx");
    let (main, source) = (dir.path().join("ole.docx"), dir.path().join("source.xlsx"));
    std::fs::copy(fixture, &main).expect("copies");
    worksheet(dir.path(), &mut worker, &source, 4);
    let doc = worker.load(&crate::file_url(&main)).expect("loads");
    assert_eq!(update(&mut worker, &doc), Vec::<String>::new());
    replace(&source, "after an update read the linked object");
}
