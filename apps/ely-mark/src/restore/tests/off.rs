//! The kept list's writes and the window's thread (X-12).

use gpui::{TestAppContext, VisualTestContext};

use crate::desk::tests::{open, start};
use crate::view::Mode;

#[gpui::test]
fn a_tick_writes_the_list_off_the_window_thread(cx: &mut TestAppContext) {
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let (a, b) = (files.path().join("a.md"), files.path().join("b.md"));
    std::fs::write(&a, "A\n").expect("a");
    std::fs::write(&b, "B\n").expect("b");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &a, cx);
    desk.update(cx, |desk, cx| desk.keep(cx));
    cx.run_until_parked();
    let session = data.path().join("session.json");
    let read = || std::fs::read_to_string(&session).expect("the list");
    open(&desk, &b, cx);
    let before = read();
    assert!(before.contains("a.md") && !before.contains("b.md"), "{before}");
    desk.update(cx, |desk, cx| desk.keep(cx));
    assert_eq!(read(), before, "the tick wrote on the window's thread");
    cx.run_until_parked();
    assert!(read().contains("b.md"), "the background write landed");
}

#[gpui::test]
fn a_quit_writes_the_list_at_once(cx: &mut TestAppContext) {
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let a = files.path().join("a.md");
    std::fs::write(&a, "A\n").expect("a");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &a, cx);
    // The same list queues; the quit still writes.
    desk.update(cx, |desk, cx| desk.keep(cx));
    desk.update(cx, |desk, cx| desk.leave(cx));
    let kept = std::fs::read_to_string(data.path().join("session.json")).expect("written before exit");
    assert!(kept.contains("a.md"), "{kept}");
}

#[gpui::test]
fn a_quit_lands_over_a_queued_list_it_repeats(cx: &mut TestAppContext) {
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let a = files.path().join("a.md");
    std::fs::write(&a, "A\n").expect("a");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &a, cx);
    let switch = |mode, cx: &mut VisualTestContext| {
        cx.update(|window, cx| desk.update(cx, |desk, cx| desk.switch(mode, window, cx)));
    };
    let session = data.path().join("session.json");
    let read = || std::fs::read_to_string(&session).expect("the list");
    switch(Mode::Visual, cx);
    desk.update(cx, |desk, cx| desk.keep(cx));
    cx.run_until_parked();
    assert!(read().contains("\"visual\""), "{}", read());
    switch(Mode::Source, cx);
    cx.run_until_parked();
    // Source queues; visual returns before it lands.
    desk.update(cx, |desk, cx| desk.keep(cx));
    switch(Mode::Visual, cx);
    desk.update(cx, |desk, cx| desk.leave(cx));
    cx.run_until_parked();
    let kept = read();
    assert!(kept.contains("\"visual\"") && !kept.contains("\"source\""), "{kept}");
}

#[gpui::test]
fn a_tick_never_removes_points(cx: &mut TestAppContext) {
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let a = files.path().join("a.md");
    std::fs::write(&a, "A\n").expect("a");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &a, cx);
    let stray = data.path().join("recovery").join("0-99");
    std::fs::create_dir_all(&stray).expect("a stray slot");
    desk.update(cx, |desk, cx| desk.keep(cx));
    cx.run_until_parked();
    assert!(stray.exists(), "a tick deleted from its own snapshot");
    desk.update(cx, |desk, cx| desk.leave(cx));
    assert!(!stray.exists(), "the quit sweeps against the live tabs");
}
