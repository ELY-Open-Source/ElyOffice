//! §4.1: the window never waits for the engine's start.

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
    let (worker, stop) = ely_office::lok::test_worker(&install);
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).expect("the work dir");
    // As at launch: the engine's start holds the lock.
    let held = super::hold::hold(&worker);
    let docs = cx.update(|app| {
        ely_ui::settings::install_defaults(app);
        ely_ui::i18n::install_as("en-US", app);
        app.new(|cx| {
            let core = ely_office::Core::new(worker, stop, work, cx);
            Docs::new(core, cx)
        })
    });
    held.release("Docs::new");
    assert!(cx.update(|app| docs.read(app).core.shared.get().stop.is_some()), "the engine can still be stopped");
}
