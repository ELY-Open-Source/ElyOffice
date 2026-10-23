use std::sync::atomic::Ordering;

use ely_mail_engine::{Error, Fetched, Key, Protocol, Security, Store};
use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{Keys, account, adding, closed_port, passed, settle, step_words, type_in, window};
use crate::{adding::Step, boxes::Open, kinds::Kind, mail::Mail};

#[test]
fn an_imap_form_says_what_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    adding(&mail, Kind::Imap, cx);
    let check = |cx: &mut gpui::VisualTestContext| {
        cx.update(|_, app| mail.update(app, |mail, cx| mail.check(cx)))
    };
    check(cx);
    assert_eq!(step_words(&mail, cx), "Type the email address.");
    type_in(&mail, cx, |f| &f.address, "ann@example.com");
    check(cx);
    assert_eq!(step_words(&mail, cx), "Type the user name.");
    type_in(&mail, cx, |f| &f.user, "ann");
    check(cx);
    assert_eq!(step_words(&mail, cx), "Type both server names.");
    type_in(&mail, cx, |f| &f.incoming, "imap.example.com");
    type_in(&mail, cx, |f| &f.outgoing, "smtp.example.com");
    type_in(&mail, cx, |f| &f.outgoing_port, "70000");
    check(cx);
    assert_eq!(
        step_words(&mail, cx),
        "70000 is not a port; use 1 to 65535."
    );
    type_in(&mail, cx, |f| &f.outgoing_port, "587");
    check(cx);
    assert_eq!(step_words(&mail, cx), "Type the password.");
}

#[test]
fn ports_follow_security_but_a_typed_one_stays() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    adding(&mail, Kind::Pop, cx);
    let ports = |cx: &mut gpui::VisualTestContext| {
        cx.update(|_, app| {
            let fields = &mail.read(app).adding.as_ref().unwrap().fields;
            (
                fields.incoming_port.read(app).text().to_owned(),
                fields.outgoing_port.read(app).text().to_owned(),
            )
        })
    };
    assert_eq!(ports(cx), ("995".into(), "587".into()));
    let secure = |cx: &mut gpui::VisualTestContext, outgoing, security| {
        cx.update(|_, app| {
            mail.update(app, |mail, cx| {
                mail.adding
                    .as_mut()
                    .unwrap()
                    .set_security(outgoing, security, cx)
            })
        });
    };
    secure(cx, false, Security::StartTls);
    assert_eq!(ports(cx), ("110".into(), "587".into()));
    type_in(&mail, cx, |f| &f.outgoing_port, "2525");
    secure(cx, true, Security::Tls);
    assert_eq!(ports(cx), ("110".into(), "2525".into()));
    cx.update(|_, app| mail.update(app, |mail, cx| mail.close_adding(cx)));
    adding(&mail, Kind::Imap, cx);
    assert_eq!(ports(cx), ("993".into(), "587".into()));
    type_in(&mail, cx, |f| &f.incoming_port, "995");
    secure(cx, true, Security::Tls);
    assert_eq!(ports(cx), ("995".into(), "465".into()));
    secure(cx, false, Security::StartTls);
    assert_eq!(ports(cx), ("995".into(), "465".into()));
}

#[test]
fn a_check_reports_each_server() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    adding(&mail, Kind::Imap, cx);
    type_in(&mail, cx, |f| &f.address, "ann@example.com");
    type_in(&mail, cx, |f| &f.user, "ann");
    type_in(&mail, cx, |f| &f.password, "pw");
    type_in(&mail, cx, |f| &f.incoming, "127.0.0.1");
    type_in(&mail, cx, |f| &f.incoming_port, &closed_port().to_string());
    type_in(&mail, cx, |f| &f.outgoing, "127.0.0.1");
    type_in(&mail, cx, |f| &f.outgoing_port, &closed_port().to_string());
    cx.update(|_, app| mail.update(app, |mail, cx| mail.check(cx)));
    assert_eq!(step_words(&mail, cx), "checking");
    settle(cx, |app| {
        matches!(
            mail.read(app).adding.as_ref().unwrap().step,
            Step::Checked { .. }
        )
    });
    cx.update(|_, app| {
        let Step::Checked {
            checked,
            account,
            kept,
        } = &mail.read(app).adding.as_ref().unwrap().step
        else {
            unreachable!()
        };
        assert!(
            matches!(checked.incoming, Err(Error::Connect(_))),
            "{:?}",
            checked.incoming
        );
        assert!(
            matches!(checked.outgoing, Err(Error::Connect(_))),
            "{:?}",
            checked.outgoing
        );
        assert_eq!(
            (account.user.as_str(), account.protocol, kept.as_str()),
            ("ann", Protocol::Imap, "pw")
        );
        let said = crate::add_view::failure(checked.incoming.as_ref().unwrap_err(), app);
        assert!(said.starts_with("cannot connect ("), "{said}");
    });
    type_in(&mail, cx, |f| &f.password, "pw2");
    assert_eq!(step_words(&mail, cx), "editing");
}

#[test]
fn a_refused_keychain_keeps_the_account_and_says_why() {
    let dir = tempfile::tempdir().unwrap();
    let keys = Keys {
        refuse: Some("locked"),
        ..Keys::default()
    };
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), keys.clone());
    adding(&mail, Kind::Imap, cx);
    passed(&mail, account("ann@example.com"), "pw", cx);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.save(cx)));
    settle(cx, |app| mail.read(app).adding.is_none());
    cx.update(|_, app| {
        let mail = mail.read(app);
        assert_eq!(mail.accounts().len(), 1);
        assert_eq!(mail.open, Open::Account(mail.accounts()[0].0));
        assert_eq!(
            mail.vault
                .get("account:ann@example.com")
                .unwrap()
                .as_deref(),
            Some("pw")
        );
    });
    assert!(keys.kept.lock().unwrap().is_empty());
    cx.update(|_, app| mail.update(app, |mail, cx| mail.ask_remove(cx)));
    settle(cx, |app| mail.read(app).removing.is_some());
    press_remove(&mail, cx);
    settle(cx, |_| keys.deletes.load(Ordering::SeqCst) == 1);
    let store = cx.update(|_, app| mail.read(app).store.clone().unwrap());
    settle(cx, |_| store.try_lock().is_ok());
    cx.run_until_parked();
    cx.update(|_, app| {
        let mail = mail.read(app);
        assert_eq!(mail.accounts().len(), 1);
        assert_eq!(
            mail.vault
                .get("account:ann@example.com")
                .unwrap()
                .as_deref(),
            Some("pw")
        );
    });
}

#[test]
fn an_account_is_kept_and_removed_with_its_secret() {
    let dir = tempfile::tempdir().unwrap();
    let keys = Keys::default();
    {
        let mut cx = TestAppContext::single();
        let (mail, cx) = window(&mut cx, dir.path(), keys.clone());
        adding(&mail, Kind::Imap, cx);
        passed(&mail, account("ann@example.com"), "pw", cx);
        cx.update(|_, app| mail.update(app, |mail, cx| mail.save(cx)));
        settle(cx, |app| mail.read(app).adding.is_none());
        adding(&mail, Kind::Imap, cx);
        passed(&mail, account("ANN@example.com"), "pw", cx);
        cx.update(|_, app| mail.update(app, |mail, cx| mail.save(cx)));
        assert_eq!(step_words(&mail, cx), "ANN@example.com is already added.");
    }
    assert_eq!(
        keys.kept
            .lock()
            .unwrap()
            .get("account:ann@example.com")
            .map(String::as_str),
        Some("pw")
    );
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), keys.clone());
    cx.update(|_, app| assert_eq!(mail.read(app).accounts()[0].1, account("ann@example.com")));
    open_first(&mail, cx);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.ask_remove(cx)));
    settle(cx, |app| mail.read(app).removing.is_some());
    cx.update(|_, app| {
        let removing = mail.read(app).removing.as_ref().unwrap();
        assert_eq!(
            (removing.address.as_str(), removing.cached.messages),
            ("ann@example.com", 0)
        );
    });
    press_remove(&mail, cx);
    settle(cx, |app| mail.read(app).accounts().is_empty());
    assert!(keys.kept.lock().unwrap().is_empty());
}

#[test]
fn a_refused_secret_removal_keeps_the_account() {
    let dir = tempfile::tempdir().unwrap();
    Store::open(dir.path())
        .unwrap()
        .add_account(&account("ann@example.com"))
        .unwrap();
    let keys = Keys {
        refuse: Some("locked"),
        ..Keys::default()
    };
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), keys.clone());
    open_first(&mail, cx);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.ask_remove(cx)));
    settle(cx, |app| mail.read(app).removing.is_some());
    press_remove(&mail, cx);
    settle(cx, |_| keys.deletes.load(Ordering::SeqCst) == 1);
    settle(cx, |app| mail.read(app).accounts().len() == 1);
    assert_eq!(
        Store::open(dir.path()).unwrap().accounts().unwrap().len(),
        1
    );
}

#[test]
fn a_failed_removal_puts_the_secret_back() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let inbox = store.mailbox(ann, "INBOX").unwrap();
    let raw: &[u8] = b"Subject: Kept

still here
";
    let key = Key::Imap {
        uidvalidity: 1,
        uid: 1,
    };
    let kept = store.put(inbox, &key, &[], Fetched::Whole(raw)).unwrap();
    drop(store);
    let keys = Keys::default();
    keys.kept
        .lock()
        .unwrap()
        .insert("account:ann@example.com".into(), "pw".into());
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), keys.clone());
    // A folder where the original would be set aside.
    let shard = std::fs::read_dir(dir.path().join("content"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let name = std::fs::read_dir(&shard)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .file_name();
    std::fs::create_dir(shard.join(format!(".{}.gone", name.to_str().unwrap()))).unwrap();
    open_first(&mail, cx);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.ask_remove(cx)));
    settle(cx, |app| mail.read(app).removing.is_some());
    press_remove(&mail, cx);
    settle(cx, |_| keys.deletes.load(Ordering::SeqCst) == 1);
    let store = cx.update(|_, app| mail.read(app).store.clone().unwrap());
    settle(cx, |_| store.try_lock().is_ok());
    cx.run_until_parked();
    assert_eq!(
        keys.kept
            .lock()
            .unwrap()
            .get("account:ann@example.com")
            .map(String::as_str),
        Some("pw")
    );
    let store = store.lock().unwrap();
    assert_eq!(store.accounts().unwrap().len(), 1);
    assert_eq!(store.original(kept).unwrap().unwrap(), raw);
}

#[test]
fn two_saves_in_flight_add_an_address_once() {
    let dir = tempfile::tempdir().unwrap();
    let keys = Keys::default();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), keys.clone());
    adding(&mail, Kind::Imap, cx);
    passed(&mail, account("ann@example.com"), "pw", cx);
    cx.update(|window, app| {
        mail.update(app, |mail, cx| {
            mail.save(cx);
            mail.close_adding(cx);
            mail.start_adding(window, cx);
        })
    });
    passed(&mail, account("ANN@example.com"), "pw2", cx);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.save(cx)));
    // Either save may reach the store first.
    settle(cx, |app| {
        let mail = mail.read(app);
        mail.accounts().len() == 1
            && !matches!(
                mail.adding.as_ref().map(|adding| &adding.step),
                Some(Step::Saving)
            )
    });
    let won = Store::open(dir.path()).unwrap().accounts().unwrap();
    assert_eq!(won.len(), 1);
    let (address, secret) = if won[0].1 == account("ann@example.com") {
        assert_eq!(step_words(&mail, cx), "ANN@example.com is already added.");
        ("ann", "pw")
    } else {
        assert_eq!(won[0].1, account("ANN@example.com"));
        cx.update(|_, app| assert!(mail.read(app).adding.is_none()));
        ("ANN", "pw2")
    };
    let kept = keys.kept.lock().unwrap();
    assert_eq!(kept.len(), 1);
    assert_eq!(
        kept.get(&format!("account:{address}@example.com"))
            .map(String::as_str),
        Some(secret)
    );
}

#[test]
fn the_keyboard_opens_another_accounts_settings() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store.add_account(&account("ann@example.com")).unwrap();
    let bo = store.add_account(&account("bo@example.com")).unwrap();
    drop(store);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    // As `mail::open` does.
    cx.update(|window, app| window.focus(&mail.read(app).focus.clone(), app));
    for _ in 0..2 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
    for _ in 0..8 {
        cx.simulate_keystrokes("tab space");
        cx.run_until_parked();
        if cx.update(|_, app| mail.read(app).open) == Open::Account(bo) {
            return;
        }
    }
    panic!("no Tab stop let Space open the second account");
}

/// Through the dialog: back to its last stop, Remove.
fn press_remove(mail: &Entity<Mail>, cx: &mut VisualTestContext) {
    for _ in 0..2 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
    cx.simulate_keystrokes("shift-tab enter");
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").unwrap(),
    });
    settle(cx, |app| mail.read(app).removing.is_none());
}

/// Opens the first account's page, as a user would.
fn open_first(mail: &Entity<Mail>, cx: &mut VisualTestContext) {
    settle(cx, |app| !mail.read(app).accounts().is_empty());
    let first = cx.update(|_, app| mail.read(app).accounts()[0].0);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(Open::Account(first), cx)));
}

/// The save lands after a second dialog opened.
fn save_then_reopen(fail: bool) {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    adding(&mail, Kind::Imap, cx);
    passed(&mail, account("ann@example.com"), "pw", cx);
    if fail {
        let store = cx.update(|_, app| mail.read(app).store.clone().unwrap());
        let added = store
            .lock()
            .unwrap()
            .add_account(&account("ann@example.com"));
        added.unwrap();
    }
    cx.update(|window, app| {
        mail.update(app, |mail, cx| {
            mail.save(cx);
            mail.close_adding(cx);
            mail.start_adding(window, cx);
            let adding = mail.adding.as_mut().unwrap();
            adding.set_kind(Kind::Pop, cx);
            let address = adding.fields.address.clone();
            address.update(cx, |field, cx| {
                field.set_text("bob@example.com".to_owned(), cx)
            });
        })
    });
    if fail {
        let store = cx.update(|_, app| mail.read(app).store.clone().unwrap());
        settle(cx, |_| store.try_lock().is_ok());
        cx.run_until_parked();
        cx.update(|_, app| assert!(mail.read(app).accounts().is_empty()));
    } else {
        settle(cx, |app| mail.read(app).accounts().len() == 1);
    }
    cx.update(|_, app| {
        let adding = mail.read(app).adding.as_ref().unwrap();
        assert_eq!(adding.kind, Kind::Pop);
        assert!(matches!(adding.step, Step::Editing));
        assert_eq!(adding.fields.address.read(app).text(), "bob@example.com");
    });
}

#[test]
fn a_late_save_leaves_a_newer_dialog_alone() {
    save_then_reopen(false);
}

#[test]
fn a_late_failed_save_leaves_a_newer_dialog_alone() {
    save_then_reopen(true);
}
