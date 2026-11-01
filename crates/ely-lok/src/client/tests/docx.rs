//! §7: Word's structures come back as Word reads them.

use std::path::Path;

use super::independent::read_back;
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
        read_back(&out, case);
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
    let wait = std::time::Duration::from_secs(3);
    let loaded = poll_until(&mut worker, &doc, &Events::default(), wait, modified);
    assert!(!modified(&loaded), "the load marked it modified");
    doc.paint_tile(&mut worker, (0, 0, 3840, 3840), (256, 256)).expect("the page paints");
    let painted = poll_until(&mut worker, &doc, &loaded, wait, modified);
    assert!(!modified(&painted), "the first paint marked it modified");
}
