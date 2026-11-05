//! A refused Microsoft token signs in again, then syncs (C12.6).

use std::{
    collections::HashMap,
    sync::{Arc, mpsc::Receiver},
    time::{Duration, Instant},
};

use base64::{Engine, engine::general_purpose::STANDARD};
use ely_mail_engine::{Account, AccountId, Auth, Security, Store};
use fake::{Step, expect, oauth, send};
use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext};

use super::{Keys, closed_port, settle, step_words, window_of};
use crate::{boxes::Open, mail::Mail};

const EXPIRED: &str = r#"{"error":"invalid_grant","error_description":"AADSTS700082: expired"}"#;

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}

/// What a code exchange grants `email`.
fn granted(email: &str) -> String {
    let id = oauth::id_token(&format!(r#"{{"email":"{email}"}}"#));
    format!(r#"{{"access_token":"at-2","expires_in":3600,"refresh_token":"rt-2","id_token":"{id}"}}"#)
}

fn bearer(token: &str) -> String {
    STANDARD.encode(format!("user=ann@outlook.com\x01auth=Bearer {token}\x01\x01"))
}

/// Logs in to IMAP with `token`, then `rest`.
fn imap(token: &str, rest: Vec<Step>) -> Vec<Step> {
    let mut script = vec![
        send("* OK ready\n"),
        expect("{tag} AUTHENTICATE XOAUTH2"),
        send("+ \n"),
        expect(&bearer(token)),
        send("{tag} OK welcome\n"),
    ];
    script.extend(rest);
    script.push(expect("{tag} LOGOUT"));
    script.push(send("* BYE\n{tag} OK bye\n"));
    script
}

/// Ann's Microsoft account, its refresh token `rt-1` kept.
fn ann(dir: &std::path::Path, incoming: u16, outgoing: u16) -> (AccountId, Keys) {
    let mut account: Account = fake::account("ann@outlook.com");
    account.auth = Auth::Microsoft;
    account.incoming.port = incoming;
    account.outgoing.port = outgoing;
    let id = Store::open(dir).unwrap().add_account(&account).unwrap();
    let keys = Keys::default();
    keys.kept.lock().unwrap().insert("account:ann@outlook.com".into(), "rt-1".into());
    (id, keys)
}

/// Opens Ann's window, its first sync refused at `token`.
fn refused<'a>(
    cx: &'a mut TestAppContext,
    dir: &std::path::Path,
    id: AccountId,
    keys: Keys,
    tls: &fake::Certs,
    token: String,
) -> (Entity<Mail>, &'a mut VisualTestContext) {
    let providers = Arc::new(move |auth| match auth {
        Auth::Microsoft => Some(Ok(oauth::provider(token.clone()))),
        other => other.provider(),
    });
    let (mail, cx) = window_of(cx, dir, keys, Ok(tls.trusted.clone()), providers);
    settle(cx, |app| {
        mail.read(app).syncs.get(&id).is_some_and(|each| each.task.is_none() && each.failed.is_some())
    });
    let failed = cx.update(|_, app| mail.read(app).syncs[&id].failed.clone().unwrap());
    assert!(failed.contains("invalid_grant"), "{failed}");
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(Open::Account(id), cx)));
    settle(cx, |_| true);
    let again = cx.debug_bounds("banner-action Sign In Again").expect("the failure offers to sign in again");
    cx.simulate_click(again.center(), Modifiers::none());
    assert_eq!(cx.update(|_, app| mail.read(app).adding.as_ref().map(|adding| adding.again)), Some(Some(id)));
    cx.update(|_, app| mail.update(app, |mail, cx| mail.sign_in(cx)));
    let deadline = Instant::now() + Duration::from_secs(60);
    while cx.opened_url().is_none() {
        assert!(Instant::now() < deadline, "the browser never opened");
        std::thread::sleep(Duration::from_millis(20));
        cx.run_until_parked();
    }
    (mail, cx)
}

fn forms(asked: &Receiver<HashMap<String, String>>, count: usize) -> Vec<HashMap<String, String>> {
    (0..count).map(|_| asked.recv().unwrap()).collect()
}

#[test]
fn a_refused_token_signs_in_again_and_syncs() {
    let rt = runtime();
    let certs = fake::certs();
    let fresh = r#"{"access_token":"at-3","expires_in":3600,"refresh_token":"rt-3"}"#;
    let (token, asked) = oauth::endpoint(vec![
        (400, EXPIRED.into()),
        (200, granted("ann@outlook.com")),
        (200, fresh.into()),
    ]);
    let listed = vec![
        expect("{tag} CAPABILITY"),
        send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
        expect("{tag} LIST \"\" *"),
        send("{tag} OK listed\n"),
    ];
    let scripts = vec![imap("at-2", vec![]), imap("at-3", listed)];
    let (incoming, reading) = rt.block_on(fake::serve_each(&certs, Security::Tls, scripts, None));
    let smtp = vec![
        send("220 mx ready\n"),
        expect("EHLO [127.0.0.1]"),
        send("250-mx\n250 AUTH XOAUTH2\n"),
        expect(&format!("AUTH XOAUTH2 {}", bearer("at-2"))),
        send("235 ok\n"),
        expect("QUIT"),
        send("221 bye\n"),
    ];
    let (outgoing, sending) = rt.block_on(fake::serve(&certs, Security::Tls, smtp));
    let dir = tempfile::tempdir().unwrap();
    let (id, keys) = ann(dir.path(), incoming.port, outgoing.port);
    let kept = keys.kept.clone();
    let mut cx = TestAppContext::single();
    let (mail, cx) = refused(&mut cx, dir.path(), id, keys, &certs, token);
    let browser = oauth::approve(&cx.opened_url().unwrap());
    assert!(browser.join().unwrap().starts_with("HTTP/1.1 200"));
    settle(cx, |app| {
        let mail = mail.read(app);
        let syncing = &mail.syncs[&id];
        mail.adding.is_none() && syncing.task.is_none() && syncing.synced.is_some_and(|synced| synced.at.is_some())
    });
    cx.update(|_, app| {
        let syncing = &mail.read(app).syncs[&id];
        assert_eq!((syncing.failed.as_deref(), syncing.signed_out), (None, false));
    });
    assert_eq!(kept.lock().unwrap()["account:ann@outlook.com"], "rt-3");
    let sent = forms(&asked, 3);
    assert_eq!((sent[0]["grant_type"].as_str(), sent[0]["refresh_token"].as_str()), ("refresh_token", "rt-1"));
    assert_eq!(sent[1]["grant_type"], "authorization_code");
    assert_eq!(sent[2]["refresh_token"], "rt-2");
    let limit = Duration::from_secs(60);
    for task in [reading, sending] {
        let done = rt.block_on(async { tokio::time::timeout(limit, task).await });
        done.expect("the script ended").unwrap().unwrap();
    }
}

#[test]
fn another_address_keeps_the_old_token() {
    let certs = fake::certs();
    let (token, _) = oauth::endpoint(vec![(400, EXPIRED.into()), (200, granted("bo@outlook.com"))]);
    let dir = tempfile::tempdir().unwrap();
    let (id, keys) = ann(dir.path(), closed_port(), closed_port());
    let kept = keys.kept.clone();
    let mut cx = TestAppContext::single();
    let (mail, cx) = refused(&mut cx, dir.path(), id, keys, &certs, token);
    let browser = oauth::approve(&cx.opened_url().unwrap());
    browser.join().unwrap();
    let words = |cx: &mut VisualTestContext| step_words(&mail, cx);
    settle(cx, |app| {
        matches!(mail.read(app).adding.as_ref().map(|adding| &adding.step), Some(crate::adding::Step::Failed(_)))
    });
    assert_eq!(words(cx), "You signed in as bo@outlook.com. Sign in as ann@outlook.com.");
    assert_eq!(kept.lock().unwrap()["account:ann@outlook.com"], "rt-1");
}
