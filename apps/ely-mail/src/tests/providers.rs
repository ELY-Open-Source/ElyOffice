use ely_mail_engine::{Auth, Error, Security, oauth};
use gpui::{Modifiers, TestAppContext};

use super::{Keys, adding, step_words, type_in, window};
use crate::{add_view::failure, kinds::Kind};

#[test]
fn an_icloud_account_takes_apples_servers() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    adding(&mail, Kind::ICloud, cx);
    type_in(&mail, cx, |f| &f.address, "ann@icloud.com");
    let account = cx
        .update(|_, app| mail.read(app).adding.as_ref().unwrap().account(None, app))
        .unwrap();
    assert_eq!(
        (account.incoming.host.as_str(), account.incoming.port),
        ("imap.mail.me.com", 993)
    );
    assert_eq!(
        (
            account.outgoing.host.as_str(),
            account.outgoing.port,
            account.outgoing.security
        ),
        ("smtp.mail.me.com", 587, Security::StartTls)
    );
    assert_eq!(
        (account.user.as_str(), account.auth),
        ("ann@icloud.com", Auth::Password)
    );
}

#[test]
fn a_gmail_account_takes_an_app_password_and_googles_servers() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    adding(&mail, Kind::Gmail, cx);
    let page = cx.debug_bounds("mail-add-page").expect("a button to Google's page");
    cx.simulate_click(page.center(), Modifiers::none());
    assert_eq!(
        cx.opened_url().as_deref(),
        Some("https://myaccount.google.com/apppasswords")
    );
    type_in(&mail, cx, |f| &f.address, "ann@gmail.com");
    let account = cx
        .update(|_, app| mail.read(app).adding.as_ref().unwrap().account(None, app))
        .unwrap();
    assert_eq!(
        (account.incoming.host.as_str(), account.incoming.port, account.incoming.security),
        ("imap.gmail.com", 993, Security::Tls)
    );
    assert_eq!(
        (account.outgoing.host.as_str(), account.outgoing.port, account.outgoing.security),
        ("smtp.gmail.com", 465, Security::Tls)
    );
    assert_eq!((account.user.as_str(), account.auth), ("ann@gmail.com", Auth::Password));
}

#[test]
fn a_build_without_a_client_id_says_so() {
    if oauth::microsoft().is_ok() {
        return eprintln!("SKIP: this build has a Microsoft client ID");
    }
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    adding(&mail, Kind::Microsoft, cx);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.sign_in(cx)));
    let words = step_words(&mail, cx);
    assert!(words.starts_with("Microsoft sign-in is not in this build"), "{words}");
}

#[test]
fn outlooks_held_account_says_what_to_do() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (_, cx) = window(&mut cx, dir.path(), Keys::default());
    let held = Error::Rejected("NO User is authenticated but not connected.".into());
    let other = Error::Rejected("NO LOGIN failed.".into());
    let (held, other) = cx.update(|_, app| (failure(&held, app), failure(&other, app)));
    assert!(held.starts_with("Outlook took the sign-in"), "{held}");
    assert!(held.contains("(NO User is authenticated but not connected.)"), "{held}");
    assert!(held.contains("outlook.live.com"), "{held}");
    assert_eq!(other.as_ref(), "the server refused the sign-in: NO LOGIN failed.");
}
