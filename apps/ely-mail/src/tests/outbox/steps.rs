//! Drafts handed over whole; the list follows each step.

use std::{
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, TryRecvError},
    thread,
    time::Duration,
};

use ely_mail_engine::{Going, Letter, Role, Store};
use gpui::TestAppContext;

use super::{outbox, unsent, with_password};
use crate::{
    boxes::Open,
    tests::{
        account,
        drafts::{new_message, pause, silent_server, with, write},
        leaving::kept,
        settle, window,
        writing::{NOTE, draft, seeded},
    },
};

#[test]
fn a_kept_draft_goes_into_the_queue_with_it() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), with_password());
    let compose = new_message(&mail, cx);
    write(compose, "Lunch?", cx);
    let addressed = draft(&["ann@example.com"], "Hi");
    with(compose, cx, |compose, _, cx| {
        compose.edited(addressed.clone(), cx)
    });
    pause(&mail, cx, 1);
    let first = kept(&mail, cx)[0].id;
    with(compose, cx, |compose, window, cx| {
        compose.send(&addressed, window, cx);
        let document = compose.document.clone();
        document.update(cx, |document, cx| document.set_text("Lunch at one?", cx));
        compose.keep(cx);
        assert!(!compose.shelf_busy(), "no draft write while sending");
    });
    settle(cx, |app| {
        let store = mail.read(app).store.clone().unwrap();
        let drafts = store.lock().unwrap().drafts().unwrap();
        outbox(&mail, app).len() == 1 && drafts.len() == 1
    });
    let [later] = kept(&mail, cx).try_into().unwrap();
    assert_ne!(later.id, first, "the sent draft went with the queue");
    assert_eq!(later.unsent.words, "Lunch at one?", "edits while sending");
    assert!(cx.update(|_, app| compose.read(app).is_ok()), "it stays");
}

#[test]
fn a_send_during_a_draft_write_waits_for_it() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), with_password());
    let compose = new_message(&mail, cx);
    write(compose, "Lunch?", cx);
    let addressed = draft(&["ann@example.com"], "Hi");
    with(compose, cx, |compose, window, cx| {
        compose.edited(addressed.clone(), cx);
        compose.keep(cx);
        compose.send(&addressed, window, cx);
    });
    settle(cx, |app| {
        compose.read(app).is_err() && outbox(&mail, app).len() == 1
    });
    cx.run_until_parked();
    assert!(kept(&mail, cx).is_empty(), "no draft left behind");
}

#[test]
fn a_send_queues_what_was_there_when_pressed() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), with_password());
    let compose = new_message(&mail, cx);
    write(compose, "Lunch?", cx);
    let addressed = draft(&["ann@example.com"], "Hi");
    with(compose, cx, |compose, window, cx| {
        compose.edited(addressed.clone(), cx);
        compose.keep(cx);
        compose.send(&addressed, window, cx);
        assert!(compose.shelf_busy(), "the send waits for the write");
        let document = compose.document.clone();
        document.update(cx, |document, cx| document.set_text("Lunch at one?", cx));
    });
    settle(cx, |app| {
        let store = mail.read(app).store.clone().unwrap();
        let drafts = store.lock().unwrap().drafts().unwrap();
        outbox(&mail, app).len() == 1 && drafts.len() == 1
    });
    let [queued] = cx.update(|_, app| outbox(&mail, app)).try_into().unwrap();
    assert_eq!(queued.unsent.words, "Lunch?", "the words as pressed");
    let [later] = kept(&mail, cx).try_into().unwrap();
    assert_eq!(
        later.unsent.words, "Lunch at one?",
        "later edits stay a draft"
    );
}

/// Holds each connection; the first goes when told, once here.
fn dropping_first() -> (u16, mpsc::Receiver<()>, mpsc::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let (go, told) = mpsc::channel();
    let (came, arrived) = mpsc::channel();
    thread::spawn(move || {
        let (mut first, mut rest, mut gone) = (None::<TcpStream>, Vec::new(), false);
        loop {
            match told.try_recv() {
                Ok(()) => gone = true,
                Err(TryRecvError::Disconnected) => return,
                Err(TryRecvError::Empty) => {}
            }
            if let Ok((stream, _)) = listener.accept() {
                match first.is_none() && !gone {
                    true => {
                        first = Some(stream);
                        came.send(()).unwrap();
                    }
                    false => rest.push(stream),
                }
            }
            if gone {
                first = None;
            }
            thread::sleep(Duration::from_millis(10));
        }
    });
    (port, arrived, go)
}

#[test]
fn the_list_follows_each_send_as_it_goes() {
    let dir = tempfile::tempdir().unwrap();
    let (port, arrived, go) = dropping_first();
    let (quiet, _keep) = silent_server();
    let mut bo = account("bo@example.com");
    (bo.outgoing.port, bo.incoming.port) = (port, quiet);
    let mut store = Store::open(dir.path()).unwrap();
    let id = store.add_account(&bo).unwrap();
    store
        .set_folders(id, &[("INBOX", Some(Role::Inbox))])
        .unwrap();
    drop(store);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), with_password());
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(Open::Outbox, cx)));
    // The opening round is over; the sync hangs, reading nothing.
    settle(cx, |app| {
        let mail = mail.read(app);
        mail.syncs.contains_key(&id) && !mail.runner.busy
    });
    let store = cx.update(|_, app| mail.read(app).store.clone().unwrap());
    let mut store = store.lock().unwrap();
    for (n, words) in ["first", "second"].into_iter().enumerate() {
        let letter = Letter {
            envelope: vec!["ann@example.com".to_owned()],
            message_id: format!("{n}@example.com"),
            bytes: b"Subject: x\r\n\r\nx\r\n".to_vec(),
        };
        store
            .enqueue(id, None, &unsent(words), &[], &letter)
            .unwrap();
    }
    drop(store);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.read_mail(cx)));
    settle(cx, |app| {
        let shown = &mail.read(app).queued;
        shown.len() == 2 && shown.iter().all(|it| it.going == Going::Queued)
    });
    cx.update(|_, app| mail.update(app, |mail, cx| mail.run_outbox(false, cx)));
    settle(cx, |app| mail.read(app).queued[0].going == Going::Sending);
    // Sending shows before the connection lands; drop it once here.
    arrived.recv_timeout(Duration::from_secs(20)).expect("the first connection");
    go.send(()).unwrap();
    settle(cx, |app| {
        let shown = &mail.read(app).queued;
        shown[0].error.is_some() && shown[1].going == Going::Sending
    });
    let held = cx.update(|_, app| outbox(&mail, app));
    assert_eq!(held[1].going, Going::Sending, "the second still hangs");
}
