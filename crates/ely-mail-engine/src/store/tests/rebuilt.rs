use super::account;
use crate::{Action, Fetched, Key, Match, Role, Store};

/// Unnamed mail: no Message-ID, so headers and body tell.
fn bare(body: &str) -> Vec<u8> {
    format!(
        "From: bo@example.com\r\nTo: ann@example.com\r\nSubject: urgent\r\n\
         Date: Tue, 2 Jan 2024 10:00:00 +0000\r\nContent-Type: text/plain\r\n\r\n{body}\r\n"
    )
    .into_bytes()
}

fn head(bytes: &[u8]) -> &[u8] {
    let end = bytes.windows(4).position(|four| four == b"\r\n\r\n").unwrap();
    &bytes[..end + 4]
}

#[derive(Clone, Copy, Debug)]
enum Kept {
    Head,
    Whole,
    Partial,
    Trimmed,
}

fn put(store: &mut Store, inbox: crate::MailboxId, uidvalidity: u32, kept: Kept) -> crate::MessageId {
    let whole = bare("hi");
    let partial = bare("hi, without the file");
    let missing = ["2".to_owned()];
    let fetched = match kept {
        Kept::Head => Fetched::Headers(head(&whole)),
        Kept::Whole | Kept::Trimmed => Fetched::Whole(&whole),
        Kept::Partial => Fetched::Text { bytes: &partial, missing: &missing },
    };
    store.put(inbox, &Key::Imap { uidvalidity, uid: 1 }, &[], fetched).unwrap()
}

#[test]
fn a_rebuild_knows_unnamed_mail_however_it_was_kept() {
    use Kept::*;
    let turns = [(Whole, Head), (Head, Whole), (Trimmed, Whole), (Partial, Whole), (Whole, Partial)];
    for (before, after) in turns {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        let ann = store.add_account(&account("ann@example.com")).unwrap();
        let [inbox] = store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
        store.skip_arrivals(ann).unwrap();
        store.open_mailbox(inbox, 1).unwrap();
        put(&mut store, inbox, 1, before);
        assert_eq!(store.handle_arrivals(ann).unwrap().arrived.len(), 1, "{before:?} is news once");
        if let Trimmed = before {
            assert_eq!(store.forget_downloads().unwrap(), 1);
        }
        store.add_rule(ann, Match::Subject, "urgent", Action::Flag).unwrap();
        store.open_mailbox(inbox, 2).unwrap();
        put(&mut store, inbox, 2, after);
        let again = store.handle_arrivals(ann).unwrap();
        assert!(again.arrived.is_empty(), "{before:?} back as {after:?} is no news");
        assert_eq!(again.ruled, 0, "{before:?} back as {after:?} meets no new rule");
    }
}

#[test]
fn upgrading_gives_old_mail_the_identity_new_mail_gets() {
    use crate::store::{open, schema::STEPS};
    let dir = tempfile::tempdir().unwrap();
    let whole = bare("hi");
    let named = b"Message-ID: <a@example.com>\r\nSubject: s\r\n\r\n".to_vec();
    let key = |uid| Key::Imap { uidvalidity: 1, uid };
    let mut old = open(dir.path(), &STEPS[..17]).unwrap();
    let ann = old.add_account(&account("ann@example.com")).unwrap();
    let [inbox] = super::old_folders(&old, ann, &[("INBOX", Some(Role::Inbox))]).try_into().unwrap();
    old.conn.execute("UPDATE mailboxes SET uidvalidity = 1 WHERE id = ?1", [inbox.0]).unwrap();
    let kept = super::old_put(&mut old, inbox, &key(1), Fetched::Whole(&whole));
    let headed = super::old_put(&mut old, inbox, &key(2), Fetched::Headers(&named));
    drop(old);
    let mut store = Store::open(dir.path()).unwrap();
    let ident = |store: &Store, id: crate::MessageId| -> String {
        store.conn.query_row("SELECT ident FROM messages WHERE id = ?1", [id.0], |row| row.get(0)).unwrap()
    };
    assert_eq!(ident(&store, headed), "a@example.com");
    let fresh = store.put(inbox, &key(3), &[], Fetched::Headers(head(&whole))).unwrap();
    assert!(ident(&store, kept).starts_with("unnamed:"));
    assert_eq!(ident(&store, kept), ident(&store, fresh), "a whole original and its headers agree");
}

#[test]
fn mail_moved_to_the_inbox_without_a_uid_is_no_news() {
    use crate::store::{Change, Done, Settled};
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let folders = [("INBOX", Some(Role::Inbox)), ("Work", None)];
    let [inbox, work] = store.set_folders(ann, &folders).unwrap().try_into().unwrap();
    store.skip_arrivals(ann).unwrap();
    store.open_mailbox(work, 1).unwrap();
    store.open_mailbox(inbox, 1).unwrap();
    let id = store.put(work, &Key::Imap { uidvalidity: 1, uid: 1 }, &[], Fetched::Whole(&bare("hi"))).unwrap();
    assert!(store.handle_arrivals(ann).unwrap().arrived.is_empty());
    store.add_rule(ann, Match::Subject, "urgent", Action::Flag).unwrap();
    store.change(&[id], Change::Move(inbox)).unwrap();
    let [pending] = store.queued_changes(ann).unwrap().try_into().unwrap();
    let settled = Settled { done: Done::Moved(None), ..Settled::waiting() };
    store.pushed(ann, &pending, &settled).unwrap();
    // The inbox's sync brings it down anew.
    store.put(inbox, &Key::Imap { uidvalidity: 1, uid: 5 }, &[], Fetched::Whole(&bare("hi"))).unwrap();
    let again = store.handle_arrivals(ann).unwrap();
    assert!(again.arrived.is_empty(), "a move is no news");
    assert_eq!(again.ruled, 0, "nor meets a rule again");
}
