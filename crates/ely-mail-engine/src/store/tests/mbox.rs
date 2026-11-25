use std::io::Cursor;

use super::account;
use crate::store::{Change, Fetched, Filter, Key, Scope, Store, open, schema::STEPS};
use crate::{Error, Role};

const FILE: &[u8] = b"From ann@example.com Tue Jan  2 10:00:00 2024\n\
    From: Ann <ann@example.com>\nSubject: first\nDate: Tue, 2 Jan 2024 10:00:00 +0000\n\nhello\n>From the top\n\n\
    From bo@example.com Wed Jan  3 10:00:00 2024\n\
    From: bo@example.com\nSubject: second\nDate: Wed, 3 Jan 2024 10:00:00 +0000\n\nbye\n";

fn subjects(store: &Store, folder: crate::MailboxId) -> Vec<(String, bool)> {
    let list = store.list(Scope::Mailbox(folder), Filter::default(), &[]).unwrap();
    let mut seen: Vec<_> = list.into_iter().map(|each| (each.subject.unwrap(), each.seen)).collect();
    seen.sort();
    seen
}

#[test]
fn an_import_makes_a_folder_of_its_own_that_sync_leaves() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    store.set_folders(ann, &[("INBOX", Some(Role::Inbox)), ("Old", None)]).unwrap();
    let first = store.import_mbox(ann, " Old ", Cursor::new(FILE)).unwrap();
    assert_eq!(first.messages, 2);
    let again = store.import_mbox(ann, "Old", Cursor::new(FILE)).unwrap();
    let names: Vec<_> = store.folders(ann).unwrap().into_iter().map(|each| (each.name, each.local)).collect();
    assert_eq!(names, [("INBOX".into(), false), ("Old".into(), false), ("Old".into(), true), ("Old 2".into(), true)]);
    assert_eq!(subjects(&store, first.folder), [("first".into(), true), ("second".into(), true)]);
    // The server lists only INBOX now.
    store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap();
    let left: Vec<_> = store.folders(ann).unwrap().into_iter().map(|each| each.name).collect();
    assert_eq!(left, ["INBOX", "Old", "Old 2"]);
    assert_eq!(subjects(&store, again.folder).len(), 2);
}

#[test]
fn a_bad_file_leaves_no_folder() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let error = store.import_mbox(ann, "Loose", Cursor::new(&b"Subject: x\n\nno From line\n"[..])).unwrap_err();
    assert!(error.to_string().contains("no mbox From line"), "{error}");
    assert!(store.folders(ann).unwrap().is_empty());
}

#[test]
fn imported_mail_changes_only_here() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox] = store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
    let key = Key::Imap { uidvalidity: 1, uid: 1 };
    let remote = store.put(inbox, &key, &[], Fetched::Whole(b"Subject: s\r\n\r\nhi\r\n")).unwrap();
    let folder = store.import_mbox(ann, "Old", Cursor::new(FILE)).unwrap().folder;
    let ids: Vec<_> = store.list(Scope::Mailbox(folder), Filter::default(), &[]).unwrap().into_iter().map(|e| e.id).collect();
    assert!(matches!(store.change(&[remote], Change::Move(folder)), Err(Error::Invalid(_))));
    assert!(matches!(store.change(&ids[..1], Change::Move(inbox)), Err(Error::Invalid(_))));
    assert_eq!(store.change(&ids[..1], Change::Flagged(true)).unwrap(), [], "no server work");
    assert_eq!(store.forever(&ids).unwrap(), 2, "deleting it is for good");
    store.change(&ids[..1], Change::Delete { erase: true }).unwrap();
    assert_eq!(subjects(&store, folder).len(), 1);
    assert_eq!(store.changes_waiting(ann).unwrap(), 0);
}

#[test]
fn a_folder_exports_whole_messages_and_reads_back() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox] = store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
    let whole = b"From: cy@example.com\r\nSubject: kept\r\nDate: Mon, 1 Jan 2024 10:00:00 +0000\r\n\r\nFrom here\r\n";
    store.put(inbox, &Key::Imap { uidvalidity: 1, uid: 1 }, &[], Fetched::Whole(whole)).unwrap();
    let head = b"From: dee@example.com\r\nSubject: head\r\n\r\n";
    store.put(inbox, &Key::Imap { uidvalidity: 1, uid: 2 }, &[], Fetched::Headers(head)).unwrap();
    let missing = ["2".to_owned()];
    let parted = b"From: eve@example.com\r\nSubject: parted\r\n\r\nsee the file\r\n";
    let text = Fetched::Text { bytes: parted, missing: &missing };
    store.put(inbox, &Key::Imap { uidvalidity: 1, uid: 3 }, &[], text).unwrap();
    let file = dir.path().join("inbox.mbox");
    let packed = store.export_mbox(inbox, &file, false).unwrap();
    assert_eq!((packed.written, packed.left), (1, 2), "a missing file keeps it out too");
    let kept = dir.path().join("kept.mbox");
    std::fs::write(&kept, b"kept").unwrap();
    assert!(store.export_mbox(inbox, &kept, false).is_err(), "no replacing unasked");
    assert_eq!(std::fs::read(&kept).unwrap(), b"kept");
    store.export_mbox(inbox, &kept, true).unwrap();
    assert_ne!(std::fs::read(&kept).unwrap(), b"kept");
    let text = std::fs::read(&file).unwrap();
    assert!(text.starts_with(b"From cy@example.com Mon Jan  1 10:00:00 2024\n"));
    let back = store.import_mbox(ann, "Back", Cursor::new(text)).unwrap();
    let id = store.list(Scope::Mailbox(back.folder), Filter::default(), &[]).unwrap()[0].id;
    assert_eq!(store.original(id).unwrap().unwrap(), whole.to_vec());
}

#[test]
fn raw_binary_mail_and_gone_folders_never_export() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox, old] = store
        .set_folders(ann, &[("INBOX", Some(Role::Inbox)), ("Old", None)])
        .unwrap()
        .try_into()
        .unwrap();
    // Its bytes hold a bare LF and CR.
    let mut binary = b"From: cy@example.com\r\nSubject: raw\r\nContent-Type: application/octet-stream\r\n\
                       Content-Transfer-Encoding: binary\r\n\r\n"
        .to_vec();
    binary.extend_from_slice(b"A\nB\rC\0D");
    store.put(inbox, &Key::Imap { uidvalidity: 1, uid: 1 }, &[], Fetched::Whole(&binary)).unwrap();
    let plain = b"From: dee@example.com\r\nSubject: fine\r\n\r\nhi\r\n";
    store.put(inbox, &Key::Imap { uidvalidity: 1, uid: 2 }, &[], Fetched::Whole(plain)).unwrap();
    // No break at its end: an mbox would add one.
    let mut open = binary[..binary.len() - 7].to_vec();
    open.extend_from_slice(b"A\0B");
    store.put(inbox, &Key::Imap { uidvalidity: 1, uid: 3 }, &[], Fetched::Whole(&open)).unwrap();
    let file = dir.path().join("inbox.mbox");
    let packed = store.export_mbox(inbox, &file, false).unwrap();
    assert_eq!((packed.written, packed.left), (1, 2), "line breaks would change their bytes");
    store.put(old, &Key::Imap { uidvalidity: 1, uid: 1 }, &[], Fetched::Whole(plain)).unwrap();
    store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap();
    let backup = dir.path().join("old.mbox");
    std::fs::write(&backup, b"kept").unwrap();
    assert!(store.export_mbox(old, &backup, true).is_err(), "a gone folder is no empty one");
    assert_eq!(std::fs::read(&backup).unwrap(), b"kept");
}

#[test]
fn upgrading_keeps_mailbox_ids_unused() {
    let dir = tempfile::tempdir().unwrap();
    let store = open(dir.path(), &STEPS[..15]).unwrap();
    let ann = super::old_account(&store, "ann@example.com");
    drop(store);
    // A v15 store: mailbox 2 made, then removed.
    let db = rusqlite::Connection::open(dir.path().join(crate::store::DB)).unwrap();
    db.execute_batch(&format!(
        "INSERT INTO mailboxes (account, name, pinned) VALUES ({0}, 'INBOX', 1), ({0}, 'Gone', 0);
         DELETE FROM mailboxes WHERE name = 'Gone';",
        ann.0
    ))
    .unwrap();
    let gone: i64 = db.query_row("SELECT seq FROM sqlite_sequence WHERE name = 'mailboxes'", [], |row| row.get(0)).unwrap();
    drop(db);
    let mut store = open(dir.path(), STEPS).unwrap();
    assert_eq!(store.read_only(), None);
    let folders = store.folders(ann).unwrap();
    assert!(folders[0].pinned && !folders[0].local, "kept as it was");
    let new = store.import_mbox(ann, "New", Cursor::new(FILE)).unwrap().folder;
    assert!(new.0 > gone, "{} reuses a removed id", new.0);
}

#[test]
fn only_an_imported_folder_is_removed_with_its_mail() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox] = store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
    let folder = store.import_mbox(ann, "Old", Cursor::new(FILE)).unwrap().folder;
    assert!(matches!(store.remove_imported(inbox), Err(Error::Invalid(_))));
    store.remove_imported(folder).unwrap();
    let left: Vec<_> = store.folders(ann).unwrap().into_iter().map(|each| each.id).collect();
    assert_eq!(left, [inbox]);
    assert!(store.list(Scope::Inboxes, Filter::default(), &[]).unwrap().is_empty());
    let prefixes = std::fs::read_dir(dir.path().join(crate::store::CONTENT)).unwrap();
    let originals: usize = prefixes.map(|prefix| std::fs::read_dir(prefix.unwrap().path()).unwrap().count()).sum();
    assert_eq!(originals, 0, "the originals went with the folder");
}

#[test]
fn forgetting_downloads_keeps_what_no_server_holds() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox, kept] = store
        .set_folders(ann, &[("INBOX", Some(Role::Inbox)), ("Kept", None)])
        .unwrap()
        .try_into()
        .unwrap();
    store.set_offline(kept, true).unwrap();
    let whole: &[u8] = b"From: cy@example.com\r\nSubject: plan\r\n\r\nthe quarterly plan\r\n";
    let key = Key::Imap { uidvalidity: 1, uid: 1 };
    let forgot = store.put(inbox, &key, &["\\Seen"], Fetched::Whole(whole)).unwrap();
    let offline = store.put(kept, &key, &[], Fetched::Whole(b"Subject: offline\r\n\r\nkept\r\n")).unwrap();
    let imported = store.import_mbox(ann, "Old", Cursor::new(FILE)).unwrap().folder;
    // Moved offline here; the server has not heard yet.
    let travel: &[u8] = b"Subject: travel\r\n\r\non its way\r\n";
    let moving = store.put(inbox, &Key::Imap { uidvalidity: 1, uid: 2 }, &[], Fetched::Whole(travel)).unwrap();
    store.change(&[moving], Change::Move(kept)).unwrap();
    assert_eq!(store.forget_downloads().unwrap(), 1);
    assert!(store.original(moving).unwrap().is_some(), "moving offline, it is kept");
    assert_eq!(store.original(forgot).unwrap(), None);
    let stored = store.get(forgot).unwrap();
    assert_eq!((stored.subject.as_deref(), stored.flags.as_slice()), (Some("plan"), &["\\Seen".to_owned()][..]));
    assert!(store.original(offline).unwrap().is_some(), "an offline folder keeps its mail");
    let local = store.list(Scope::Mailbox(imported), Filter::default(), &[]).unwrap();
    assert!(local.iter().all(|each| store.original(each.id).unwrap().is_some()), "no server holds imports");
    let words = crate::store::Search { words: "quarterly".into(), ..Default::default() };
    assert!(store.search(&words).unwrap().messages.is_empty(), "its words left the index");
    // The next sync brings it back.
    store.put(inbox, &key, &["\\Seen"], Fetched::Whole(whole)).unwrap();
    assert_eq!(store.original(forgot).unwrap().unwrap(), whole.to_vec());
}

/// Reads `bytes`, then fails.
struct Breaking<'a>(&'a [u8]);

impl std::io::Read for Breaking<'_> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        match self.0.is_empty() {
            true => Err(std::io::Error::other("the disk went away")),
            false => self.0.read(out),
        }
    }
}

#[test]
fn a_read_failure_names_the_message_and_leaves_no_folder() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let before = store.folders(ann).unwrap().len();
    let cut = FILE.windows(5).rposition(|at| at == b"\n\nbye").unwrap();
    let input = std::io::BufReader::new(Breaking(&FILE[..cut]));
    let failed = store.import_mbox(ann, "Broken", input).unwrap_err().to_string();
    assert!(failed.contains("message 2"), "{failed}");
    assert_eq!(store.folders(ann).unwrap().len(), before);
}
