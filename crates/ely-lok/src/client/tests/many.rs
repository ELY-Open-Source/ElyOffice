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
fn three_documents_edit_save_and_close() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let office = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office");
    let docs: Vec<Doc> = ["redline.docx", "toc.docx", "equation.docx"]
        .iter()
        .map(|name| worker.load(&crate::file_url(&office.join(name))).unwrap_or_else(|e| panic!("loading {name}: {e}")))
        .collect();
    let mut docs = docs.into_iter();
    let (first, second, third) = (docs.next().expect("one"), docs.next().expect("two"), docs.next().expect("three"));
    for round in 0..20 {
        for c in "Ely edit ".chars() {
            worker.post_key(&first, c as u32, 0).expect("a key posts");
        }
        alive(&mut worker, &format!("typing, round {round}"));
        for (n, doc) in [&first, &second, &third].into_iter().enumerate() {
            let copy = dir.path().join(format!("recovery-{n}.docx"));
            doc.save_as(&mut worker, &crate::file_url(&copy), "docx").expect("a recovery copy");
        }
        alive(&mut worker, &format!("recovery copies, round {round}"));
    }
    worker.close_doc(third).expect("closes the third");
    alive(&mut worker, "closing the third");
    for c in "after ".chars() {
        worker.post_key(&first, c as u32, 0).expect("a key posts after a close");
    }
    first.save_as(&mut worker, &crate::file_url(&dir.path().join("last.docx")), "docx").expect("saves after a close");
    second.save_as(&mut worker, &crate::file_url(&dir.path().join("other.docx")), "docx").expect("saves the other");
    alive(&mut worker, "after the close");
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
            let seen = poll_until(&mut worker, &second, &base, HANG, |seen| !seen.blocked.is_empty());
            blocked.extend(seen.blocked.clone());
            let ended = poll_until(&mut worker, &second, &seen, HANG, |e| e.dismissed > base.dismissed);
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
    let titles: Vec<&str> = blocked.iter().map(|(title, _): &(String, String)| title.as_str()).collect();
    assert_eq!(titles, ["Insert Table", "AutoText"], "each dialog is reported");
}
