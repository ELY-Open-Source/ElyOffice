use ely_mail_engine::{Protocol, Role, Store};
use gpui::TestAppContext;

use super::{Keys, account, adding, closed_port, settle, type_in, window, window_with};
use crate::kinds::Kind;

#[test]
fn every_account_syncs_on_open_and_says_why_it_failed() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let mut ann = account("ann@example.com");
    ann.incoming.port = closed_port();
    let ann = store.add_account(&ann).unwrap();
    let mut pop = ely_mail_engine::Account {
        protocol: Protocol::Pop { keep: true },
        ..account("bo@example.com")
    };
    pop.incoming.port = closed_port();
    let bo = store.add_account(&pop).unwrap();
    drop(store);
    let keys = Keys::default();
    keys.kept
        .lock()
        .unwrap()
        .extend(["ann", "bo"].map(|name| (format!("account:{name}@example.com"), "pw".into())));
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), keys);
    settle(cx, |app| {
        [ann, bo].iter().all(|id| {
            mail.read(app)
                .syncs
                .get(id)
                .is_some_and(|each| each.task.is_none() && each.synced.is_some())
        })
    });
    cx.update(|_, app| {
        for id in [ann, bo] {
            let syncing = &mail.read(app).syncs[&id];
            let failed = syncing.failed.as_deref().unwrap();
            assert!(failed.starts_with("cannot connect"), "{failed}");
            assert_eq!(syncing.synced.unwrap().at, None);
        }
    });
}

#[test]
fn an_account_without_its_secret_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let ann = Store::open(dir.path())
        .unwrap()
        .add_account(&account("ann@example.com"))
        .unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    settle(cx, |app| {
        mail.read(app)
            .syncs
            .get(&ann)
            .is_some_and(|each| each.failed.is_some())
    });
    cx.update(|_, app| {
        let failed = mail.read(app).syncs[&ann].failed.clone().unwrap();
        assert_eq!(
            failed,
            "the system's credential store failed: no saved secret for account:ann@example.com"
        );
    });
}

#[test]
fn a_missing_tls_is_the_sync_failure() {
    let dir = tempfile::tempdir().unwrap();
    let ann = Store::open(dir.path())
        .unwrap()
        .add_account(&account("ann@example.com"))
        .unwrap();
    let mut cx = TestAppContext::single();
    let tls = Err("platform trust store unavailable".to_owned());
    let (mail, cx) = window_with(&mut cx, dir.path(), Keys::default(), tls);
    settle(cx, |app| {
        mail.read(app)
            .syncs
            .get(&ann)
            .is_some_and(|each| each.synced.is_some())
    });
    cx.update(|_, app| {
        let syncing = &mail.read(app).syncs[&ann];
        assert_eq!(
            syncing.failed.as_deref(),
            Some("Secure connections are not available: platform trust store unavailable")
        );
        assert!(syncing.task.is_none());
    });
}

#[test]
fn a_pop_account_keeps_a_server_copy_unless_told() {
    let dir = tempfile::tempdir().unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    adding(&mail, Kind::Pop, cx);
    type_in(&mail, cx, |f| &f.address, "bo@example.com");
    type_in(&mail, cx, |f| &f.user, "bo");
    type_in(&mail, cx, |f| &f.incoming, "pop.example.com");
    type_in(&mail, cx, |f| &f.outgoing, "smtp.example.com");
    let protocol = |cx: &mut gpui::VisualTestContext| {
        cx.update(|_, app| {
            let adding = mail.read(app).adding.as_ref().unwrap();
            adding.account(None, app).unwrap().protocol
        })
    };
    assert_eq!(protocol(cx), Protocol::Pop { keep: true });
    cx.update(|_, app| mail.update(app, |mail, _| mail.adding.as_mut().unwrap().set_keep(false)));
    assert_eq!(protocol(cx), Protocol::Pop { keep: false });
}

#[test]
fn folders_show_at_a_step_before_the_sync_ends() {
    let dir = tempfile::tempdir().unwrap();
    let ann = Store::open(dir.path())
        .unwrap()
        .add_account(&account("ann@example.com"))
        .unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    settle(cx, |app| {
        mail.read(app)
            .syncs
            .get(&ann)
            .is_some_and(|each| each.failed.is_some())
    });
    let (told, steps) = tokio::sync::mpsc::unbounded_channel();
    cx.update(|_, app| {
        mail.update(app, |mail, cx| {
            let run = mail.syncs[&ann].run;
            mail.follow_steps(ann, run, steps, cx);
        })
    });
    Store::open(dir.path())
        .unwrap()
        .set_folders(
            ann,
            &[("INBOX", Some(Role::Inbox)), ("Sent", Some(Role::Sent))],
        )
        .unwrap();
    told.send(()).unwrap();
    settle(cx, |app| mail.read(app).syncs[&ann].folders.len() == 2);
}
