use super::*;

/// A busy engine never stalls the window opening a view.
#[test]
fn a_view_opens_while_the_engine_is_busy() {
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = crate::Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("note.fodt");
    std::fs::write(&source, crate::client::tests::FODT).expect("the fixture writes");
    let doc = worker.load(&crate::file_url(&source)).expect("the document loads");
    let laid = super::super::layout(&mut worker, &doc).expect("the layout reads");
    let feed = worker.feed();
    let shared = Arc::new(Mutex::new(worker));
    let (held, release) = (std::sync::mpsc::channel(), std::sync::mpsc::channel::<()>());
    let holder = {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || {
            let _engine = shared.lock().expect("the worker lock");
            held.0.send(()).expect("the test waits");
            let _ = release.1.recv_timeout(std::time::Duration::from_secs(5));
        })
    };
    held.1.recv().expect("the engine is held");
    let began = std::time::Instant::now();
    let cx = gpui::TestAppContext::single();
    cx.update(|app| drop(LokView::open(Arc::clone(&shared), doc, laid, feed, app)));
    let took = began.elapsed();
    release.0.send(()).expect("the holder lets go");
    holder.join().expect("the holder ends");
    assert!(took < std::time::Duration::from_secs(1), "opening waited {took:?} for the engine");
}
