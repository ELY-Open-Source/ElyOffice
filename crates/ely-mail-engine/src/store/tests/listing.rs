use super::account;
use crate::{Change, Fetched, Filter, Key, MailboxId, Role, Scope, Store};

fn mail(subject: &str, date: Option<&str>, attachment: bool) -> Vec<u8> {
    let mut head = format!("From: Ann <ann@example.com>\r\nSubject: {subject}\r\n");
    if let Some(date) = date {
        head.push_str(&format!("Date: {date}\r\n"));
    }
    if attachment {
        format!(
            "{head}Content-Type: multipart/mixed; boundary=b\r\n\r\n--b\r\n\
             Content-Type: text/plain\r\n\r\nhi\r\n--b\r\n\
             Content-Type: application/pdf\r\nContent-Disposition: attachment; filename=a.pdf\r\n\r\n\
             %PDF\r\n--b--\r\n"
        )
        .into_bytes()
    } else {
        format!("{head}Content-Type: text/plain\r\n\r\nhi\r\n").into_bytes()
    }
}

fn put(store: &mut Store, mailbox: MailboxId, uid: u32, flags: &[&str], bytes: &[u8]) {
    let key = Key::Imap {
        uidvalidity: 1,
        uid,
    };
    store
        .put(mailbox, &key, flags, Fetched::Whole(bytes))
        .unwrap();
}

fn subjects(store: &Store, scope: Scope, filter: Filter) -> Vec<String> {
    let listed = store.list(scope, filter, &[]).unwrap();
    listed.into_iter().map(|s| s.subject.unwrap()).collect()
}

/// Two accounts: inbox and archive, inbox only.
fn two_accounts() -> (tempfile::TempDir, Store, [MailboxId; 3]) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let bob = store.add_account(&account("bob@example.com")).unwrap();
    let [ann_inbox, archive] = store
        .set_folders(
            ann,
            &[
                ("INBOX", Some(Role::Inbox)),
                ("Archive", Some(Role::Archive)),
            ],
        )
        .unwrap()
        .try_into()
        .unwrap();
    let [bob_inbox] = store
        .set_folders(bob, &[("INBOX", Some(Role::Inbox))])
        .unwrap()
        .try_into()
        .unwrap();
    (dir, store, [ann_inbox, archive, bob_inbox])
}

#[test]
fn every_inbox_lists_newest_first_and_undated_last() {
    let (_dir, mut store, [ann, archive, bob]) = two_accounts();
    put(
        &mut store,
        ann,
        1,
        &[],
        &mail("old", Some("Mon, 1 Jan 2024 10:00:00 +0000"), false),
    );
    put(
        &mut store,
        bob,
        1,
        &[],
        &mail("new", Some("Tue, 2 Jan 2024 10:00:00 +0000"), false),
    );
    put(&mut store, ann, 2, &[], &mail("undated", None, false));
    put(
        &mut store,
        archive,
        1,
        &[],
        &mail("filed", Some("Wed, 3 Jan 2024 10:00:00 +0000"), false),
    );

    assert_eq!(
        subjects(&store, Scope::Inboxes, Filter::default()),
        ["new", "old", "undated"]
    );
    assert_eq!(
        subjects(&store, Scope::Mailbox(archive), Filter::default()),
        ["filed"]
    );
    assert_eq!(
        subjects(&store, Scope::Mailbox(ann), Filter::default()),
        ["old", "undated"]
    );

    let first = &store.list(Scope::Inboxes, Filter::default(), &[]).unwrap()[0];
    assert_eq!(first.from[0].email, "ann@example.com");
    assert_eq!(first.date, Some(1_704_189_600));
}

#[test]
fn filters_narrow_together() {
    let (_dir, mut store, [ann, ..]) = two_accounts();
    let date = Some("Mon, 1 Jan 2024 10:00:00 +0000");
    put(&mut store, ann, 1, &["\\Seen"], &mail("read", date, false));
    put(
        &mut store,
        ann,
        2,
        &["\\SEEN", "\\flagged"],
        &mail("read flagged", date, true),
    );
    put(
        &mut store,
        ann,
        3,
        &["\\Flagged"],
        &mail("unread flagged", date, true),
    );
    put(&mut store, ann, 4, &[], &mail("unread", date, false));

    let scope = Scope::Mailbox(ann);
    let unread = Filter {
        unread: true,
        ..Filter::default()
    };
    let flagged = Filter {
        flagged: true,
        ..Filter::default()
    };
    let attached = Filter {
        attachments: true,
        ..Filter::default()
    };
    let all = Filter {
        unread: true,
        flagged: true,
        attachments: true,
    };
    assert_eq!(
        subjects(&store, scope, unread),
        ["unread", "unread flagged"]
    );
    assert_eq!(
        subjects(&store, scope, flagged),
        ["unread flagged", "read flagged"]
    );
    assert_eq!(
        subjects(&store, scope, attached),
        ["unread flagged", "read flagged"]
    );
    assert_eq!(subjects(&store, scope, all), ["unread flagged"]);

    let listed = store.list(scope, Filter::default(), &[]).unwrap();
    let read_flagged = listed
        .iter()
        .find(|s| s.subject.as_deref() == Some("read flagged"))
        .unwrap();
    assert!(read_flagged.seen && read_flagged.flagged);
    assert_eq!(read_flagged.attachments, Some(1));
}

#[test]
fn unread_counts_by_mailbox() {
    let (_dir, mut store, [ann, archive, bob]) = two_accounts();
    put(&mut store, ann, 1, &[], &mail("a", None, false));
    put(&mut store, ann, 2, &["\\seen"], &mail("b", None, false));
    put(&mut store, ann, 3, &[], &mail("c", None, false));
    put(&mut store, archive, 1, &["\\Seen"], &mail("d", None, false));
    put(&mut store, bob, 1, &[], &mail("e", None, false));

    let counts = store.unread().unwrap();
    assert_eq!(counts.len(), 2);
    assert_eq!((counts[&ann], counts[&bob]), (2, 1));
    assert!(!counts.contains_key(&archive));
}

#[test]
fn arrived_is_unread_inbox_mail_stored_past_the_mark() {
    let (_dir, mut store, [ann, archive, bob]) = two_accounts();
    put(&mut store, ann, 1, &[], &mail("before", Some("Tue, 2 Jan 2024 10:00:00 +0000"), false));
    let mark = store.newest().unwrap();
    let recent = "Tue, 2 Jan 2024 10:00:00 +0000";
    put(&mut store, ann, 2, &[], &mail("new", Some(recent), false));
    put(&mut store, ann, 3, &["\\Seen"], &mail("read", Some(recent), false));
    put(&mut store, ann, 4, &[], &mail("old date", Some("Mon, 1 Jan 2024 10:00:00 +0000"), false));
    put(&mut store, ann, 5, &[], &mail("undated", None, false));
    put(&mut store, archive, 1, &[], &mail("archived", Some(recent), false));
    put(&mut store, bob, 1, &[], &mail("bob's", Some(recent), false));
    let ann_account = store.accounts().unwrap()[0].0;
    let arrived = store.arrived(ann_account, mark).unwrap();
    let subjects: Vec<_> = arrived.into_iter().map(|each| each.subject.unwrap()).collect();
    assert_eq!(subjects, ["new", "old date", "undated"], "its own date does not decide");
    assert!(store.newest().unwrap().0 > mark.0);
}

#[test]
fn a_held_message_stays_though_it_no_longer_fits_the_filter() {
    let (_dir, mut store, [ann, archive, _]) = two_accounts();
    let date = Some("Mon, 1 Jan 2024 10:00:00 +0000");
    put(&mut store, ann, 1, &[], &mail("opened", date, false));
    put(&mut store, ann, 2, &[], &mail("waiting", date, false));
    put(&mut store, archive, 3, &[], &mail("elsewhere", date, false));
    let unread = Filter { unread: true, ..Filter::default() };
    let all = store.list(Scope::Mailbox(ann), Filter::default(), &[]).unwrap();
    let opened = all.iter().find(|s| s.subject.as_deref() == Some("opened")).unwrap().id;
    let elsewhere = store.list(Scope::Mailbox(archive), Filter::default(), &[]).unwrap()[0].id;
    store.change(&[opened], Change::Seen(true)).unwrap();
    let held = [opened, elsewhere];
    let listed = store.list(Scope::Mailbox(ann), unread, &held).unwrap();
    let names: Vec<_> = listed.iter().map(|s| (s.subject.clone().unwrap(), s.seen)).collect();
    let expected = [("waiting".to_owned(), false), ("opened".to_owned(), true)];
    assert_eq!(names, expected, "the read one stays; the other folder's does not come in");
    assert_eq!(subjects(&store, Scope::Mailbox(ann), unread), ["waiting"], "without the hold it goes");
}
