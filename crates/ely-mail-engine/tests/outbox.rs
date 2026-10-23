use ely_mail_engine::{
    Draft, DraftId, Error, Filed, Filing, Going, Landing, Left, Letter, Mailbox, OutboxId, Remote,
    Store, Unsent, build, compose::attachment, read, without_bcc,
};
use fake::account;

fn unsent() -> Unsent {
    Unsent {
        to: vec!["bo@example.com".into()],
        cc: Vec::new(),
        bcc: vec!["cy@example.com".into()],
        subject: "Plan".into(),
        words: "Lunch?".into(),
        markdown: false,
        earlier: None,
        in_reply_to: None,
        references: Vec::new(),
        headers_only: false,
        missing: 0,
    }
}

fn letter() -> Letter {
    Letter {
        envelope: vec!["bo@example.com".to_owned(), "cy@example.com".to_owned()],
        message_id: "id@example.com".into(),
        bytes: b"bytes".to_vec(),
    }
}

fn queue(store: &mut Store) -> OutboxId {
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let file = attachment("plan.pdf", b"%PDF".to_vec());
    let (id, left) = store
        .enqueue(ann, None, &unsent(), &[file], &letter())
        .unwrap();
    assert_eq!(left, None);
    id
}

#[test]
fn a_draft_goes_into_the_queue_in_one_step() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let draft = store.shelve(None, ann, &unsent(), Some(&[])).unwrap().id;
    let copy = Remote {
        mailbox: "Drafts".into(),
        uidvalidity: 7,
        uid: 41,
    };
    store.set_remote(draft, ann, &copy).unwrap();
    let (id, left) = store
        .enqueue(ann, Some(draft), &unsent(), &[], &letter())
        .unwrap();
    let held = Left {
        account: ann,
        remote: copy,
    };
    assert_eq!(left, Some(held), "its server copy is to remove");
    assert!(store.drafts().unwrap().is_empty(), "the draft went with it");
    assert_eq!(store.outbox().unwrap()[0].id, id);
}

#[test]
fn a_gone_draft_queues_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let gone = Some(DraftId(999));
    let err = store
        .enqueue(ann, gone, &unsent(), &[], &letter())
        .unwrap_err();
    assert!(matches!(err, Error::Store(_)), "{err}");
    assert!(store.outbox().unwrap().is_empty(), "rolled back");
}

fn going(store: &Store, id: OutboxId) -> (Going, Option<Filing>, Option<String>) {
    let held = store.queued(id).unwrap();
    (held.going, held.filing, held.error)
}

#[test]
fn a_queued_message_is_sent_once_then_filed() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let id = queue(&mut store);
    assert_eq!(store.to_send(false).unwrap(), [id]);
    assert_eq!(store.pending().unwrap(), 1);
    let handed = store.take_queued(id).unwrap().expect("queued");
    assert_eq!(handed.envelope, ["bo@example.com", "cy@example.com"]);
    assert_eq!(handed.bytes, b"bytes");
    assert_eq!(going(&store, id).0, Going::Sending);
    assert!(store.take_queued(id).unwrap().is_none(), "taken once");
    assert!(store.to_send(true).unwrap().is_empty());
    store.landed(id, Landing::Accepted { file: true }).unwrap();
    assert_eq!(
        going(&store, id),
        (Going::Accepted, Some(Filing::Waiting), None)
    );
    assert_eq!(store.to_file().unwrap(), [id]);
    assert_eq!(store.filing(id).unwrap().unwrap().1, b"bytes");
    store.filed(id, Filed::Later("offline".into())).unwrap();
    assert_eq!(store.to_file().unwrap(), [id], "unreached, it waits");
    store.retry(id).unwrap();
    assert_eq!(
        going(&store, id),
        (Going::Accepted, Some(Filing::Waiting), None),
        "tried again now, the reason clears"
    );
    store.filed(id, Filed::Failed("no room".into())).unwrap();
    let (state, filing, error) = going(&store, id);
    assert_eq!((state, filing), (Going::Accepted, Some(Filing::Failed)));
    assert_eq!(error.as_deref(), Some("no room"));
    assert!(store.to_file().unwrap().is_empty());
    assert!(store.to_send(true).unwrap().is_empty(), "never sent again");
    store.retry(id).unwrap();
    assert_eq!(
        going(&store, id),
        (Going::Accepted, Some(Filing::Waiting), None)
    );
    store.filed(id, Filed::Done).unwrap();
    assert!(store.outbox().unwrap().is_empty(), "filed, the record goes");
}

#[test]
fn accepted_without_a_copy_leaves_no_record() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let id = queue(&mut store);
    store.take_queued(id).unwrap().unwrap();
    store.landed(id, Landing::Accepted { file: false }).unwrap();
    assert!(store.outbox().unwrap().is_empty());
    assert!(matches!(
        store.landed(id, Landing::Failed("x".into())),
        Err(Error::Store(_))
    ));
}

#[test]
fn a_send_cut_off_is_unknown_after_a_restart_and_never_resent() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let id = queue(&mut store);
    store.take_queued(id).unwrap().unwrap();
    drop(store);
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(store.recover_outbox().unwrap(), 1);
    assert_eq!(going(&store, id), (Going::Unsure, None, None));
    assert!(store.to_send(true).unwrap().is_empty());
    assert_eq!(store.pending().unwrap(), 0);
    let kept = store.queued(id).unwrap();
    assert_eq!(kept.message_id, "id@example.com", "kept to check by");
}

#[test]
fn an_unreached_server_waits_for_a_retry_round() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let id = queue(&mut store);
    store.take_queued(id).unwrap().unwrap();
    store
        .landed(id, Landing::Waiting("offline".into()))
        .unwrap();
    let (state, _, error) = going(&store, id);
    assert_eq!((state, error.as_deref()), (Going::Queued, Some("offline")));
    assert!(
        store.to_send(false).unwrap().is_empty(),
        "not again at once"
    );
    assert_eq!(store.to_send(true).unwrap(), [id]);
    assert!(store.queued(id).unwrap().tried.is_some());
}

#[test]
fn cancelled_or_failed_goes_back_to_a_draft_with_its_files() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let id = queue(&mut store);
    assert!(matches!(store.forget(id), Err(Error::Invalid(_))), "queued");
    store.cancel(id).unwrap();
    assert_eq!(going(&store, id).0, Going::Cancelled);
    assert!(store.take_queued(id).unwrap().is_none(), "cancelled stays");
    assert!(matches!(store.cancel(id), Err(Error::Invalid(_))));
    store.retry(id).unwrap();
    store.take_queued(id).unwrap().unwrap();
    assert!(
        matches!(store.cancel(id), Err(Error::Invalid(_))),
        "sending"
    );
    assert!(
        matches!(store.unqueue(id), Err(Error::Invalid(_))),
        "sending"
    );
    store.landed(id, Landing::Failed("550 no".into())).unwrap();
    let draft = store.unqueue(id).unwrap();
    assert!(store.outbox().unwrap().is_empty());
    assert_eq!(store.draft(draft).unwrap().unwrap().unsent, unsent());
    let file = attachment("plan.pdf", b"%PDF".to_vec());
    assert_eq!(store.draft_files(draft).unwrap(), [file]);
}

#[test]
fn an_unknown_send_is_removed_only_when_asked() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let id = queue(&mut store);
    store.take_queued(id).unwrap().unwrap();
    store.landed(id, Landing::Unsure("lost".into())).unwrap();
    assert!(matches!(store.cancel(id), Err(Error::Invalid(_))));
    store.forget(id).unwrap();
    assert!(store.outbox().unwrap().is_empty());
}

#[test]
fn an_accounts_copy_choice_and_size_limit_are_kept() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    assert_eq!(store.sent_copy(ann).unwrap(), None, "follows the provider");
    assert_eq!(store.smtp_limit(ann).unwrap(), None);
    store.set_sent_copy(ann, Some(false)).unwrap();
    store.set_smtp_limit(ann, Some(25_000_000)).unwrap();
    drop(store);
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(store.sent_copy(ann).unwrap(), Some(false));
    assert_eq!(store.smtp_limit(ann).unwrap(), Some(25_000_000));
}

#[test]
fn the_sent_form_loses_only_its_hidden_copies() {
    let person = |email: &str| Mailbox {
        name: None,
        email: email.into(),
    };
    let built = build(&Draft {
        from: person("ann@example.com"),
        to: vec![person("bo@example.com")],
        cc: Vec::new(),
        bcc: vec![person("cy@example.com"), person("di@example.com")],
        subject: "Plan bcc marker".into(),
        text: "Bcc: not a header\r\n".into(),
        html: None,
        attachments: Vec::new(),
        in_reply_to: None,
        references: Vec::new(),
        date: 1_700_000_000,
    })
    .unwrap();
    let sent = without_bcc(&built.bytes).unwrap();
    let head = |bytes: &[u8]| {
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        text.split("\r\n\r\n").next().unwrap().to_owned()
    };
    assert!(head(&built.bytes).contains("cy@example.com"));
    assert!(!head(&sent).contains("cy@example.com"), "{}", head(&sent));
    assert!(!head(&sent).lines().any(|line| {
        line.split_once(':')
            .is_some_and(|(name, _)| name.eq_ignore_ascii_case("bcc"))
    }));
    let read = read(&sent).unwrap();
    assert_eq!(read.subject.as_deref(), Some("Plan bcc marker"));
    assert_eq!(read.message_id.as_deref(), Some(built.message_id.as_str()));
    assert_eq!(read.to, [person("bo@example.com")]);
    assert!(
        read.text.unwrap().contains("Bcc: not a header"),
        "the body stays"
    );
    let folded = b"From: a@x\r\nBCC: b@x,\r\n c@x\r\nTo: d@x\r\n\r\nhi\r\n";
    let unfolded = without_bcc(folded).unwrap();
    assert_eq!(unfolded, b"From: a@x\r\nTo: d@x\r\n\r\nhi\r\n");
}
