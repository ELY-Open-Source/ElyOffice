use std::collections::HashSet;

use ely_mail_engine::{
    AccountId, Auth, Change, Error, Fetched, Filter, Key, MailboxId, MessageId, Protocol, Role,
    Scope, Store,
};

const MAIL: &[u8] = b"From: a@example.com\r\nSubject: Hi\r\n\r\nhello\r\n";

struct Boxes {
    inbox: MailboxId,
    trash: MailboxId,
    archive: MailboxId,
    work: MailboxId,
}

/// Each folder opened at UIDVALIDITY 1.
fn imap_store(
    folders: &[(&str, Option<Role>)],
) -> (tempfile::TempDir, Store, AccountId, Vec<MailboxId>) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let id = store
        .add_account(&fake::account("ann@example.com"))
        .unwrap();
    let ids = store.set_folders(id, folders).unwrap();
    for &mailbox in &ids {
        store.open_mailbox(mailbox, 1).unwrap();
    }
    (dir, store, id, ids)
}

fn standard() -> (tempfile::TempDir, Store, AccountId, Boxes) {
    let (dir, store, id, ids) = imap_store(&[
        ("INBOX", Some(Role::Inbox)),
        ("Trash", Some(Role::Trash)),
        ("Archive", Some(Role::Archive)),
        ("Work", None),
    ]);
    let [inbox, trash, archive, work] = ids.try_into().unwrap();
    let boxes = Boxes {
        inbox,
        trash,
        archive,
        work,
    };
    (dir, store, id, boxes)
}

fn put(store: &mut Store, mailbox: MailboxId, uid: u32, flags: &[&str]) -> MessageId {
    let key = Key::Imap {
        uidvalidity: 1,
        uid,
    };
    store
        .put(mailbox, &key, flags, Fetched::Whole(MAIL))
        .unwrap()
}

fn listed(store: &Store, mailbox: MailboxId) -> Vec<MessageId> {
    let list = store.list(Scope::Mailbox(mailbox), Filter::default(), &[]);
    list.unwrap().into_iter().map(|each| each.id).collect()
}

#[test]
fn reading_and_flagging_show_now_and_wait_for_the_server() {
    let (_dir, mut store, id, boxes) = standard();
    let one = put(&mut store, boxes.inbox, 1, &["\\SEEN"]);
    let two = put(&mut store, boxes.inbox, 2, &[]);
    assert_eq!(store.change(&[two], Change::Seen(true)).unwrap(), [id]);
    assert_eq!(store.change(&[one], Change::Seen(false)).unwrap(), [id]);
    store.change(&[two], Change::Flagged(true)).unwrap();
    assert_eq!(store.get(one).unwrap().flags, Vec::<String>::new());
    assert_eq!(store.get(two).unwrap().flags, ["\\Flagged", "\\Seen"]);
    assert_eq!(store.unread().unwrap()[&boxes.inbox], 1);
    // One queued change per message.
    assert_eq!(store.synced(id).unwrap().waiting, 2);
    // The server's older flags keep queued changes.
    store.set_flags(two, &[]).unwrap();
    store.set_flags(one, &["\\Seen"]).unwrap();
    assert_eq!(store.get(two).unwrap().flags, ["\\Flagged", "\\Seen"]);
    assert_eq!(store.get(one).unwrap().flags, Vec::<String>::new());
}

#[test]
fn a_move_shows_in_its_target_and_back_again_is_no_change() {
    let (_dir, mut store, id, boxes) = standard();
    let one = put(&mut store, boxes.inbox, 1, &[]);
    store.change(&[one], Change::Move(boxes.work)).unwrap();
    assert_eq!(listed(&store, boxes.inbox), []);
    assert_eq!(listed(&store, boxes.work), [one]);
    assert_eq!(store.synced(id).unwrap().waiting, 1);
    // Already shown there: nothing new for the server.
    assert_eq!(store.change(&[one], Change::Move(boxes.work)).unwrap(), []);
    store.change(&[one], Change::Move(boxes.inbox)).unwrap();
    assert_eq!(listed(&store, boxes.inbox), [one]);
    assert_eq!(store.synced(id).unwrap().waiting, 0);
}

#[test]
fn delete_goes_to_trash_and_from_trash_for_good() {
    let (_dir, mut store, id, boxes) = standard();
    let one = put(&mut store, boxes.inbox, 1, &[]);
    let two = put(&mut store, boxes.trash, 2, &[]);
    assert_eq!(store.forever(&[one]).unwrap(), 0);
    assert_eq!(store.forever(&[one, two]).unwrap(), 1);
    store
        .change(&[one], Change::Delete { erase: false })
        .unwrap();
    assert_eq!(listed(&store, boxes.trash), [two, one]);
    // Shown in Trash now: the next delete is final.
    assert_eq!(store.forever(&[one]).unwrap(), 1);
    let unasked = store.change(&[one, two], Change::Delete { erase: false });
    assert!(matches!(unasked, Err(Error::Invalid(_))));
    assert_eq!(listed(&store, boxes.trash), [two, one]);
    store
        .change(&[one, two], Change::Delete { erase: true })
        .unwrap();
    assert_eq!(listed(&store, boxes.trash), []);
    assert!(matches!(store.get(one), Err(Error::Store(_))));
    assert_eq!(store.synced(id).unwrap().waiting, 2);
    // A download in flight does not bring it back.
    let key = Key::Imap {
        uidvalidity: 1,
        uid: 2,
    };
    for fetched in [Fetched::Whole(MAIL), Fetched::Headers(MAIL)] {
        let late = store.put(boxes.trash, &key, &[], fetched);
        assert!(matches!(late, Err(Error::Deleted)));
    }
    assert_eq!(listed(&store, boxes.trash), []);
}

#[test]
fn restore_takes_trash_to_the_inbox_and_nothing_else() {
    let (_dir, mut store, _id, boxes) = standard();
    let one = put(&mut store, boxes.trash, 1, &[]);
    let two = put(&mut store, boxes.work, 2, &[]);
    assert!(matches!(
        store.change(&[one, two], Change::Restore),
        Err(Error::Invalid(_))
    ));
    // All or none: the first stayed.
    assert_eq!(listed(&store, boxes.trash), [one]);
    store.change(&[one], Change::Restore).unwrap();
    assert_eq!(listed(&store, boxes.inbox), [one]);
}

#[test]
fn archive_takes_archive_then_all_mail() {
    let (_dir, mut store, _id, boxes) = standard();
    let one = put(&mut store, boxes.inbox, 1, &[]);
    store.change(&[one], Change::Archive).unwrap();
    assert_eq!(listed(&store, boxes.archive), [one]);
    let (_dir, mut gmail, _, ids) = imap_store(&[
        ("INBOX", Some(Role::Inbox)),
        ("[Gmail]/All Mail", Some(Role::All)),
    ]);
    let (inbox, all) = (ids[0], ids[1]);
    let mine = put(&mut gmail, inbox, 1, &[]);
    gmail.change(&[mine], Change::Archive).unwrap();
    assert_eq!(listed(&gmail, all), [mine]);
    let (_dir, mut bare, _, ids) = imap_store(&[("INBOX", Some(Role::Inbox))]);
    let lone = put(&mut bare, ids[0], 1, &[]);
    assert!(matches!(
        bare.change(&[lone], Change::Archive),
        Err(Error::Invalid(_))
    ));
    // Without Trash, delete is for good.
    assert_eq!(bare.forever(&[lone]).unwrap(), 1);
}

#[test]
fn mail_moves_only_within_its_account() {
    let (_dir, mut store, _id, boxes) = standard();
    let other = store.add_account(&fake::account("bo@example.com")).unwrap();
    let theirs = store.set_folders(other, &[("INBOX", Some(Role::Inbox))]);
    let one = put(&mut store, boxes.inbox, 1, &[]);
    let refused = store.change(&[one], Change::Move(theirs.unwrap()[0]));
    assert!(matches!(refused, Err(Error::Invalid(_))));
    assert_eq!(listed(&store, boxes.inbox), [one]);
}

#[test]
fn pop_mail_changes_here_only_and_stays_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let account = ely_mail_engine::Account {
        auth: Auth::Password,
        protocol: Protocol::Pop { keep: true },
        ..fake::account("ann@example.com")
    };
    let id = store.add_account(&account).unwrap();
    let inbox = store.set_folders(id, &[("INBOX", Some(Role::Inbox))]);
    let inbox = inbox.unwrap()[0];
    let key = Key::Pop { uidl: "u1".into() };
    let one = store.put(inbox, &key, &[], Fetched::Whole(MAIL)).unwrap();
    assert_eq!(store.change(&[one], Change::Seen(true)).unwrap(), []);
    assert_eq!(store.get(one).unwrap().flags, ["\\Seen"]);
    assert!(matches!(
        store.change(&[one], Change::Archive),
        Err(Error::Invalid(_))
    ));
    assert_eq!(store.forever(&[one]).unwrap(), 1);
    assert_eq!(
        store
            .change(&[one], Change::Delete { erase: true })
            .unwrap(),
        []
    );
    assert_eq!(listed(&store, inbox), []);
    assert_eq!(
        store.uidls(inbox).unwrap(),
        HashSet::from(["u1".to_owned()])
    );
    let late = store.put(inbox, &key, &[], Fetched::Whole(MAIL));
    assert!(matches!(late, Err(Error::Deleted)));
    assert_eq!(listed(&store, inbox), []);
    store.forget_gone(inbox, &HashSet::new()).unwrap();
    assert_eq!(store.uidls(inbox).unwrap(), HashSet::new());
    assert_eq!(store.synced(id).unwrap().waiting, 0);
}
