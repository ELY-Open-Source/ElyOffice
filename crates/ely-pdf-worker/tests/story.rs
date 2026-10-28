//! An HTML folder set as a PDF (MARK-11).

use std::path::Path;

use ely_pdf_engine::Worker;

fn worker() -> Worker {
    Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker")
}

/// A 1×1 red PNG.
const DOT: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00,
    0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08,
    0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xdd, 0x8d, 0xb0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

fn page(dir: &Path, body: &str) {
    let html = format!("<!doctype html><html><head><meta charset=\"utf-8\"></head><body>{body}</body></html>");
    std::fs::write(dir.join("page.html"), html).expect("writes");
}

/// The PDF's pages and its first page's text.
fn read(worker: &mut Worker, pdf: &Path) -> (usize, String) {
    let doc = worker.open(pdf).expect("opens").doc;
    let pages = worker.pages(doc).expect("pages");
    let text: String = worker.text(doc, 0).expect("text").iter().map(|ch| ch.c).collect();
    worker.close(doc).expect("closes");
    (pages, text)
}

#[test]
fn a_page_with_pictures_and_cjk_sets_as_a_pdf() {
    let dir = tempfile::tempdir().expect("a temp dir");
    std::fs::create_dir(dir.path().join("files")).expect("makes");
    std::fs::write(dir.path().join("files/dot.png"), DOT).expect("writes");
    let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"20\" height=\"10\"><path d=\"M0 0H20V10Z\"/></svg>";
    page(dir.path(), &format!("<h1>Title</h1><p>中文段落 {svg}</p><p><img src=\"files/dot.png\" alt=\"dot\"></p>"));
    let pdf = dir.path().join("out.pdf");
    let mut worker = worker();
    worker.html_pdf(dir.path(), "page.html", &pdf).expect("sets");
    let (pages, text) = read(&mut worker, &pdf);
    assert_eq!(pages, 1);
    assert!(text.contains("Title") && text.contains("中文段落"), "{text:?}");
}

#[test]
fn a_long_page_runs_on() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let body: String = (0..300).map(|n| format!("<p>Paragraph {n}.</p>")).collect();
    page(dir.path(), &body);
    let pdf = dir.path().join("out.pdf");
    let mut worker = worker();
    worker.html_pdf(dir.path(), "page.html", &pdf).expect("sets");
    assert!(read(&mut worker, &pdf).0 > 3);
}

#[test]
fn a_missing_picture_fails_and_writes_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    page(dir.path(), "<p><img src=\"files/gone.png\" alt=\"gone\"></p>");
    let pdf = dir.path().join("out.pdf");
    let error = worker().html_pdf(dir.path(), "page.html", &pdf).expect_err("refused");
    assert!(error.to_string().contains("did not set whole"), "{error}");
    assert!(!pdf.exists());
}

#[test]
fn a_picture_taller_than_a_page_still_sets() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"3000\"><path d=\"M0 0H100V3000Z\"/></svg>";
    page(dir.path(), &format!("<p>Before</p>{svg}<p>After</p>"));
    let pdf = dir.path().join("out.pdf");
    let mut worker = worker();
    worker.html_pdf(dir.path(), "page.html", &pdf).expect("sets");
    assert!(read(&mut worker, &pdf).0 >= 1);
}
