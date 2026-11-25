//! The v12 upgrade indexes held mail again.

use std::fs;

use rusqlite::Connection;

use super::version;
use crate::Role;
use crate::store::{CONTENT, DB, Fetched, Field, Key, MessageId, Search, open, schema::STEPS};

const BODY: &[u8] =
    "From: a@x.com\r\nSubject: s\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n明天开会议\r\n"
        .as_bytes();

/// A v11 store: one message, the old index.
fn at_v11(dir: &std::path::Path, body: &[u8]) -> MessageId {
    let mut store = open(dir, &STEPS[..12]).unwrap();
    let id = super::old_account(&store, "ann@example.com");
    let [inbox] = super::old_folders(&store, id, &[("INBOX", Some(Role::Inbox))])
        .try_into()
        .unwrap();
    store.conn.execute("UPDATE mailboxes SET uidvalidity = 1 WHERE id = ?1", [inbox.0]).unwrap();
    let key = Key::Imap {
        uidvalidity: 1,
        uid: 1,
    };
    let message = super::old_put(&mut store, inbox, &key, Fetched::Whole(body));
    drop(store);
    Connection::open(dir.join(DB))
        .unwrap()
        .execute_batch(
            "DROP TABLE search;
             CREATE VIRTUAL TABLE search USING fts5(
                 subject, people, body, files,
                 content = '', contentless_delete = 1, tokenize = 'trigram');
             PRAGMA user_version = 11;",
        )
        .unwrap();
    message
}

fn meeting() -> Search {
    Search {
        words: "会议".into(),
        field: Field::Body,
        ..Search::default()
    }
}

#[test]
fn upgrading_indexes_every_message_again() {
    let dir = tempfile::tempdir().unwrap();
    let message = at_v11(dir.path(), BODY);
    let store = open(dir.path(), STEPS).unwrap();
    assert_eq!(store.read_only(), None);
    let found = store.search(&meeting()).unwrap();
    assert_eq!(
        found
            .messages
            .iter()
            .map(|each| each.id)
            .collect::<Vec<_>>(),
        [message]
    );
}

#[test]
fn an_unreadable_original_keeps_the_old_store() {
    let dir = tempfile::tempdir().unwrap();
    at_v11(dir.path(), BODY);
    fs::remove_dir_all(dir.path().join(CONTENT)).unwrap();
    let store = open(dir.path(), STEPS).unwrap();
    assert!(store.read_only().is_some());
    assert_eq!(version(&dir.path().join(DB)), 11);
}

#[test]
fn an_original_parked_by_a_crash_returns_before_reindexing() {
    let dir = tempfile::tempdir().unwrap();
    let message = at_v11(dir.path(), BODY);
    let shard = fs::read_dir(dir.path().join(CONTENT))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let file = fs::read_dir(&shard)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let name = file.file_name().unwrap().to_str().unwrap().to_owned();
    fs::rename(&file, shard.join(format!(".{name}.gone"))).unwrap();
    let store = open(dir.path(), STEPS).unwrap();
    assert_eq!(store.read_only(), None);
    assert!(file.exists());
    let found = store.search(&meeting()).unwrap();
    assert_eq!(
        found
            .messages
            .iter()
            .map(|each| each.id)
            .collect::<Vec<_>>(),
        [message]
    );
}

const MIXED: &[u8] = b"From: a@x.com\r\nSubject: s\r\n\
    Content-Type: multipart/mixed; boundary=b\r\n\r\n\
    --b\r\nContent-Type: text/plain\r\n\r\nFirst section\r\n\
    --b\r\nContent-Type: text/html\r\n\r\n<p>The meeting is tomorrow</p>\r\n--b--\r\n";

#[test]
fn upgrading_indexes_html_beside_plain_text() {
    let dir = tempfile::tempdir().unwrap();
    let message = at_v11(dir.path(), MIXED);
    let store = open(dir.path(), STEPS).unwrap();
    let search = Search {
        words: "meeting".into(),
        field: Field::Body,
        ..Search::default()
    };
    let found = store.search(&search).unwrap();
    assert_eq!(
        found
            .messages
            .iter()
            .map(|each| each.id)
            .collect::<Vec<_>>(),
        [message]
    );
}
