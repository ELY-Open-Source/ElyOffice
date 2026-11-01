//! §4.1: the window never waits for the engine's start.

use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use gpui::{AppContext as _, TestAppContext};

use crate::docs::Docs;

#[test]
fn docs_build_while_the_engine_holds_its_lock() {
    let Some(install) = ely_lok::test_install() else {
        return eprintln!("SKIP: no LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let (worker, stop) = ely_office::lok::test_worker(&install, &dir.path().join("profile"));
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).expect("the work dir");
    // As at launch: the engine's start holds the lock.
    let (held, holding) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    let busy = Arc::clone(&worker);
    let starting = std::thread::spawn(move || {
        let _guard = busy.lock().expect("the worker lock");
        held.send(()).expect("the test waits");
        let _ = released.recv_timeout(Duration::from_secs(5));
    });
    holding.recv().expect("the lock is held");
    let begun = Instant::now();
    let docs = cx.update(|app| {
        ely_ui::settings::install_defaults(app);
        ely_ui::i18n::install_as("en-US", app);
        app.new(|cx| Docs::new(worker, stop, work, cx))
    });
    let took = begun.elapsed();
    release.send(()).expect("the holder waits");
    starting.join().expect("the holder ends");
    assert!(took < Duration::from_secs(1), "Docs::new waited {took:?} on the engine");
    assert!(cx.update(|app| docs.read(app).core.stop.is_some()), "the engine can still be stopped");
}
