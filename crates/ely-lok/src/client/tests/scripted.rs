//! §10: a click on a script URL saves without harm.

use std::path::Path;

use super::*;

#[test]
fn a_click_on_a_script_url_saves_without_harm() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("p")).expect("up");
    let deck = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/show/script-link.pptx");
    let doc = worker.load(&crate::file_url(&deck)).expect("loads");
    // The slideshow saves this way before it starts.
    let out = dir.path().join("out.fodp");
    doc.save_as(&mut worker, &crate::file_url(&out), "fodp").expect("the save survives");
    let saved = std::fs::read_to_string(&out).expect("reads");
    assert!(saved.contains("Module1.Touch"), "the click keeps its target");
}
