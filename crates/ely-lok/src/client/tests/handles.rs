//! Closed documents leave no handles behind (PERF-08).

use std::os::windows::io::AsRawHandle as _;

use super::*;

/// Load and close cycles after the first ones fill caches.
const CYCLES: u32 = 20;

fn handles(worker: &Worker) -> u32 {
    let mut count = 0;
    let done = unsafe {
        windows_sys::Win32::System::Threading::GetProcessHandleCount(worker.child.as_raw_handle(), &mut count)
    };
    assert_ne!(done, 0, "{}", std::io::Error::last_os_error());
    count
}

#[test]
fn closed_documents_leave_no_handles() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker spawns");
    let source = dir.path().join("note.fodt");
    write(&source, FODT);
    let url = crate::file_url(&source);
    let cycle = |worker: &mut Worker| {
        let doc = worker.load(&url).expect("loads");
        worker.close_doc(doc).expect("closes");
    };
    for _ in 0..3 {
        cycle(&mut worker);
    }
    let before = handles(&worker);
    for _ in 0..CYCLES {
        cycle(&mut worker);
    }
    let after = handles(&worker);
    // A leak of one handle a cycle fails here.
    assert!(after < before + CYCLES, "{CYCLES} cycles took the worker from {before} to {after} handles");
}
