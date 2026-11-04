//! §7: Word's structures come back as Word reads them.

use std::path::Path;

use super::independent::read_with;
use super::*;

/// Saves Word's `fixture`, then Ely's output; reads both as `case`.
fn saved_twice(fixture: &str, case: &str) {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let mut source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office").join(fixture);
    for name in ["first.docx", "again.docx"] {
        let doc = worker.load(&crate::file_url(&source)).expect("loads");
        let out = dir.path().join(name);
        doc.save_as(&mut worker, &crate::file_url(&out), "docx").expect("saves");
        worker.close_doc(doc).expect("closes");
        read_with("ooxml_word.py", &out, case);
        source = out;
    }
}

#[test]
fn equation_minus_signs_save_as_math() {
    saved_twice("equation.docx", "equation");
}

#[test]
fn titled_toc_field_ends_in_its_own_paragraph() {
    saved_twice("toc.docx", "toc-end");
}

#[test]
fn text_after_a_toc_field_end_stays_outside_the_field() {
    saved_twice("toc-tail.docx", "toc-tail");
    saved_twice("toc-tail-untitled.docx", "toc-tail");
    saved_twice("toc-tail-entry.docx", "toc-tail");
    saved_twice("toc-tail-entry-untitled.docx", "toc-tail");
}

#[test]
fn a_text_box_after_a_toc_field_end_stays_outside_the_field() {
    saved_twice("toc-shape.docx", "toc-shape");
    saved_twice("toc-shape-untitled.docx", "toc-shape");
}

#[test]
fn an_inline_content_control_keeps_the_toc_end_alone() {
    saved_twice("toc-inline.docx", "toc-end");
}

/// Whether the kernel has reported the document modified.
fn modified(seen: &Events) -> bool {
    seen.states.iter().any(|(name, state)| name == ".uno:ModifiedStatus" && state == "true")
}

#[test]
fn a_docx_with_page_references_opens_unmodified() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office/redline.docx");
    let doc = worker.load(&crate::file_url(&source)).expect("loads");
    // A late mark fails too, however loaded the machine.
    let loaded = poll_until(&mut worker, &doc, &Events::default(), HANG, modified);
    assert!(!modified(&loaded), "the load marked it modified");
    doc.paint_tile(&mut worker, (0, 0, 3840, 3840), (256, 256)).expect("the page paints");
    let painted = poll_until(&mut worker, &doc, &loaded, HANG, modified);
    assert!(!modified(&painted), "the first paint marked it modified");
}
