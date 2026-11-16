//! A moved PDF tab keeps its page (W.1c).

use std::{cell::RefCell, rc::Rc, sync::{Arc, Mutex}};

use ely_office::{Core, Opening as _};
use gpui::{AnyWindowHandle, Entity, TestAppContext};

use super::file;
use crate::{engine::{Mupdf, Pdf}, viewer::Viewer};
use ely_pdf_engine::Worker;

const PAGE: (f32, f32, &[&str]) = (612.0, 792.0, &["a page of text"]);

/// Draws the windows until `done`.
#[track_caller]
fn wait(cx: &mut TestAppContext, done: impl Fn(&gpui::App) -> bool) {
    // The worker answers in real time, not test time.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while std::time::Instant::now() < deadline {
        cx.run_until_parked();
        if cx.update(|app| done(app)) {
            return;
        }
        cx.executor().advance_clock(std::time::Duration::from_millis(20));
    }
    panic!("the windows never showed it");
}

#[test]
fn a_moved_pdf_keeps_its_page_and_draws_anew() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = file(dir.path(), "four.pdf", &[PAGE, PAGE, PAGE, PAGE]);
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).expect("a work dir");
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        ely_ui::settings::install_defaults(cx);
        ely_ui::install_theme(cx);
        ely_ui::i18n::install_as("en-US", cx);
        crate::keys::bind(cx);
        ely_ui::windows::allow_for_tests(ely_platform::AppId::Pdf, crate::viewer::more, cx);
    });
    let held = Rc::new(RefCell::new(None));
    let kept = Rc::clone(&held);
    let home = cx
        .update(|cx| {
            cx.open_window(Default::default(), move |window, cx| {
                let mupdf = Mupdf::with(Worker::spawn().expect("the worker"));
                let stop = <Pdf as ely_office::Engine>::stopper(&mupdf);
                let viewer = crate::viewer::viewer(move |cx| Core::new(Arc::new(Mutex::new(mupdf)), stop, work, cx), cx);
                kept.replace(Some(viewer.clone()));
                crate::viewer::root(&viewer, window, cx)
            })
        })
        .expect("a window");
    let home: AnyWindowHandle = home.into();
    cx.update(|cx| ely_ui::tray::testing::opened(ely_platform::AppId::Pdf, home, cx));
    let viewer: Entity<Viewer> = held.take().expect("the viewer");
    cx.update(|app| viewer.update(app, |viewer, cx| viewer.open_file(path.clone(), cx)));
    wait(&mut cx, |app| viewer.read(app).core.tabs.len() == 1);
    let view = cx.update(|app| viewer.read(app).active_view()).expect("a tab");
    cx.update(|app| view.update(app, |view, cx| view.turn(2, cx)));
    wait(&mut cx, |app| view.read(app).current == 2);
    let id = cx.update(|app| viewer.read(app).core.active()).expect("a tab");
    cx.update(|app| viewer.read(app).side.clone()).update(&mut cx, |side, _| side.pane = crate::side::Pane::Notes);
    cx.update(|app| ely_office::moving::to_new_window(viewer.clone(), id, app));
    wait(&mut cx, |app| !app.windows().contains(&home));
    let other = cx.update(|app| ely_office::shared::Handle::<Viewer>::joined(app).ok()?.front(None, app)).expect("the new viewer");
    wait(&mut cx, |app| other.read(app).active_view().is_some_and(|shown| !shown.read(app).tiles.is_empty()));
    let moved = cx.update(|app| other.read(app).active_view()).expect("the moved tab");
    assert_eq!(moved, view, "the same view, not reopened");
    assert_eq!(cx.update(|app| moved.read(app).current), 2, "its page came along");
    let pane = cx.update(|app| other.read(app).side.read(app).pane);
    assert!(pane == crate::side::Pane::Notes, "its side pane came along");
}

#[test]
fn a_text_write_in_flight_keeps_its_tab() {
    use ely_office::moving::Moving as _;
    use super::editing::{blocks, click, editing, type_in};
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = editing(&mut cx, dir.path());
    click(cx, &view, (80.0, 66.0));
    type_in(cx, &viewer, "Hi world\nSecond line");
    let stays = cx.update(|_, app| {
        viewer.update(app, |viewer, cx| {
            viewer.write_text(false, cx);
            let id = viewer.core.active().expect("a tab");
            viewer.move_out(id, cx).err()
        })
    });
    assert_eq!(stays, Some("office.move.busy"), "the write lands here first");
    let now = blocks(cx, &view);
    assert!(now.iter().any(|b| b.starts_with("Hi world")), "{now:?}");
}
