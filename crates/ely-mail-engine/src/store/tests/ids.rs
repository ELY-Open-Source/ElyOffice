//! Ids are never reused, so a stale one names nothing.

use super::{STEPS, account, held, open};
use crate::{Fetched, Key, MessageId, Role, Store};

const MAIL: &[u8] = b"From: a@example.com\r\nSubject: hi\r\n\r\nhi\r\n";

fn imap(uidvalidity: u32) -> Key {
    Key::Imap {
        uidvalidity,
        uid: 1,
    }
}

#[test]
fn a_removed_account_folder_or_message_leaves_its_id_unused() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let bob = store.add_account(&account("bob@example.com")).unwrap();
    store.remove_account(bob).unwrap();
    let cy = store.add_account(&account("cy@example.com")).unwrap();
    assert!(cy.0 > bob.0, "account {} came back", bob.0);
    let [_, old] = store
        .set_folders(ann, &[("INBOX", Some(Role::Inbox)), ("Old", None)])
        .unwrap()
        .try_into()
        .unwrap();
    store
        .set_folders(ann, &[("INBOX", Some(Role::Inbox))])
        .unwrap();
    let [inbox, new] = store
        .set_folders(ann, &[("INBOX", Some(Role::Inbox)), ("New", None)])
        .unwrap()
        .try_into()
        .unwrap();
    assert!(new.0 > old.0, "folder {} came back", old.0);
    store.open_mailbox(inbox, 1).unwrap();
    let first = store
        .put(inbox, &imap(1), &[], Fetched::Whole(MAIL))
        .unwrap();
    store.open_mailbox(inbox, 2).unwrap();
    let second = store
        .put(inbox, &imap(2), &[], Fetched::Whole(MAIL))
        .unwrap();
    assert!(second.0 > first.0, "message {} came back", first.0);
}

#[test]
fn a_v3_store_keeps_its_ids_and_stops_reusing_them() {
    let dir = tempfile::tempdir().unwrap();
    let old = open(dir.path(), &STEPS[..3]).unwrap();
    let ann = super::old_account(&old, "ann@example.com");
    let [inbox] = super::old_folders(&old, ann, &[("INBOX", Some(Role::Inbox))])
        .try_into()
        .unwrap();
    old.conn
        .execute("UPDATE mailboxes SET uidvalidity = 1", [])
        .unwrap();
    // Today's writes name later columns; insert as SQL.
    old.conn
        .execute(
            "INSERT INTO messages (mailbox, uidvalidity, uid, in_reply_to, refs, sender,
                 recipients, copies, content) VALUES (?1, 1, 1, '[]', '[]', '[]', '[]', '[]', ?2)",
            rusqlite::params![inbox.0, held(dir.path())],
        )
        .unwrap();
    let first = MessageId(old.conn.last_insert_rowid());
    drop(old);
    let mut store = open(dir.path(), STEPS).unwrap();
    assert_eq!(store.read_only(), None);
    assert_eq!(store.accounts().unwrap()[0].0, ann);
    let held = store.open_mailbox(inbox, 1).unwrap();
    assert_eq!(held[&1].id, first);
    store.open_mailbox(inbox, 2).unwrap();
    let second = store
        .put(inbox, &imap(2), &[], Fetched::Whole(MAIL))
        .unwrap();
    assert!(second.0 > first.0, "message {} came back", first.0);
}

#[test]
fn a_flagged_headers_only_v5_message_upgrades_and_stays_writable() {
    let dir = tempfile::tempdir().unwrap();
    let old = open(dir.path(), &STEPS[..5]).unwrap();
    let ann = super::old_account(&old, "ann@example.com");
    let [inbox] = super::old_folders(&old, ann, &[("INBOX", Some(Role::Inbox))])
        .try_into()
        .unwrap();
    old.conn
        .execute("UPDATE mailboxes SET uidvalidity = 1", [])
        .unwrap();
    old.conn
        .execute(
            "INSERT INTO messages (mailbox, uidvalidity, uid, in_reply_to, refs, sender,
                 recipients, copies) VALUES (?1, 1, 1, '[]', '[]', '[]', '[]', '[]')",
            [inbox.0],
        )
        .unwrap();
    let gone = old.conn.last_insert_rowid();
    old.conn
        .execute(
            r"INSERT INTO flags (message, flag) VALUES (?1, '\Seen')",
            [gone],
        )
        .unwrap();
    drop(old);
    let mut store = open(dir.path(), STEPS).unwrap();
    assert_eq!(store.read_only(), None);
    assert!(store.open_mailbox(inbox, 1).unwrap().is_empty());
    let again = store
        .put(inbox, &imap(1), &[], Fetched::Whole(MAIL))
        .unwrap();
    assert_eq!(store.find(inbox, &imap(1)).unwrap(), Some(again));
}
