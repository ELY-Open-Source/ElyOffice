//! Waits that step the test scheduler.

use gpui::TestAppContext;

/// Steps the scheduler until `done` or a minute passes.
#[track_caller]
pub(crate) fn settle(cx: &TestAppContext, done: impl Fn(&gpui::App) -> bool) {
    assert!(settled(cx, done), "the state never settled");
}

/// Whether `done` came to hold within a minute.
pub(crate) fn settled(cx: &TestAppContext, done: impl Fn(&gpui::App) -> bool) -> bool {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while std::time::Instant::now() < deadline {
        if !cx.executor().tick() {
            cx.executor()
                .advance_clock(std::time::Duration::from_millis(20));
        }
        if cx.update(|app| done(app)) {
            return true;
        }
    }
    false
}
