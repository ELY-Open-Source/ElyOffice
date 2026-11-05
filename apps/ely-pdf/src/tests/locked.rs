//! PDF-11: a locked file asks, refuses a wrong password, opens.

use ely_office::{Dialog, Opening as _};
use gpui::TestAppContext;

use super::{frames, settle, windowed};

#[test]
fn a_locked_file_asks_refuses_a_wrong_password_then_opens() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = dir.path().join("locked.pdf");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/pdf/aes128.pdf");
    std::fs::copy(fixture, &path).expect("copies");
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[], true);
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.open_file(path.clone(), cx)));
    let asked = |app: &gpui::App| match &viewer.read(app).core.dialog {
        Some(Dialog::Password { path: asked, wrong, .. }) => Some((asked.clone(), *wrong)),
        _ => None,
    };
    settle(cx, |app| asked(app).is_some());
    assert_eq!(cx.update(|_, app| asked(app)), Some((path.clone(), false)), "it asks first");
    frames(cx);
    cx.simulate_input("nope");
    cx.simulate_keystrokes("enter");
    settle(cx, |app| asked(app).is_some_and(|(_, wrong)| wrong));
    assert!(cx.update(|_, app| viewer.read(app).core.tabs.is_empty()), "a wrong password opens nothing");
    frames(cx);
    cx.simulate_input("user");
    cx.simulate_keystrokes("enter");
    settle(cx, |app| viewer.read(app).core.tabs.len() == 1);
    assert!(cx.update(|_, app| asked(app)).is_none(), "the prompt closes");
}
