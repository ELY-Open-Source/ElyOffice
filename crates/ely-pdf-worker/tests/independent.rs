//! Ely's edits read back by another reader, PDFKit (QA-13).
#![cfg(target_os = "macos")]

use std::path::Path;

use ely_pdf_engine::{FieldKind, NewField, Worker, Writing, Wrote, sample::{Extras, pdf_with}};
use objc2::AllocAnyThread as _;
use objc2_foundation::{NSString, NSURL};
use objc2_pdf_kit::PDFDocument;

fn worker() -> Worker {
    Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker")
}

/// Each page's text and widget values, as PDFKit reads them.
fn read_back(path: &Path) -> Vec<(String, Vec<String>)> {
    let url = NSURL::fileURLWithPath(&NSString::from_str(path.to_str().expect("UTF-8")));
    let document = unsafe { PDFDocument::initWithURL(PDFDocument::alloc(), &url) }.expect("PDFKit opens it");
    (0..unsafe { document.pageCount() })
        .map(|at| {
            let page = unsafe { document.pageAtIndex(at) }.expect("a page");
            let text = unsafe { page.string() }.map(|s| s.to_string()).unwrap_or_default();
            let values = unsafe { page.annotations() }.to_vec().into_iter().filter_map(|a| unsafe { a.widgetStringValue() }.map(|s| s.to_string())).collect();
            (text, values)
        })
        .collect()
}

#[test]
fn text_a_field_and_page_order_read_back_elsewhere() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("doc.pdf");
    std::fs::write(&path, pdf_with(&[(612.0, 792.0, &["First page"]), (612.0, 792.0, &["Second page"])], &[], &Extras::default())).expect("writes");
    let mut worker = worker();
    let mut doc = worker.open(&path).expect("opens").doc;
    let block = worker.text_blocks(doc, 0).expect("blocks").into_iter().position(|b| b.text.contains("First page")).expect("the block");
    let writing = Writing {
        block: Some(block),
        expected: "First page".into(),
        text: "Changed words".into(),
        font: "helvetica".into(),
        size: 12.0,
        rgb: 0,
        bx: [72.0, 60.0, 400.0],
    };
    assert!(matches!(worker.put_text(doc, 0, &writing, 0.0).expect("writes"), Wrote::Done(_)));
    doc = worker.now(doc).expect("the state");
    let field = NewField { kind: FieldKind::Text, name: "name".into(), rect: [72.0, 200.0, 272.0, 220.0], options: Vec::new() };
    worker.add_field(doc, 0, &field).expect("adds a field");
    doc = worker.now(doc).expect("the state");
    let id = worker.marks(doc, 0).expect("marks").fields[0].id;
    worker.fill(doc, 0, id, "Ada").expect("fills");
    doc = worker.now(doc).expect("the state");
    worker.move_pages(doc, &[1], 0).expect("moves");
    doc = worker.now(doc).expect("the state");
    let out = dir.path().join("edited.pdf");
    worker.save(doc, &out).expect("saves");
    let pages = read_back(&out);
    assert_eq!(pages.len(), 2);
    assert!(pages[0].0.contains("Second page"), "{pages:?}");
    assert!(pages[1].0.contains("Changed words") && !pages[1].0.contains("First page"), "{pages:?}");
    assert_eq!(pages[1].1, vec!["Ada".to_string()]);
}

#[test]
fn text_in_a_subset_font_reads_back_elsewhere() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office/t.pdf");
    let mut worker = worker();
    let mut doc = worker.open(&path).expect("opens").doc;
    let blocks = worker.text_blocks(doc, 0).expect("blocks");
    let (block, read) = blocks.iter().enumerate().find(|(_, b)| b.text.contains("BUNDLE")).expect("the block");
    let writing = Writing {
        block: Some(block),
        expected: read.text.clone(),
        text: "TEXT BUNDLE".into(),
        font: "original".into(),
        size: read.size,
        rgb: read.rgb,
        bx: [read.rect[0], read.rect[1], (read.rect[2] - read.rect[0]) * 2.0],
    };
    assert!(matches!(worker.put_text(doc, 0, &writing, 0.0).expect("writes"), Wrote::Done(_)));
    doc = worker.now(doc).expect("the state");
    let out = dir.path().join("edited.pdf");
    worker.save(doc, &out).expect("saves");
    let pages = read_back(&out);
    assert!(pages[0].0.contains("TEXT BUNDLE"), "{pages:?}");
}
