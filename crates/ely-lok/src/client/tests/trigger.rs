//! §8: a PowerPoint video keeps its effects through a save.

use std::path::Path;

use super::independent::read_with;
use super::*;

#[test]
fn a_video_keeps_its_fade_and_click_trigger() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/show/video.pptx");
    let doc = worker.load(&crate::file_url(&fixture)).expect("loads");
    let first = dir.path().join("first.pptx");
    doc.save_as(&mut worker, &crate::file_url(&first), "pptx").expect("saves");
    read_with("ooxml_motion.py", &first, "video");
    // Ely's own file reads back the same.
    let again = worker.load(&crate::file_url(&first)).expect("reloads");
    let second = dir.path().join("second.pptx");
    again.save_as(&mut worker, &crate::file_url(&second), "pptx").expect("saves again");
    read_with("ooxml_motion.py", &second, "video");
}
