//! The whole window against scripted servers (C12.23, QA-17, QA-18).

use std::path::Path;

use ely_mail_engine::{
    Account, AccountId, Filing, Going, Letter, Role, Scope, Security, Store, Summary,
};
use fake::{Step, expect, send};
use gpui::{App, Entity, TestAppContext, VisualTestContext};

use super::{
    closed_port,
    outbox::{outbox, unsent, with_password},
    settle, settle_within, window_with,
};
use crate::mail::Mail;

/// Below the engine's 60 s wait: no logout awaited.
const PROMPT: std::time::Duration = std::time::Duration::from_secs(20);

/// Bo's PLAIN login, `\0bo@example.com\0pw`.
const AUTH: &str = "AUTH PLAIN AGJvQGV4YW1wbGUuY29tAHB3";

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}

/// The script's result; a stall fails, never hangs.
fn ended(rt: &tokio::runtime::Runtime, task: tokio::task::JoinHandle<Result<(), String>>) {
    let limit = std::time::Duration::from_secs(60);
    let done = rt.block_on(async { tokio::time::timeout(limit, task).await });
    done.expect("the script ended").unwrap().unwrap();
}

fn bo(incoming: u16, outgoing: u16) -> Account {
    let mut bo = fake::account("bo@example.com");
    bo.incoming.port = incoming;
    bo.outgoing.port = outgoing;
    bo
}

/// Bo's store with one message queued to Ann.
fn queued(dir: &Path, account: &Account) {
    let mut store = Store::open(dir).unwrap();
    let bo = store.add_account(account).unwrap();
    let folders = [("INBOX", Some(Role::Inbox)), ("Sent", Some(Role::Sent))];
    store.set_folders(bo, &folders).unwrap();
    let letter = Letter {
        envelope: vec!["ann@example.com".into()],
        message_id: "q@example.com".into(),
        bytes: b"Subject: Hi\r\n\r\nLunch?\r\n".to_vec(),
    };
    store.enqueue(bo, None, &unsent("Lunch?"), &[], &letter).unwrap();
}

/// Greeting, login and envelope up to RCPT.
fn smtp(rest: Vec<Step>) -> Vec<Step> {
    let mut script = vec![
        send("220 mx ready\n"),
        expect("EHLO [127.0.0.1]"),
        send("250-mx\n250 AUTH PLAIN\n"),
        expect(AUTH),
        send("235 ok\n"),
        expect("MAIL FROM:<bo@example.com>"),
        send("250 ok\n"),
        expect("RCPT TO:<ann@example.com>"),
    ];
    script.extend(rest);
    script
}

fn idle(mail: &Entity<Mail>, cx: &mut VisualTestContext) {
    cx.update(|_, app| mail.update(app, |mail, cx| mail.sync_all(cx)));
    settle(cx, |app| !mail.read(app).runner.busy);
}

#[test]
fn a_refused_recipient_fails_and_is_never_sent_again() {
    let rt = runtime();
    let certs = fake::certs();
    let (seen, accepted) = std::sync::mpsc::channel();
    let script = smtp(vec![send("550 5.1.1 no such user\n"), Step::Hang]);
    let serving = fake::serve_each(&certs, Security::Tls, vec![script], Some(seen));
    let (server, task) = rt.block_on(serving);
    let dir = tempfile::tempdir().unwrap();
    queued(dir.path(), &bo(closed_port(), server.port));
    let mut cx = TestAppContext::single();
    let (mail, cx) = window_with(&mut cx, dir.path(), with_password(), Ok(certs.trusted.clone()));
    settle_within(cx, PROMPT, |app| {
        let held = outbox(&mail, app);
        held.len() == 1 && held[0].going == Going::Failed
    });
    ended(&rt, task);
    idle(&mail, cx);
    let [held] = cx.update(|_, app| outbox(&mail, app)).try_into().unwrap();
    assert_eq!(held.going, Going::Failed);
    let error = held.error.unwrap();
    assert!(error.contains("550") && error.contains("no such user"), "{error}");
    assert_eq!(accepted.try_iter().count(), 1, "one connection, no retry");
}

#[test]
fn an_accepted_message_with_no_quit_answer_waits_only_for_its_copy() {
    let rt = runtime();
    let certs = fake::certs();
    let (seen, accepted) = std::sync::mpsc::channel();
    let script = smtp(vec![
        send("250 ok\n"),
        expect("DATA"),
        send("354 go\n"),
        Step::AnyData,
        send("250 queued\n"),
        Step::Hang,
    ]);
    let serving = fake::serve_each(&certs, Security::Tls, vec![script], Some(seen));
    let (server, task) = rt.block_on(serving);
    let dir = tempfile::tempdir().unwrap();
    queued(dir.path(), &bo(closed_port(), server.port));
    let mut cx = TestAppContext::single();
    let (mail, cx) = window_with(&mut cx, dir.path(), with_password(), Ok(certs.trusted.clone()));
    let taken = |app: &App| {
        let held = outbox(&mail, app);
        held.len() == 1
            && held[0].going == Going::Accepted
            && held[0].filing == Some(Filing::Waiting)
    };
    settle_within(cx, PROMPT, taken);
    ended(&rt, task);
    idle(&mail, cx);
    assert!(cx.update(|_, app| taken(app)), "still accepted, copy waiting");
    assert_eq!(accepted.try_iter().count(), 1, "never sent twice");
}

/// A literal FETCH line with `head` as the header.
fn header(uid: u32, head: &str) -> String {
    let crlf = head.replace('\n', "\r\n");
    format!(
        "* 1 FETCH (UID {uid} FLAGS (\\Seen) BODY[HEADER] {{{}}}\n{head})\n{{tag}} OK headers\n",
        crlf.len()
    )
}

/// One old message in INBOX under `validity`.
fn imap(validity: u32, uid: u32, head: &str) -> Vec<Step> {
    vec![
        send("* OK ready\n"),
        expect("{tag} LOGIN \"bo@example.com\" \"pw\""),
        send("{tag} OK done\n"),
        expect("{tag} CAPABILITY"),
        send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
        expect("{tag} LIST \"\" *"),
        send("* LIST (\\HasNoChildren) \"/\" INBOX\n{tag} OK listed\n"),
        expect("{tag} EXAMINE \"INBOX\""),
        send(&format!(
            "* 1 EXISTS\n* OK [UIDVALIDITY {validity}] v\n{{tag}} OK in\n"
        )),
        expect("{tag} UID FETCH 1:* (UID FLAGS)"),
        send(&format!("* 1 FETCH (UID {uid} FLAGS (\\Seen))\n{{tag}} OK flags\n")),
        Step::Prefix("{tag} UID SEARCH SINCE".into()),
        send("* SEARCH\n{tag} OK found\n"),
        expect(&format!("{{tag}} UID FETCH {uid} (UID FLAGS BODY.PEEK[HEADER])")),
        send(&header(uid, head)),
        expect("{tag} LOGOUT"),
        send("* BYE\n{tag} OK bye\n"),
    ]
}

fn listed(mail: &Entity<Mail>, cx: &mut VisualTestContext) -> Vec<Summary> {
    cx.update(|_, app| {
        let store = mail.read(app).store.clone().unwrap();
        let store = store.lock().unwrap();
        store.list(Scope::Inboxes, Default::default()).unwrap()
    })
}

/// Starts a sync and waits until it ends.
fn synced(mail: &Entity<Mail>, id: AccountId, cx: &mut VisualTestContext) {
    let before = cx.update(|_, app| mail.read(app).syncs[&id].run);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.sync_all(cx)));
    settle(cx, |app| {
        let syncing = &mail.read(app).syncs[&id];
        syncing.run != before && syncing.task.is_none()
    });
}

#[test]
fn a_new_uidvalidity_rebuilds_and_a_refused_login_keeps_local_mail() {
    let rt = runtime();
    let certs = fake::certs();
    let scripts = vec![
        imap(7, 1, "Subject: Old\nFrom: a@example.com\n\n"),
        imap(8, 5, "Subject: New\nFrom: b@example.com\n\n"),
        vec![
            send("* OK ready\n"),
            expect("{tag} LOGIN \"bo@example.com\" \"pw\""),
            send("{tag} NO [AUTHENTICATIONFAILED] token expired\n"),
        ],
    ];
    let (server, task) = rt.block_on(fake::serve_each(&certs, Security::Tls, scripts, None));
    let dir = tempfile::tempdir().unwrap();
    let id = Store::open(dir.path())
        .unwrap()
        .add_account(&bo(server.port, closed_port()))
        .unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window_with(&mut cx, dir.path(), with_password(), Ok(certs.trusted.clone()));
    settle(cx, |app| {
        mail.read(app)
            .syncs
            .get(&id)
            .is_some_and(|each| each.task.is_none() && each.synced.is_some())
    });
    let [old] = listed(&mail, cx).try_into().unwrap();
    assert_eq!(old.subject.as_deref(), Some("Old"));
    cx.update(|_, app| {
        let store = mail.read(app).store.clone().unwrap();
        let mut store = store.lock().unwrap();
        store.shelve(None, id, &unsent("half written"), None).unwrap();
    });
    synced(&mail, id, cx);
    let [new] = listed(&mail, cx).try_into().unwrap();
    assert_eq!(new.subject.as_deref(), Some("New"));
    assert_ne!(new.id, old.id, "rebuilt, not renamed in place");
    synced(&mail, id, cx);
    let failed = cx.update(|_, app| mail.read(app).syncs[&id].failed.clone()).unwrap();
    assert!(failed.contains("token expired"), "{failed}");
    let [kept] = listed(&mail, cx).try_into().unwrap();
    assert_eq!(kept.id, new.id, "local mail stays after a refused login");
    let drafts = cx.update(|_, app| {
        let store = mail.read(app).store.clone().unwrap();
        let drafts = store.lock().unwrap().drafts().unwrap();
        drafts.into_iter().map(|each| each.unsent.words).collect::<Vec<_>>()
    });
    assert_eq!(drafts, ["half written"], "syncs never touch drafts");
    ended(&rt, task);
}

/// INBOX under 7 holds uid 1, nothing new or recent.
fn quiet() -> Vec<Step> {
    vec![
        send("* OK ready\n"),
        expect("{tag} LOGIN \"bo@example.com\" \"pw\""),
        send("{tag} OK done\n"),
        expect("{tag} CAPABILITY"),
        send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
        expect("{tag} LIST \"\" *"),
        send("* LIST (\\HasNoChildren) \"/\" INBOX\n{tag} OK listed\n"),
        expect("{tag} EXAMINE \"INBOX\""),
        send("* 1 EXISTS\n* OK [UIDVALIDITY 7] v\n{tag} OK in\n"),
        expect("{tag} UID FETCH 1:* (UID FLAGS)"),
        send("* 1 FETCH (UID 1 FLAGS (\\Seen))\n{tag} OK flags\n"),
        Step::Prefix("{tag} UID SEARCH SINCE".into()),
        send("* SEARCH\n{tag} OK found\n"),
        expect("{tag} LOGOUT"),
        send("* BYE\n{tag} OK bye\n"),
    ]
}

#[test]
fn a_sync_trims_downloads_to_the_cap() {
    let rt = runtime();
    let certs = fake::certs();
    let (server, task) = rt.block_on(fake::serve_each(&certs, Security::Tls, vec![quiet(), quiet()], None));
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let id = store.add_account(&bo(server.port, closed_port())).unwrap();
    let [inbox] = store.set_folders(id, &[("INBOX", Some(ely_mail_engine::Role::Inbox))]).unwrap().try_into().unwrap();
    store.open_mailbox(inbox, 7).unwrap();
    let whole = b"Subject: kept\r\nFrom: a@example.com\r\n\r\nthe body\r\n";
    let key = ely_mail_engine::Key::Imap { uidvalidity: 7, uid: 1 };
    let message = store.put(inbox, &key, &["\\Seen"], ely_mail_engine::Fetched::Whole(whole)).unwrap();
    drop(store);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window_with(&mut cx, dir.path(), with_password(), Ok(certs.trusted.clone()));
    settle(cx, |app| mail.read(app).syncs.get(&id).is_some_and(|each| each.task.is_none() && each.synced.is_some()));
    let original = |cx: &mut VisualTestContext| {
        cx.update(|_, app| mail.read(app).store.clone().unwrap().lock().unwrap().original(message).unwrap())
    };
    assert!(original(cx).is_some(), "no cap, nothing trimmed");
    // Any cap above nothing kept: zero bytes.
    cx.update(|_, app| mail.update(app, |mail, _| mail.cap.gb = Some(Ok(Some(0)))));
    synced(&mail, id, cx);
    assert_eq!(original(cx), None, "the sync trimmed to the cap");
    ended(&rt, task);
}

#[test]
fn a_cap_lifted_during_a_sync_trims_nothing() {
    let rt = runtime();
    let certs = fake::certs();
    let (server, task) = rt.block_on(fake::serve_each(&certs, Security::Tls, vec![quiet(), quiet()], None));
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let id = store.add_account(&bo(server.port, closed_port())).unwrap();
    let [inbox] = store.set_folders(id, &[("INBOX", Some(ely_mail_engine::Role::Inbox))]).unwrap().try_into().unwrap();
    store.open_mailbox(inbox, 7).unwrap();
    let whole = b"Subject: kept\r\nFrom: a@example.com\r\n\r\nthe body\r\n";
    let key = ely_mail_engine::Key::Imap { uidvalidity: 7, uid: 1 };
    let message = store.put(inbox, &key, &["\\Seen"], ely_mail_engine::Fetched::Whole(whole)).unwrap();
    drop(store);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window_with(&mut cx, dir.path(), with_password(), Ok(certs.trusted.clone()));
    settle(cx, |app| mail.read(app).syncs.get(&id).is_some_and(|each| each.task.is_none() && each.synced.is_some()));
    cx.update(|_, app| mail.update(app, |mail, _| mail.cap.gb = Some(Ok(Some(0)))));
    let before = cx.update(|_, app| mail.read(app).syncs[&id].run);
    // Lifted while the server still talks.
    cx.update(|_, app| {
        mail.update(app, |mail, cx| {
            mail.sync_all(cx);
            mail.set_cap(None, cx);
        })
    });
    settle(cx, |app| {
        let syncing = &mail.read(app).syncs[&id];
        syncing.run != before && syncing.task.is_none()
    });
    let kept = cx.update(|_, app| mail.read(app).store.clone().unwrap().lock().unwrap().original(message).unwrap());
    assert!(kept.is_some(), "no cap now, nothing trimmed");
    ended(&rt, task);
}
