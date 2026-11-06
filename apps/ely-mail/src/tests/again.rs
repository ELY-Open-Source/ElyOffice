//! A refused Microsoft token signs in again, then syncs (C12.6).

use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    },
    time::{Duration, Instant},
};

use base64::{Engine, engine::general_purpose::STANDARD};
use ely_mail_engine::{Account, AccountId, Auth, Security, Server, Store};
use fake::{Step, expect, oauth, send};
use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext};
use tokio::{runtime::Runtime, task::JoinHandle};

use super::{Keys, closed_port, settle, step_words, window_of};
use crate::{boxes::Open, mail::Mail};

const EXPIRED: &str = r#"{"error":"invalid_grant","error_description":"AADSTS700082: expired"}"#;
const FRESH: &str = r#"{"access_token":"at-3","expires_in":3600,"refresh_token":"rt-3"}"#;

type Script = JoinHandle<Result<(), String>>;

fn runtime() -> Runtime {
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
        expect("{tag} CAPABILITY"),
        send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
    ];
    script.extend(rest);
    script.push(expect("{tag} LOGOUT"));
    script.push(send("* BYE\n{tag} OK bye\n"));
    script
}

/// A sync with no folders, as `token`.
fn listed(token: &str) -> Vec<Step> {
    let rest = vec![
        expect("{tag} LIST \"\" *"),
        send("{tag} OK listed\n"),
    ];
    imap(token, rest)
}

/// IMAP serving `scripts`; SMTP passing the check with `at-2`.
fn servers(rt: &Runtime, certs: &fake::Certs, scripts: Vec<Vec<Step>>) -> (Server, Server, [Script; 2]) {
    let (incoming, reading) = rt.block_on(fake::serve_each(certs, Security::Tls, scripts, None));
    let smtp = vec![
        send("220 mx ready\n"),
        expect("EHLO [127.0.0.1]"),
        send("250-mx\n250 AUTH XOAUTH2\n"),
        expect(&format!("AUTH XOAUTH2 {}", bearer("at-2"))),
        send("235 ok\n"),
        expect("QUIT"),
        send("221 bye\n"),
    ];
    let (outgoing, sending) = rt.block_on(fake::serve(certs, Security::Tls, smtp));
    (incoming, outgoing, [reading, sending])
}

fn ended(rt: &Runtime, scripts: [Script; 2]) {
    for task in scripts {
        let done = rt.block_on(async { tokio::time::timeout(Duration::from_secs(60), task).await });
        done.expect("the script ended").unwrap().unwrap();
    }
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
    token: Arc<Mutex<String>>,
) -> (Entity<Mail>, &'a mut VisualTestContext) {
    let providers = Arc::new(move |auth| match auth {
        Auth::Microsoft => Some(Ok(oauth::provider(token.lock().unwrap().clone()))),
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

/// Synced since, its refusal gone.
fn synced(mail: &Entity<Mail>, id: AccountId, cx: &mut VisualTestContext) {
    settle(cx, |app| {
        let mail = mail.read(app);
        let syncing = &mail.syncs[&id];
        mail.adding.is_none() && syncing.task.is_none() && syncing.synced.is_some_and(|synced| synced.at.is_some())
    });
    cx.update(|_, app| {
        let syncing = &mail.read(app).syncs[&id];
        assert_eq!((syncing.failed.as_deref(), syncing.signed_out), (None, false));
    });
}

#[test]
fn a_refused_token_signs_in_again_and_syncs() {
    let rt = runtime();
    let certs = fake::certs();
    let answers = vec![(400, EXPIRED.into()), (200, granted("ann@outlook.com")), (200, FRESH.into())];
    let (token, asked) = oauth::endpoint(answers);
    let (incoming, outgoing, scripts) = servers(&rt, &certs, vec![imap("at-2", vec![]), listed("at-3")]);
    let dir = tempfile::tempdir().unwrap();
    let (id, keys) = ann(dir.path(), incoming.port, outgoing.port);
    let kept = keys.kept.clone();
    let mut cx = TestAppContext::single();
    let (mail, cx) = refused(&mut cx, dir.path(), id, keys, &certs, Arc::new(Mutex::new(token)));
    let browser = oauth::approve(&cx.opened_url().unwrap());
    assert!(browser.join().unwrap().starts_with("HTTP/1.1 200"));
    synced(&mail, id, cx);
    assert_eq!(kept.lock().unwrap()["account:ann@outlook.com"], "rt-3");
    let sent = forms(&asked, 3);
    assert_eq!((sent[0]["grant_type"].as_str(), sent[0]["refresh_token"].as_str()), ("refresh_token", "rt-1"));
    assert_eq!(sent[1]["grant_type"], "authorization_code");
    assert_eq!(sent[2]["refresh_token"], "rt-2");
    ended(&rt, scripts);
}

#[test]
fn another_address_keeps_the_old_token() {
    let certs = fake::certs();
    let (token, _) = oauth::endpoint(vec![(400, EXPIRED.into()), (200, granted("bo@outlook.com"))]);
    let dir = tempfile::tempdir().unwrap();
    let (id, keys) = ann(dir.path(), closed_port(), closed_port());
    let kept = keys.kept.clone();
    let mut cx = TestAppContext::single();
    let (mail, cx) = refused(&mut cx, dir.path(), id, keys, &certs, Arc::new(Mutex::new(token)));
    let browser = oauth::approve(&cx.opened_url().unwrap());
    browser.join().unwrap();
    settle(cx, |app| {
        matches!(mail.read(app).adding.as_ref().map(|adding| &adding.step), Some(crate::adding::Step::Failed(_)))
    });
    assert_eq!(step_words(&mail, cx), "You signed in as bo@outlook.com. Sign in as ann@outlook.com.");
    assert_eq!(kept.lock().unwrap()["account:ann@outlook.com"], "rt-1");
}

#[test]
fn a_sign_in_while_paused_clears_the_refusal() {
    let rt = runtime();
    let certs = fake::certs();
    let (token, _) = oauth::endpoint(vec![(400, EXPIRED.into()), (200, granted("ann@outlook.com"))]);
    let (incoming, outgoing, scripts) = servers(&rt, &certs, vec![imap("at-2", vec![])]);
    let dir = tempfile::tempdir().unwrap();
    let (id, keys) = ann(dir.path(), incoming.port, outgoing.port);
    let kept = keys.kept.clone();
    let mut cx = TestAppContext::single();
    let (mail, cx) = refused(&mut cx, dir.path(), id, keys, &certs, Arc::new(Mutex::new(token)));
    cx.update(|_, app| mail.update(app, |mail, cx| mail.set_paused(true, cx)));
    oauth::approve(&cx.opened_url().unwrap()).join().unwrap();
    settle(cx, |app| mail.read(app).adding.is_none());
    cx.update(|_, app| {
        let syncing = &mail.read(app).syncs[&id];
        assert_eq!((syncing.failed.as_deref(), syncing.signed_out), (None, false));
        assert!(syncing.task.is_none(), "paused, nothing syncs");
    });
    settle(cx, |_| true);
    assert!(cx.debug_bounds("banner-action Sign In Again").is_none());
    assert_eq!(kept.lock().unwrap()["account:ann@outlook.com"], "rt-2");
    ended(&rt, scripts);
}

/// A token endpoint that holds its one request, then answers.
fn held(status: u16, body: &'static str) -> (String, Receiver<()>, mpsc::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/token", listener.local_addr().unwrap());
    let (took, taken) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut data = Vec::new();
        let mut chunk = [0u8; 4096];
        while !String::from_utf8_lossy(&data).contains("refresh_token=") {
            let count = stream.read(&mut chunk).unwrap();
            data.extend_from_slice(&chunk[..count]);
        }
        took.send(()).unwrap();
        released.recv().unwrap();
        let reply = format!(
            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        // The abandoned sync may have gone.
        stream.write_all(reply.as_bytes()).ok();
    });
    (url, taken, release)
}

/// Waits for `told`, the window running meanwhile.
#[track_caller]
fn heard(told: &Receiver<()>, cx: &mut VisualTestContext) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while told.try_recv().is_err() {
        assert!(Instant::now() < deadline, "never told");
        std::thread::sleep(Duration::from_millis(20));
        cx.run_until_parked();
    }
}

#[test]
fn a_sign_in_during_a_sync_starts_over_on_the_new_token() {
    let rt = runtime();
    let certs = fake::certs();
    let (token, asked) = oauth::endpoint(vec![(400, EXPIRED.into()), (200, granted("ann@outlook.com"))]);
    let (incoming, outgoing, scripts) = servers(&rt, &certs, vec![imap("at-2", vec![]), listed("at-3")]);
    let dir = tempfile::tempdir().unwrap();
    let (id, keys) = ann(dir.path(), incoming.port, outgoing.port);
    let kept = keys.kept.clone();
    let url = Arc::new(Mutex::new(token));
    let mut cx = TestAppContext::single();
    let (mail, cx) = refused(&mut cx, dir.path(), id, keys, &certs, url.clone());
    let (old, took_old, release_old) = held(400, EXPIRED);
    *url.lock().unwrap() = old;
    cx.update(|_, app| mail.update(app, |mail, cx| mail.sync(id, cx)));
    heard(&took_old, cx);
    let (new, took_new, release_new) = held(200, FRESH);
    *url.lock().unwrap() = new;
    oauth::approve(&cx.opened_url().unwrap()).join().unwrap();
    heard(&took_new, cx);
    release_old.send(()).unwrap();
    let shown = |cx: &mut VisualTestContext| {
        settle(cx, |_| true);
        let syncing = cx.update(|_, app| {
            let syncing = &mail.read(app).syncs[&id];
            (syncing.task.is_some(), syncing.signed_out, syncing.failed.clone())
        });
        (syncing, cx.debug_bounds("banner-action Sign In Again").is_some())
    };
    for _ in 0..10 {
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(shown(cx), ((true, false, None), false), "the old refusal stays gone");
    }
    release_new.send(()).unwrap();
    synced(&mail, id, cx);
    assert_eq!(kept.lock().unwrap()["account:ann@outlook.com"], "rt-3");
    assert_eq!(forms(&asked, 2)[1]["grant_type"], "authorization_code");
    ended(&rt, scripts);
}
