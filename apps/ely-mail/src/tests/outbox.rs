//! The outbox: waiting, unknown, cancelled, quitting (C12.14, QA-17).

use ely_gpui_component::primitives::Severity;
use ely_mail_engine::{
    AccountId, Error, Filed, Filing, Going, Landing, Letter, OutboxId, Protocol, Queued,
    Store, Unsent, compose::attachment,
};
use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{
    Keys, account,
    drafts::{new_message, rooted, told, with, write},
    leaving::kept,
    settle, window,
    writing::{NOTE, composing, draft, seeded},
};
use crate::{
    boxes::Open,
    mail::Mail,
    mail::text,
    outbox::{copies, filed, landing, say_filed},
    outbox_list::{Act, acts, key, state},
};
mod steps;

pub(super) fn outbox(mail: &Entity<Mail>, app: &gpui::App) -> Vec<Queued> {
    let store = mail.read(app).store.clone().unwrap();
    store.lock().unwrap().outbox().unwrap()
}

pub(super) fn with_password() -> Keys {
    let keys = Keys::default();
    keys.kept
        .lock()
        .unwrap()
        .insert("account:bo@example.com".into(), "pw".into());
    keys
}

pub(super) fn unsent(words: &str) -> Unsent {
    Unsent {
        to: vec!["ann@example.com".into()],
        cc: Vec::new(),
        bcc: Vec::new(),
        subject: "Hi".into(),
        words: words.into(),
        markdown: false,
        earlier: None,
        in_reply_to: None,
        references: Vec::new(),
        headers_only: false,
        missing: 0,
    }
}

/// Queued before the window opens; `taken` was mid-send.
fn queued_before(dir: &std::path::Path, taken: bool) -> OutboxId {
    let mut store = Store::open(dir).unwrap();
    let bo = store.accounts().unwrap()[0].0;
    let file = attachment("menu.txt", b"soup".to_vec());
    let letter = Letter {
        envelope: vec!["ann@example.com".to_owned()],
        message_id: "q@example.com".into(),
        bytes: b"x".to_vec(),
    };
    let (id, _) = store
        .enqueue(bo, None, &unsent("Lunch?"), &[file], &letter)
        .unwrap();
    if taken {
        store.take_queued(id).unwrap().unwrap();
    }
    id
}

fn act(mail: &Entity<Mail>, id: OutboxId, act: Act, cx: &mut VisualTestContext) {
    cx.update(|_, app| mail.update(app, |mail, cx| mail.outbox_act(id, act, cx)));
}

#[test]
fn an_unreached_server_leaves_the_message_waiting() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), with_password());
    let root = rooted(&mail, cx);
    let compose = new_message(&mail, cx);
    write(compose, "Lunch?", cx);
    with(compose, cx, |compose, window, cx| {
        let addressed = draft(&["ann@example.com"], "Hi");
        compose.edited(addressed.clone(), cx);
        compose.send(&addressed, window, cx);
    });
    settle(cx, |app| {
        let waiting = outbox(&mail, app);
        compose.read(app).is_err()
            && waiting.len() == 1
            && waiting[0].going == Going::Queued
            && waiting[0].error.is_some()
    });
    assert!(told(&root, Severity::Info, cx), "it says it waits");
    assert!(!told(&root, Severity::Danger, cx), "not a failure");
    settle(cx, |app| mail.read(app).pending == 1);
    assert!(kept(&mail, cx).is_empty(), "the draft went with it");
    let [waiting] = cx.update(|_, app| outbox(&mail, app)).try_into().unwrap();
    assert_eq!(waiting.unsent.words, "Lunch?");
    assert_eq!(waiting.unsent.to, ["ann@example.com"]);
}

#[test]
fn a_send_cut_off_is_unknown_at_start_and_never_resent() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let id = queued_before(dir.path(), true);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), with_password());
    cx.update(|_, app| mail.update(app, |mail, cx| mail.run_outbox(true, cx)));
    settle(cx, |app| !mail.read(app).runner.busy);
    let [held] = cx.update(|_, app| outbox(&mail, app)).try_into().unwrap();
    assert_eq!((held.id, held.going, held.error), (id, Going::Unsure, None));
    assert_eq!(cx.update(|_, app| mail.read(app).pending), 0);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(Open::Outbox, cx)));
    settle(cx, |app| mail.read(app).queued.len() == 1);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.select(&[key(id)], cx)));
    settle(cx, |_| true);
    assert!(cx.debug_bounds("mail-outbox-acts").is_some());
}

#[test]
fn a_failed_message_cancels_or_goes_back_to_a_draft() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let id = queued_before(dir.path(), false);
    let mut cx = TestAppContext::single();
    // No password kept: the send fails before any server.
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    settle(cx, |app| {
        let held = outbox(&mail, app);
        held.len() == 1 && held[0].going == Going::Failed
    });
    act(&mail, id, Act::Cancel, cx);
    settle(cx, |app| outbox(&mail, app)[0].going == Going::Cancelled);
    cx.update(|_, app| mail.update(app, |mail, cx| mail.run_outbox(true, cx)));
    settle(cx, |app| !mail.read(app).runner.busy);
    assert_eq!(
        cx.update(|_, app| outbox(&mail, app))[0].going,
        Going::Cancelled,
        "a retry round leaves it"
    );
    act(&mail, id, Act::Edit, cx);
    settle(cx, |_| true);
    let [again] = composing(cx).try_into().unwrap();
    cx.update(|_, app| {
        let again = again.read(app).unwrap();
        assert_eq!(again.document.read(app).text(app), "Lunch?");
        assert_eq!(again.to, ["ann@example.com"]);
        assert_eq!(again.files.len(), 1);
    });
    assert!(cx.update(|_, app| outbox(&mail, app)).is_empty());
}

#[test]
fn quitting_with_mail_waiting_asks_first() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.update(|_, app| mail.update(app, |mail, _| mail.pending = 2));
    let leaves = cx.update(|_, app| mail.update(app, |mail, cx| mail.may_quit(cx)));
    assert!(!leaves, "it asks");
    assert!(cx.update(|_, app| mail.read(app).quitting));
    cx.update(|window, app| window.focus(&mail.read(app).focus.clone(), app));
    for _ in 0..2 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
    cx.simulate_keystrokes("shift-tab enter");
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").unwrap(),
    });
    settle(cx, |app| mail.read(app).quit_anyway);
    let leaves = cx.update(|_, app| mail.update(app, |mail, cx| mail.may_quit(cx)));
    assert!(leaves, "asked once, it goes");
    cx.update(|_, app| {
        mail.update(app, |mail, _| {
            (mail.pending, mail.quit_anyway) = (0, false);
        })
    });
    let leaves = cx.update(|_, app| mail.update(app, |mail, cx| mail.may_quit(cx)));
    assert!(leaves, "nothing waits, nothing asked");
}

#[test]
fn a_message_over_the_known_limit_warns() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let store = Store::open(dir.path()).unwrap();
    let bo = store.accounts().unwrap()[0].0;
    store.set_smtp_limit(bo, Some(2_000)).unwrap();
    drop(store);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    let compose = new_message(&mail, cx);
    let file = dir.path().join("big.bin");
    std::fs::write(&file, vec![7u8; 1_800]).unwrap();
    with(compose, cx, |compose, _, cx| compose.attach(vec![file], cx));
    settle(cx, |app| compose.read(app).unwrap().reading == 0);
    let over = cx.update(|_, app| {
        let compose = compose.read(app).unwrap();
        compose
            .over_limit(app)
            .map(|(sender, whole, limit)| (sender.id, whole, limit))
    });
    assert_eq!(over, Some((bo, 2_400, 2_000)), "base64 makes it larger");
    let store = cx.update(|_, app| mail.read(app).store.clone().unwrap());
    store
        .lock()
        .unwrap()
        .set_smtp_limit(bo, Some(10_000))
        .unwrap();
    with(compose, cx, |_, window, _| window.remove_window());
    let compose = new_message(&mail, cx);
    let fits = cx.update(|_, app| compose.read(app).unwrap().over_limit(app).is_none());
    assert!(fits, "under the limit, nothing warns");
}

#[test]
fn sends_and_copies_end_in_the_right_state() {
    let accepted = landing(Ok(Ok(Some(5))), true);
    assert!(matches!(
        accepted,
        (Landing::Accepted { file: true }, Some(Some(5)))
    ));
    let offline = Error::Connect(std::io::ErrorKind::ConnectionRefused.into());
    assert!(matches!(
        landing(Ok(Err(offline)), true).0,
        Landing::Waiting(_)
    ));
    let slow = Error::Timeout("connecting");
    assert!(matches!(
        landing(Ok(Err(slow)), true).0,
        Landing::Waiting(_)
    ));
    let lost = Error::Unsure("no answer".into());
    assert!(matches!(landing(Ok(Err(lost)), true).0, Landing::Unsure(_)));
    let refused = Error::Server("550 no".into());
    assert!(matches!(
        landing(Ok(Err(refused)), true).0,
        Landing::Failed(_)
    ));
    let large = Error::TooLarge { size: 9, limit: 4 };
    assert!(matches!(
        landing(Ok(Err(large)), false),
        (Landing::Failed(_), Some(Some(4)))
    ));
    let broke = landing(Err("a mail task failed".into()), true);
    assert!(matches!(broke.0, Landing::Unsure(_)), "it may have gone");
    assert!(matches!(filed(Ok(Ok(()))), Filed::Done));
    let offline = Error::Connect(std::io::ErrorKind::ConnectionRefused.into());
    assert!(matches!(filed(Ok(Err(offline))), Filed::Later(_)));
    let full = Error::Server("NO [OVERQUOTA]".into());
    assert!(matches!(filed(Ok(Err(full))), Filed::Failed(_)));
}

#[test]
fn the_sent_copy_follows_the_provider_until_chosen() {
    let mut plain = account("ann@example.com");
    assert!(copies(&plain, None), "a plain server needs it");
    assert!(!copies(&plain, Some(false)));
    plain.outgoing.host = "smtp.gmail.com".into();
    assert!(!copies(&plain, None), "Gmail files its own");
    assert!(copies(&plain, Some(true)), "unless asked");
    plain.protocol = Protocol::Pop { keep: true };
    assert!(!copies(&plain, Some(true)), "POP3 has no Sent folder");
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    drop(store);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.update(|_, app| mail.update(app, |mail, cx| mail.open_box(Open::Account(ann), cx)));
    settle(cx, |app| mail.read(app).sent_copy == Some((ann, None)));
    assert!(
        cx.debug_bounds("mail-sent-copy").is_some(),
        "the switch shows"
    );
    let switch = cx.debug_bounds("mail-sent-copy").unwrap();
    cx.simulate_click(switch.center(), gpui::Modifiers::none());
    settle(cx, |app| {
        let store = mail.read(app).store.clone().unwrap();
        let choice = store.lock().unwrap().sent_copy(ann).unwrap();
        choice == Some(false)
    });
}

#[test]
fn a_copy_waiting_after_a_miss_says_why_and_can_go_now() {
    let dir = tempfile::tempdir().unwrap();
    seeded(
        dir.path(),
        format!(
            "{NOTE}
x"
        )
        .as_bytes(),
        &[],
    );
    let mut cx = TestAppContext::single();
    let (_, cx) = window(&mut cx, dir.path(), Keys::default());
    let missed = Queued {
        id: OutboxId(1),
        account: AccountId(1),
        going: Going::Accepted,
        filing: Some(Filing::Waiting),
        message_id: "q@example.com".into(),
        unsent: unsent("Lunch?"),
        queued: 0,
        tried: Some(0),
        error: Some("refused".into()),
    };
    let saving = Queued {
        error: None,
        ..missed.clone()
    };
    let (shown, later, now) = cx.update(|_, app| {
        let later = text(app, "mail.outbox.filing_later", &[("error", "refused")]);
        (state(&missed, app), later, state(&saving, app))
    });
    assert_eq!(shown, later, "it says why, and when it is tried");
    assert_ne!(now, later, "a copy going now says so");
    assert_eq!(acts(&missed), [(Act::Retry, "mail.outbox.file_again")]);
    assert!(acts(&saving).is_empty(), "nothing to do while it goes");
    let told = say_filed(&Filed::Later("refused".into()), false);
    assert_eq!(
        told,
        Some((
            "mail.outbox.unfiled_later",
            Severity::Info,
            "refused".into()
        ))
    );
    assert_eq!(say_filed(&Filed::Later("refused".into()), true), None);
}
