use std::fs;

use rusqlite::Connection;
use sha2::{Digest, Sha256};

use super::folders::Synced;
use super::{BACKUPS, CONTENT, DB, open, schema::STEPS};
use crate::{Account, Auth, Error, Protocol, Role, Security, Server};

fn account(address: &str) -> Account {
    let server = Server {
        host: "127.0.0.1".into(),
        port: 993,
        security: Security::Tls,
    };
    Account {
        address: address.into(),
        user: address.into(),
        auth: Auth::Password,
        protocol: Protocol::Imap,
        incoming: server.clone(),
        outgoing: server,
    }
}

/// Folders in a store left at an older version.
fn old_folders(store: &super::Store, account: crate::AccountId, folders: &[(&str, Option<Role>)]) -> Vec<crate::MailboxId> {
    let insert = "INSERT INTO mailboxes (account, name, role) VALUES (?1, ?2, ?3)";
    let made = |(name, role): &(&str, Option<Role>)| {
        store.conn.execute(insert, rusqlite::params![account.0, name, role.map(Role::name)]).unwrap();
        crate::MailboxId(store.conn.last_insert_rowid())
    };
    folders.iter().map(made).collect()
}

/// Puts mail the way a pre-v18 store did.
fn old_put(store: &mut super::Store, mailbox: crate::MailboxId, key: &crate::Key, fetched: crate::Fetched) -> crate::MessageId {
    store.conn.execute_batch("ALTER TABLE messages ADD COLUMN ident TEXT").unwrap();
    let id = store.put(mailbox, key, &[], fetched).unwrap();
    store.conn.execute_batch("ALTER TABLE messages DROP COLUMN ident").unwrap();
    id
}

/// An original on disk for a row written as SQL.
fn held(dir: &std::path::Path) -> String {
    let bytes = b"Subject: kept\r\n\r\nhi\r\n";
    let hash: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let folder = dir.join(CONTENT).join(&hash[..2]);
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join(&hash), bytes).unwrap();
    hash
}

mod contacts;
mod ids;
mod listing;
mod mbox;
mod names;
mod rebuilt;
mod reindex;
mod rules;
mod side;
mod threads;
mod trim;

const ADD: &str = "ALTER TABLE accounts ADD COLUMN nick TEXT";
const BROKEN: &str = "ALTER TABLE nowhere ADD COLUMN nick TEXT";

fn version(path: &std::path::Path) -> i64 {
    Connection::open(path)
        .unwrap()
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap()
}

fn backups(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    match fs::read_dir(dir.join(BACKUPS)) {
        Ok(entries) => entries.map(|entry| entry.unwrap().path()).collect(),
        Err(_) => Vec::new(),
    }
}

#[test]
fn a_new_store_is_at_the_last_schema() {
    let dir = tempfile::tempdir().unwrap();
    let store = open(dir.path(), STEPS).unwrap();
    assert_eq!(store.read_only(), None);
    assert_eq!(version(&dir.path().join(DB)), STEPS.len() as i64);
    assert!(backups(dir.path()).is_empty());
}

#[test]
fn an_upgrade_backs_up_first() {
    let dir = tempfile::tempdir().unwrap();
    open(dir.path(), &[STEPS[0]])
        .unwrap()
        .add_account(&account("ann@example.com"))
        .unwrap();
    let store = open(dir.path(), &[STEPS[0], ADD]).unwrap();
    assert_eq!(store.read_only(), None);
    assert_eq!(version(&dir.path().join(DB)), 2);
    let [backup] = backups(dir.path()).try_into().unwrap();
    assert_eq!(version(&backup), 1);
    let accounts: i64 = Connection::open(&backup)
        .unwrap()
        .query_row("SELECT count(*) FROM accounts", [], |row| row.get(0))
        .unwrap();
    assert_eq!(accounts, 1);
}

#[test]
fn a_failed_upgrade_keeps_the_data_and_refuses_writes() {
    let dir = tempfile::tempdir().unwrap();
    open(dir.path(), &[STEPS[0]])
        .unwrap()
        .add_account(&account("ann@example.com"))
        .unwrap();
    let store = open(dir.path(), &[STEPS[0], ADD, BROKEN]).unwrap();
    let reason = store.read_only().unwrap();
    assert!(reason.contains("v1 to v3"), "{reason}");
    assert_eq!(store.accounts().unwrap()[0].1.address, "ann@example.com");
    let err = store.add_account(&account("bo@example.com")).unwrap_err();
    assert!(matches!(err, Error::ReadOnly(_)), "{err}");
    drop(store);
    assert_eq!(version(&dir.path().join(DB)), 1);
    assert_eq!(backups(dir.path()).len(), 1);
    let partial = Connection::open(dir.path().join(DB))
        .unwrap()
        .prepare("SELECT nick FROM accounts")
        .map(drop);
    assert!(partial.is_err(), "the first step was not rolled back");
}

#[test]
fn a_newer_schema_is_read_only() {
    let dir = tempfile::tempdir().unwrap();
    drop(open(dir.path(), &[STEPS[0], ADD]).unwrap());
    let store = open(dir.path(), &[STEPS[0]]).unwrap();
    let reason = store.read_only().unwrap();
    assert!(reason.contains("newer"), "{reason}");
    assert!(store.accounts().unwrap().is_empty());
}

#[test]
fn no_backup_means_no_upgrade() {
    let dir = tempfile::tempdir().unwrap();
    drop(open(dir.path(), &[STEPS[0]]).unwrap());
    fs::write(dir.path().join(BACKUPS), b"a file in the way").unwrap();
    let store = open(dir.path(), &[STEPS[0], ADD]).unwrap();
    let reason = store.read_only().unwrap();
    assert!(reason.contains("no backup"), "{reason}");
    drop(store);
    assert_eq!(version(&dir.path().join(DB)), 1);
}

#[test]
fn a_v1_store_gains_sync_and_keeps_its_mail() {
    let dir = tempfile::tempdir().unwrap();
    let old = open(dir.path(), &[STEPS[0]]).unwrap();
    let ann = old.add_account(&account("ann@example.com")).unwrap();
    old.conn
        .execute_batch(&format!(
            "INSERT INTO mailboxes (account, name) VALUES ({}, 'INBOX');
             INSERT INTO messages (mailbox, uidvalidity, uid, in_reply_to, refs,
                 sender, recipients, copies, content) VALUES
                 (1, 7, 1, '[]', '[]', '[]', '[]', '[]', '{}'),
                 (1, 6, 2, '[]', '[]', '[]', '[]', '[]', NULL);",
            ann.0,
            held(dir.path())
        ))
        .unwrap();
    drop(old);
    let mut store = open(dir.path(), STEPS).unwrap();
    assert_eq!(store.read_only(), None);
    assert_eq!(version(&dir.path().join(DB)), STEPS.len() as i64);
    assert_eq!(backups(dir.path()).len(), 1);
    // v6 drops the headers-only row; sync fetches it again.
    let synced = Synced {
        at: None,
        messages: 1,
        uncached: 0,
        partial: 0,
        waiting: 0,
    };
    assert_eq!(store.synced(ann).unwrap(), synced);
    store
        .set_folders(ann, &[("INBOX", Some(Role::Inbox))])
        .unwrap();
    let [inbox] = store.folders(ann).unwrap().try_into().unwrap();
    assert_eq!(
        (inbox.name.as_str(), inbox.role),
        ("INBOX", Some(Role::Inbox))
    );
    assert!(!inbox.offline);
    let held = store.open_mailbox(inbox.id, 7).unwrap();
    assert_eq!(held.keys().copied().collect::<Vec<_>>(), [1]);
    assert!(held[&1].body);
    store.set_synced(ann, 9).unwrap();
    assert_eq!(store.synced(ann).unwrap().at, Some(9));
    assert_eq!(store.synced(ann).unwrap().messages, 1);
}

#[test]
fn an_upgrade_leaving_a_row_without_its_parent_fails() {
    let dir = tempfile::tempdir().unwrap();
    let old = open(dir.path(), &[STEPS[0]]).unwrap();
    old.conn
        .execute_batch(
            "PRAGMA foreign_keys = OFF;
             INSERT INTO mailboxes (account, name) VALUES (99, 'INBOX');",
        )
        .unwrap();
    drop(old);
    let store = open(dir.path(), STEPS).unwrap();
    let reason = store.read_only().unwrap();
    assert!(reason.contains("missing rows"), "{reason}");
    drop(store);
    assert_eq!(version(&dir.path().join(DB)), 1);
}
