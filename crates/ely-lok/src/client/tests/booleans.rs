//! §7: bare TRUE and FALSE survive a save.

use std::path::Path;

use super::independent::read_with;
use super::*;

#[test]
fn bare_true_and_false_stay_bare() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/table/booleans.xlsx");
    let doc = worker.load(&crate::file_url(&fixture)).expect("loads");
    let first = dir.path().join("first.xlsx");
    doc.save_as(&mut worker, &crate::file_url(&first), "xlsx").expect("saves");
    read_with("ooxml_formula.py", &first, "booleans");
    // Ely's own file reads back the same.
    let again = worker.load(&crate::file_url(&first)).expect("reloads");
    let second = dir.path().join("second.xlsx");
    again.save_as(&mut worker, &crate::file_url(&second), "xlsx").expect("saves again");
    read_with("ooxml_formula.py", &second, "booleans");
}
