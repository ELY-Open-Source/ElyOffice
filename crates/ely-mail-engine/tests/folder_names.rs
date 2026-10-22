use std::{io::Cursor, sync::Mutex};

use ely_mail_engine::{Account, Change, Fetched, Key, Role, Secret, Security, Store, push::Dropped, sync};
use fake::{certs, expect, send, serve};

/// 草稿箱, as 163 lists it.
const DRAFTS: &str = "&g0l6P3ux-";
const MAIL: &[u8] = b"From: a@example.com\r\nSubject: Hi\r\n\r\nhello\r\n";

fn opened() -> (tempfile::TempDir, Store, ely_mail_engine::AccountId) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let id = store.add_account(&fake::account("ann@example.com")).unwrap();
    (dir, store, id)
}

#[tokio::test]
async fn a_folder_reads_in_chinese_and_is_asked_for_as_listed() {
    let (_dir, store, id) = opened();
    let store = Mutex::new(store);
    let certs = certs();
    let script = vec![
        send("* OK ready\n"),
        expect("{tag} LOGIN \"ann@example.com\" \"pw\""),
        send("{tag} OK done\n"),
        expect("{tag} CAPABILITY"),
        send("* CAPABILITY IMAP4rev1\n{tag} OK caps\n"),
        expect("{tag} LIST \"\" *"),
        send("* LIST (\\HasNoChildren \\Drafts) \"/\" \"&g0l6P3ux-\"\n{tag} OK listed\n"),
        expect("{tag} EXAMINE \"&g0l6P3ux-\""),
        send("* 0 EXISTS\n* OK [UIDVALIDITY 3] v\n{tag} OK in\n"),
        expect("{tag} LOGOUT"),
        send("* BYE\n{tag} OK bye\n"),
    ];
    let (incoming, task) = serve(&certs, Security::Tls, script).await;
    let account = Account {
        incoming,
        ..fake::account("ann@example.com")
    };
    let secret = Secret::Password("pw".into());
    sync::imap(&account, id, secret, &certs.trusted, &store, 1_792_411_200, &|| {})
        .await
        .unwrap();
    task.await.unwrap().unwrap();
    let folders = store.lock().unwrap().folders(id).unwrap();
    let folder = &folders[0];
    assert_eq!(
        (folder.name.as_str(), folder.shown.as_str(), folder.role),
        (DRAFTS, "草稿箱", Some(Role::Drafts))
    );
}

#[test]
fn a_vanished_folder_is_named_as_people_read_it() {
    let (_dir, mut store, id) = opened();
    let folders = [("INBOX", Some(Role::Inbox)), (DRAFTS, None)];
    let [inbox, drafts] = store.set_folders(id, &folders).unwrap().try_into().unwrap();
    store.open_mailbox(inbox, 7).unwrap();
    store.open_mailbox(drafts, 7).unwrap();
    let key = Key::Imap {
        uidvalidity: 7,
        uid: 1,
    };
    let message = store.put(inbox, &key, &[], Fetched::Whole(MAIL)).unwrap();
    store.change(&[message], Change::Move(drafts)).unwrap();
    store.set_folders(id, &[("INBOX", Some(Role::Inbox))]).unwrap();
    assert_eq!(store.take_dropped(id).unwrap(), [Dropped::Vanished("草稿箱".into())]);
}

#[test]
fn an_imported_folder_keeps_its_name() {
    let (_dir, mut store, id) = opened();
    let mbox = "From a@example.com Mon Jan  5 10:00:00 2026\nSubject: Hi\n\nhello\n";
    store.import_mbox(id, "旧邮件 & more", Cursor::new(mbox)).unwrap();
    let folders = store.folders(id).unwrap();
    assert_eq!(folders[0].shown, "旧邮件 & more");
}

#[test]
fn a_listing_in_no_modified_utf7_changes_nothing() {
    let (_dir, mut store, id) = opened();
    store.set_folders(id, &[("INBOX", Some(Role::Inbox)), ("Work", None)]).unwrap();
    let bad = [("INBOX", Some(Role::Inbox)), ("caf\u{e9}", None)];
    let err = store.set_folders(id, &bad).unwrap_err().to_string();
    assert!(err.contains("caf\u{e9}"), "{err}");
    let names = |store: &Store| -> Vec<String> { store.folders(id).unwrap().into_iter().map(|it| it.shown).collect() };
    assert_eq!(names(&store), ["INBOX", "Work"]);
    store.set_folders(id, &[("INBOX", Some(Role::Inbox))]).unwrap();
    assert_eq!(names(&store), ["INBOX"]);
}
