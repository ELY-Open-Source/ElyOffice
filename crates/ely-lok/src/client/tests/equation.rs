//! §7: a Word equation's minus signs stay minus signs.

use std::path::Path;

use super::independent::read_back;
use super::*;

#[test]
fn equation_minus_signs_save_as_math() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office/equation.docx");
    let mut source = fixture;
    // Word's file, then Ely's own output.
    for name in ["first.docx", "again.docx"] {
        let doc = worker.load(&crate::file_url(&source)).expect("loads");
        let out = dir.path().join(name);
        doc.save_as(&mut worker, &crate::file_url(&out), "docx").expect("saves");
        worker.close_doc(doc).expect("closes");
        read_back(&out, "equation");
        source = out;
    }
}
