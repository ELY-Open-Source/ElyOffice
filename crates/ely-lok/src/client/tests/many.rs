//! §6: one engine edits several open documents.

use std::path::Path;

use super::*;

const SETTLE: std::time::Duration = std::time::Duration::from_secs(5);

/// Fails with how the worker ended, if it has.
fn alive(worker: &mut Worker, step: &str) {
    if let Some(end) = worker.ended() {
        panic!("the worker ended at {step}: {end}");
    }
}

#[test]
fn writer_dialog_keys_leave_every_document_working() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let (done, wait) = std::sync::mpsc::channel();
    let (pid_out, pid_in) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let dir = tempfile::tempdir().expect("a temp dir");
        let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
        pid_out.send(worker.child.id()).expect("the test waits");
        let office = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office");
        let first = worker.load(&crate::file_url(&office.join("redline.docx"))).expect("loads");
        let second = worker.load(&crate::file_url(&office.join("equation.docx"))).expect("loads");
        let mut blocked = Vec::new();
        // Ctrl+F12 opens Insert Table; Ctrl+F3, AutoText.
        for key in [8971, 8962] {
            let base = poll_until(&mut worker, &second, &Events::default(), SETTLE, |_| false);
            worker.post_key(&second, 0, key).expect("the key posts");
            let seen = poll_until(&mut worker, &second, &base, SETTLE, |seen| !seen.blocked.is_empty());
            blocked.extend(seen.blocked.clone());
            let ended = poll_until(&mut worker, &second, &seen, SETTLE, |e| e.dismissed > base.dismissed);
            assert!(ended.dismissed > base.dismissed, "the dialog of key {key} never ended");
            for doc in [&first, &second] {
                worker.post_key(doc, 'z' as u32, 537).expect("typing still works");
            }
            alive(&mut worker, &format!("after key {key}"));
        }
        for (n, doc) in [&first, &second].into_iter().enumerate() {
            let copy = dir.path().join(format!("after-{n}.docx"));
            doc.save_as(&mut worker, &crate::file_url(&copy), "docx").expect("saves after the dialogs");
        }
        alive(&mut worker, "saving after the dialogs");
        done.send(blocked).expect("the test waits");
    });
    let pid = pid_in.recv().expect("the worker starts");
    let outcome = wait.recv_timeout(std::time::Duration::from_secs(60));
    if outcome.is_err() {
        kill(pid);
    }
    let blocked = outcome.expect("the worker died or hung after a Writer dialog");
    assert_eq!(blocked, ["Insert Table", "AutoText"], "each dialog is reported");
}
