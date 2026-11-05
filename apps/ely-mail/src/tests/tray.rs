//! Quitting from the tray reopens the window to ask.

use ely_platform::AppId;
use gpui::TestAppContext;

use super::{Keys, settle, window, writing::{NOTE, seeded}};

/// Unshipped: its saved window place is no one's.
const APP: AppId = AppId::Agent;
const PNG: &[u8] = include_bytes!("../../../../assets/icons/ely-mail.png");

// AppKit's status item needs the main thread.
#[cfg(not(target_os = "macos"))]
#[test]
fn a_quit_from_the_tray_opens_the_window_to_ask() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    let kept = mail.clone();
    // A reopened main window needs the theme.
    cx.update(|_, app| ely_ui::install_theme(app));
    let tray = cx.update(|_, app| {
        ely_ui::tray::keep(APP, PNG, "envelope", move |window, cx| crate::mail::attach(APP, &kept, true, window, cx), app)
    });
    if let Err(error) = tray {
        return eprintln!("SKIP: no tray host: {error:#}");
    }
    let before = cx.update(|_, app| app.windows().len());
    cx.update(|_, app| mail.update(app, |mail, _| mail.pending = 1));
    let leaves = cx.update(|_, app| mail.update(app, |mail, cx| mail.may_quit(cx)));
    assert!(!leaves, "it asks");
    settle(cx, |app| app.windows().len() == before + 1);
    cx.update(|_, app| {
        let mail = mail.read(app);
        assert!(mail.quitting, "the question shows");
        assert!(mail.root.upgrade().is_some(), "in the reopened window");
    });
}
