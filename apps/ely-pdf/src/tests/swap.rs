//! A held engine is stopped and the tabs reopen.

use ely_office::Stopping as _;
use gpui::TestAppContext;

use super::{file, settle, windowed};

#[test]
fn a_swap_stops_the_running_worker_and_reopens_the_tabs() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = file(dir.path(), "one.pdf", &[(612.0, 792.0, &["one"])]);
    let mut cx = TestAppContext::single();
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let pid = |cx: &mut gpui::VisualTestContext| cx.update(|_, app| viewer.read(app).core.worker.lock().expect("the worker").pid());
    let (old, id) = (pid(cx).expect("a worker"), cx.update(|_, app| viewer.read(app).core.tabs[0].id));
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.swap_engine(99, cx)));
    assert!(cx.update(|_, app| viewer.read(app).core.stopped.is_some()), "the swap shows");
    settle(cx, |app| viewer.read(app).core.stopped.is_none() && viewer.read(app).core.tabs.len() == 1);
    assert_ne!(cx.update(|_, app| viewer.read(app).core.tabs[0].id), id, "the tab came back");
    let fresh = pid(cx).expect("a new worker");
    assert_ne!(fresh, old, "a new worker runs");
    // The stopped worker is reaped: its pid is gone.
    #[cfg(unix)]
    assert_ne!(unsafe { libc::kill(i32::try_from(old).expect("a pid"), 0) }, 0, "the old worker ended");
}
