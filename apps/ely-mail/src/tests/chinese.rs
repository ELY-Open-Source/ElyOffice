//! The account page in Chinese draws no English (#48).

use std::sync::Arc;

use ely_gpui_component::i18n::I18n;
use ely_mail_engine::{Action, Match, Role, Store};
use ely_ui::laid::LaidOut;
use gpui::{TestAppContext, TestDispatcher, px, size};

use super::{Keys, account, settle, window};
use crate::boxes::Open;

#[test]
fn the_account_page_speaks_chinese() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    store.set_folders(ann, &[("INBOX", Some(Role::Inbox)), ("Receipts", None)]).unwrap();
    store.add_rule(ann, Match::Subject, "urgent", Action::Flag).unwrap();
    drop(store);
    let laid = Arc::new(LaidOut::default());
    let mut cx = TestAppContext::build_with_text_system(TestDispatcher::new(0), None, laid.clone());
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.update(|_, app| I18n::set_locale("zh-CN", app));
    // The start's sync failed in English; this one is read.
    cx.update(|_, app| mail.update(app, |mail, cx| mail.sync(ann, cx)));
    settle(cx, |app| mail.read(app).syncs.get(&ann).is_some_and(|each| each.failed.as_ref().is_some_and(|failed| failed.starts_with("系统"))));
    // What was drawn before the page does not count.
    laid.clear();
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(Open::Account(ann), cx)));
    cx.simulate_resize(size(px(1100.), px(2400.)));
    settle(cx, |app| {
        let mail = mail.read(app);
        mail.ruling.as_ref().is_some_and(|ruling| ruling.rules.is_some()) && mail.syncs.get(&ann).is_some_and(|each| each.folders.len() == 2)
    });
    assert!(laid.saw("ann@example.com"), "the page drew the account");
    // No secret is kept: the failure leads in Chinese.
    assert!(laid.saw("系统的凭据库出错："), "the failure has no Chinese lead");
    // The account's own words, names, the store's own words.
    let allowed = ["ann", "example", "com", "INBOX", "Receipts", "urgent", "IMAP", "TLS", "MBOX", "no", "saved", "secret", "for", "account"];
    assert_eq!(laid.english(&allowed), Default::default());
}
