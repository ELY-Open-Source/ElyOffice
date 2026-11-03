//! EPUB books open read-only in the real worker (#46, PDF-01).

use std::path::{Path, PathBuf};

use ely_pdf_engine::{Doc, Worker, proto::may};

fn worker() -> Worker {
    Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker")
}

fn sample() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/epub/sample.epub")
}

fn opened() -> (Worker, Doc) {
    let mut worker = worker();
    let opened = worker.open(&sample()).expect("opens");
    assert!(!opened.locked, "a book has no password");
    (worker, opened.doc)
}

#[test]
fn a_book_reads_draws_and_searches_as_pages() {
    let (mut worker, doc) = opened();
    // Drawn first, the missing picture warns inside the draw.
    let tile = worker.render(doc, 0, (1.0, 0), [0, 0, 200, 200]).expect("draws");
    assert!(tile.bgra.as_chunks::<4>().0.iter().any(|p| p[0] < 128), "it draws text");
    assert!(!tile.damaged, "a book's warning is no damage");
    let pages = worker.pages(doc).expect("counts");
    assert!(pages >= 3, "two chapters, the second long: {pages}");
    let bounds = worker.bounds(doc, 0, 1).expect("measures")[0];
    assert!(bounds[2] > 100.0 && bounds[3] > 100.0, "{bounds:?}");
    let text: String = worker.text(doc, 0).expect("reads").iter().map(|c| c.c).collect();
    assert!(text.contains("你好") && text.contains("Ely reads books"), "{text}");
    assert_eq!(worker.search(doc, 0, "离线阅读").expect("searches").len(), 1);
    let marks: Vec<_> = worker.outline(doc).expect("outline").into_iter().map(|b| (b.title, b.page)).collect();
    assert_eq!(marks[0], ("First Chapter".to_string(), Some(0)));
    assert_eq!(marks[1].0, "Second Chapter");
    let second = marks[1].1.expect("the second chapter's page");
    let text: String = worker.text(doc, second).expect("reads").iter().map(|c| c.c).collect();
    assert!(second >= 1 && text.contains("Second Chapter"), "page {second}: {text}");
    assert_eq!(worker.permissions(doc).expect("permissions"), may::COPY);
    assert_eq!(worker.signed(doc).expect("signed"), 0);
    assert_eq!(worker.repair_report(doc).expect("a report"), None);
    let marks = worker.marks(doc, 0).expect("marks");
    assert!(marks.annots.is_empty() && marks.fields.is_empty());
}

#[test]
fn a_book_refuses_every_change_and_stays_the_same() {
    let (mut worker, doc) = opened();
    let before = worker.history(doc).expect("a mark");
    let dir = tempfile::tempdir().expect("a temp dir");
    let refusals = [
        worker.delete_pages(doc, &[0]).map(|_| ()),
        worker.rotate_pages(doc, &[0], 90).map(|_| ()),
        worker.save(doc, &dir.path().join("book.pdf")),
        worker.text_blocks(doc, 0).map(|_| ()),
    ];
    for refused in refusals {
        let error = refused.expect_err("refused");
        assert_eq!(ely_pdf_engine::refused(&error), Some("pdf"), "{error}");
    }
    assert_eq!(worker.history(doc).expect("a mark"), before);
    assert!(!dir.path().join("book.pdf").exists(), "nothing written");
    assert!(worker.pages(doc).expect("counts") >= 3);
}

#[test]
fn other_kinds_mupdf_reads_stay_out() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("note.txt");
    std::fs::write(&path, "plain text").expect("writes");
    let said = worker().open(&path).expect_err("refused");
    assert_eq!(said.to_string(), "pdf.error.not_pdf", "worded across the pipe");
}
