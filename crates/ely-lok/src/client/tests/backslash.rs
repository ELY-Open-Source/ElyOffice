//! #49: odd and broken packages answer, never hang.

use std::time::Duration;

use super::*;

/// Loads fixture `name`; past 30 s it fails.
fn loaded(install: &std::path::Path, name: &str) -> std::io::Result<()> {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(install, &dir.path().join("profile")).expect("the worker");
    worker.ready().expect("the worker passes its checks");
    let pid = worker.child.id();
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office").join(name);
    let (done, wait) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let loaded = worker.load(&crate::file_url(&file)).map(|_| ());
        done.send((loaded, worker)).expect("the test waits");
    });
    let Ok((loaded, _worker)) = wait.recv_timeout(Duration::from_secs(60)) else {
        kill(pid);
        panic!("{name} still loading after 60 s");
    };
    loaded
}

#[test]
fn backslash_folder_entries_open() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    for name in ["backslash.docx", "backslash.xlsx"] {
        loaded(&install, name).unwrap_or_else(|error| panic!("{name}: {error}"));
    }
}

#[test]
fn a_broken_package_fails_instead_of_asking() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let error = loaded(&install, "broken.docx").expect_err("a broken package opened");
    assert!(error.get_ref().is_some_and(|inner| inner.is::<crate::Damaged>()), "named as damaged: {error}");
}

#[test]
fn a_malformed_part_fails_instead_of_asking() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let error = loaded(&install, "malformed.docx").expect_err("a malformed body opened");
    assert!(error.get_ref().is_some_and(|inner| inner.is::<crate::Damaged>()), "named as damaged: {error}");
}
