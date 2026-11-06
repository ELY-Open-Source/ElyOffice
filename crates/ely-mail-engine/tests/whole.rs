use std::sync::Mutex;

use ely_mail_engine::{Error, Fetched, Key, MessageId, Role, Secret, Store, whole::whole};
use fake::{account, certs, expect, send, serve};

const WHOLE: &str = "Subject: Plan\nFrom: b@example.com\nContent-Type: multipart/mixed; boundary=b\n\n\
--b\nContent-Type: text/plain\n\nsee the plan\n--b\nContent-Type: application/pdf\n\
Content-Disposition: attachment; filename=plan.pdf\n\n%PDF-1.7 plan\n--b--\n";

/// The message as a sync keeps it, without its file.
fn partial() -> String {
    WHOLE.replace("%PDF-1.7 plan\n", "")
}

/// A store with INBOX message 9, its part 2 missing.
fn kept() -> (tempfile::TempDir, Mutex<Store>, ely_mail_engine::AccountId, MessageId) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let id = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox] = store
        .set_folders(id, &[("INBOX", Some(Role::Inbox))])
        .unwrap()
        .try_into()
        .unwrap();
    let key = Key::Imap { uidvalidity: 7, uid: 9 };
    let bytes = partial().replace('\n', "\r\n");
    let missing = ["2".to_owned()];
    let message = store
        .put(inbox, &key, &[], Fetched::Text { bytes: bytes.as_bytes(), missing: &missing })
        .unwrap();
    (dir, Mutex::new(store), id, message)
}

fn literal(text: &str) -> String {
    let crlf = text.replace('\n', "\r\n");
    format!("* 3 FETCH (UID 9 FLAGS (\\Seen) BODY[] {{{}}}\n{text})\n", crlf.len())
}

#[tokio::test]
async fn a_left_out_file_comes_down_with_its_message() {
    let (_dir, store, id, message) = kept();
    assert_eq!(store.lock().unwrap().get(message).unwrap().missing, ["2"]);
    let certs = certs();
    let script = vec![
        send("* OK ready\n"),
        expect("{tag} LOGIN \"ann@example.com\" \"pw\""),
        send("{tag} OK done\n"),
        expect("{tag} CAPABILITY"),
        send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
        expect("{tag} EXAMINE \"INBOX\""),
        send("* 3 EXISTS\n* OK [UIDVALIDITY 7] v\n{tag} OK in\n"),
        expect("{tag} UID FETCH 9 (UID FLAGS BODY.PEEK[])"),
        send(&literal(WHOLE)),
        send("{tag} OK bodies\n"),
        expect("{tag} LOGOUT"),
        send("* BYE\n{tag} OK bye\n"),
    ];
    let (incoming, task) = serve(&certs, ely_mail_engine::Security::Tls, script).await;
    let account = ely_mail_engine::Account { incoming, ..account("ann@example.com") };
    let secret = Secret::Password("pw".into());
    whole(&account, id, secret, &certs.trusted, &store, message).await.unwrap();
    task.await.unwrap().unwrap();
    let store = store.lock().unwrap();
    let stored = store.get(message).unwrap();
    assert!(stored.missing.is_empty(), "nothing waits now");
    assert_eq!(stored.flags, ["\\Seen"]);
    let mail = ely_mail_engine::read(&store.original(message).unwrap().unwrap()).unwrap();
    assert_eq!(mail.attachments[0].bytes, b"%PDF-1.7 plan");
}

#[tokio::test]
async fn a_changed_folder_keeps_the_message_as_it_was() {
    let (_dir, store, id, message) = kept();
    let certs = certs();
    let script = vec![
        send("* OK ready\n"),
        expect("{tag} LOGIN \"ann@example.com\" \"pw\""),
        send("{tag} OK done\n"),
        expect("{tag} CAPABILITY"),
        send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
        expect("{tag} EXAMINE \"INBOX\""),
        send("* 3 EXISTS\n* OK [UIDVALIDITY 8] v\n{tag} OK in\n"),
    ];
    let (incoming, _task) = serve(&certs, ely_mail_engine::Security::Tls, script).await;
    let account = ely_mail_engine::Account { incoming, ..account("ann@example.com") };
    let secret = Secret::Password("pw".into());
    let failed = whole(&account, id, secret, &certs.trusted, &store, message).await;
    assert!(matches!(failed, Err(Error::Server(ref said)) if said.contains("changed")), "{failed:?}");
    assert_eq!(store.lock().unwrap().get(message).unwrap().missing, ["2"]);
}

/// A download whose message `change` replaces while FETCH runs.
async fn changed_during(change: fn(&mut Store, MessageId)) -> (Result<(), Error>, Vec<MessageId>) {
    let (_dir, store, id, message) = kept();
    let store = std::sync::Arc::new(store);
    let inbox = store.lock().unwrap().get(message).unwrap().mailbox;
    let during = store.clone();
    let certs = certs();
    let script = vec![
        send("* OK ready\n"),
        expect("{tag} LOGIN \"ann@example.com\" \"pw\""),
        send("{tag} OK done\n"),
        expect("{tag} CAPABILITY"),
        send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
        expect("{tag} EXAMINE \"INBOX\""),
        send("* 3 EXISTS\n* OK [UIDVALIDITY 7] v\n{tag} OK in\n"),
        expect("{tag} UID FETCH 9 (UID FLAGS BODY.PEEK[])"),
        fake::Step::Run(Box::new(move || change(&mut during.lock().unwrap(), message))),
        send(&literal(WHOLE)),
        send("{tag} OK bodies\n"),
        expect("{tag} LOGOUT"),
        send("* BYE\n{tag} OK bye\n"),
    ];
    let (incoming, task) = serve(&certs, ely_mail_engine::Security::Tls, script).await;
    let account = ely_mail_engine::Account { incoming, ..account("ann@example.com") };
    let secret = Secret::Password("pw".into());
    let done = whole(&account, id, secret, &certs.trusted, &store, message).await;
    task.await.unwrap().unwrap();
    let left = store.lock().unwrap().open_mailbox(inbox, 7).unwrap().values().map(|held| held.id).collect();
    (done, left)
}

#[tokio::test]
async fn a_message_removed_meanwhile_stays_removed() {
    let (done, left) = changed_during(|store, message| store.remove(message).unwrap()).await;
    assert!(matches!(done, Err(Error::Deleted)), "{done:?}");
    assert!(left.is_empty(), "nothing came back: {left:?}");
}

#[tokio::test]
async fn a_folder_rebuilt_meanwhile_gets_no_old_message() {
    let rebuild = |store: &mut Store, message: MessageId| {
        let inbox = store.get(message).unwrap().mailbox;
        store.open_mailbox(inbox, 8).unwrap();
    };
    let (done, left) = changed_during(rebuild).await;
    assert!(matches!(done, Err(Error::Deleted)), "{done:?}");
    assert!(left.is_empty(), "nothing came back: {left:?}");
}
