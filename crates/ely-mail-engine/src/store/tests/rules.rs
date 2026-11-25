use super::account;
use crate::{Action, Fetched, Filter, Key, Match, MessageId, Role, Scope, Store};

fn mail(from: &str, to: &str, subject: &str) -> Vec<u8> {
    format!(
        "From: {from}\r\nTo: {to}\r\nSubject: {subject}\r\nDate: Tue, 2 Jan 2024 10:00:00 +0000\r\n\
         Content-Type: text/plain\r\n\r\nhi\r\n"
    )
    .into_bytes()
}

#[test]
fn the_first_matching_rule_acts_on_new_inbox_mail() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox, receipts, archive] = store
        .set_folders(
            ann,
            &[("INBOX", Some(Role::Inbox)), ("Receipts", None), ("Archive", Some(Role::Archive))],
        )
        .unwrap()
        .try_into()
        .unwrap();
    let put = |store: &mut Store, uid, bytes: Vec<u8>| {
        let key = Key::Imap { uidvalidity: 1, uid };
        store.put(inbox, &key, &[], Fetched::Whole(&bytes)).unwrap()
    };
    let old = put(&mut store, 1, mail("Shop <orders@shop.example>", "ann@example.com", "Receipt 1"));
    store.skip_arrivals(ann).unwrap();
    store.add_rule(ann, Match::From, " SHOP.example ", Action::Move(receipts)).unwrap();
    store.add_rule(ann, Match::Subject, "receipt", Action::Flag).unwrap();
    store.add_rule(ann, Match::To, "Team", Action::Archive).unwrap();
    let shop = put(&mut store, 2, mail("orders@shop.example", "ann@example.com", "Receipt 2"));
    let paper = put(&mut store, 3, mail("bo@example.com", "ann@example.com", "Your receipt"));
    let team = put(&mut store, 4, mail("cy@example.com", "The Team <team@example.com>", "plan"));
    let other = put(&mut store, 5, mail("dee@example.com", "ann@example.com", "hello"));
    let handled = store.handle_arrivals(ann).unwrap();
    assert_eq!(handled.ruled, 3);
    let news: Vec<_> = handled.arrived.iter().map(|each| each.id).collect();
    assert_eq!(news, [other, paper], "moved and archived mail is no news");
    let listed = |store: &Store, mailbox| -> Vec<MessageId> {
        store.list(Scope::Mailbox(mailbox), Filter::default(), &[]).unwrap().into_iter().map(|each| each.id).collect()
    };
    assert_eq!(listed(&store, receipts), [shop], "the first rule wins over the subject rule");
    assert_eq!(listed(&store, archive), [team]);
    let flagged = Filter { flagged: true, ..Filter::default() };
    let flagged: Vec<_> = store.list(Scope::Mailbox(inbox), flagged, &[]).unwrap().into_iter().map(|each| each.id).collect();
    assert_eq!(flagged, [paper]);
    let mut kept = listed(&store, inbox);
    kept.sort_by_key(|id| id.0);
    assert_eq!(kept, [old, paper, other], "mail from before the mark stays as it was");
    assert_eq!(store.changes_waiting(ann).unwrap(), 3, "each change waits for the server");
}

#[test]
fn a_rule_needs_words_and_its_own_folder() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let bo = store.add_account(&account("bo@example.com")).unwrap();
    let [bo_inbox] = store.set_folders(bo, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
    assert!(store.add_rule(ann, Match::From, "  ", Action::Flag).is_err());
    assert!(store.add_rule(ann, Match::From, "x", Action::Move(bo_inbox)).is_err());
    let imported = store.import_mbox(ann, "Old", std::io::Cursor::new(b"")).unwrap().folder;
    assert!(store.add_rule(ann, Match::From, "x", Action::Move(imported)).is_err(), "an imported folder is no target");
    let id = store.add_rule(ann, Match::To, "Ann", Action::Flag).unwrap();
    assert_eq!(store.rules(ann).unwrap()[0].words, "ann");
    store.remove_rule(id).unwrap();
    assert!(store.rules(ann).unwrap().is_empty());
    assert!(store.remove_rule(id).is_err(), "a gone rule says so");
    let pop = crate::Account { protocol: crate::Protocol::Pop { keep: true }, ..account("cy@example.com") };
    let cy = store.add_account(&pop).unwrap();
    assert!(store.add_rule(cy, Match::From, "x", Action::Archive).is_err(), "POP3 has nowhere to archive");
    store.add_rule(cy, Match::From, "x", Action::Flag).unwrap();
}

#[test]
fn mail_stored_before_a_failed_sync_still_meets_the_rules() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox] = store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
    store.skip_arrivals(ann).unwrap();
    store.add_rule(ann, Match::Subject, "urgent", Action::Flag).unwrap();
    let put = |store: &mut Store, uid, subject: &str| {
        let key = Key::Imap { uidvalidity: 1, uid };
        store.put(inbox, &key, &[], Fetched::Whole(&mail("bo@example.com", "ann@example.com", subject))).unwrap()
    };
    // This sync failed after storing one.
    let one = put(&mut store, 1, "Urgent one");
    let two = put(&mut store, 2, "Urgent two");
    let handled = store.handle_arrivals(ann).unwrap();
    assert_eq!(handled.ruled, 2);
    assert_eq!(handled.arrived.iter().map(|each| each.id).collect::<Vec<_>>(), [two, one]);
    let again = store.handle_arrivals(ann).unwrap();
    assert_eq!((again.ruled, again.arrived.len()), (0, 0), "each message is handled once");
}

#[test]
fn mail_a_rebuild_brings_back_is_no_news() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox] = store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
    store.skip_arrivals(ann).unwrap();
    store.add_rule(ann, Match::Subject, "plan", Action::Flag).unwrap();
    let with_id = |id: &str| {
        let mut bytes = format!("Message-ID: <{id}>\r\n").into_bytes();
        bytes.extend(mail("bo@example.com", "ann@example.com", "plan"));
        bytes
    };
    let put = |store: &mut Store, uidvalidity, uid, bytes: &[u8]| {
        store.put(inbox, &Key::Imap { uidvalidity, uid }, &[], Fetched::Whole(bytes)).unwrap()
    };
    store.open_mailbox(inbox, 1).unwrap();
    put(&mut store, 1, 1, &with_id("old@example.com"));
    let first = store.handle_arrivals(ann).unwrap();
    assert_eq!((first.ruled, first.arrived.len()), (1, 1));
    // A rule added later, then the server renumbered the folder.
    let other = store.add_rule(ann, Match::From, "bo@", Action::Flag).unwrap();
    store.remove_rule(store.rules(ann).unwrap()[0].id).unwrap();
    assert_eq!(store.rules(ann).unwrap()[0].id, other);
    store.open_mailbox(inbox, 2).unwrap();
    let back = put(&mut store, 2, 1, &with_id("old@example.com"));
    let fresh = put(&mut store, 2, 2, &with_id("new@example.com"));
    let again = store.handle_arrivals(ann).unwrap();
    assert_eq!(again.ruled, 1, "rules pass over mail they met before");
    assert!(store.get(back).unwrap().flags.is_empty(), "the later rule leaves rebuilt mail be");
    assert_eq!(again.arrived.iter().map(|each| each.id).collect::<Vec<_>>(), [fresh]);
    // Handled, the rebuild is forgotten: a later copy is news.
    let copy = put(&mut store, 2, 3, &with_id("old@example.com"));
    let later = store.handle_arrivals(ann).unwrap();
    assert_eq!(later.arrived.iter().map(|each| each.id).collect::<Vec<_>>(), [copy]);
}

#[test]
fn mail_delivered_while_away_is_news_whatever_its_date() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox] = store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
    store.skip_arrivals(ann).unwrap();
    // Written two days before this sync, delivered while offline.
    let late = store
        .put(inbox, &Key::Imap { uidvalidity: 1, uid: 1 }, &[], Fetched::Whole(&mail("bo@example.com", "ann@example.com", "late")))
        .unwrap();
    let arrived = store.handle_arrivals(ann).unwrap().arrived;
    assert_eq!(arrived.iter().map(|each| each.id).collect::<Vec<_>>(), [late]);
}

#[test]
fn a_rebuild_without_message_ids_is_no_news() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox] = store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
    store.skip_arrivals(ann).unwrap();
    let bare = mail("bo@example.com", "ann@example.com", "no id");
    let put = |store: &mut Store, uidvalidity, uid, bytes: &[u8]| {
        store.put(inbox, &Key::Imap { uidvalidity, uid }, &[], Fetched::Whole(bytes)).unwrap()
    };
    store.open_mailbox(inbox, 1).unwrap();
    put(&mut store, 1, 1, &bare);
    assert_eq!(store.handle_arrivals(ann).unwrap().arrived.len(), 1);
    store.open_mailbox(inbox, 2).unwrap();
    put(&mut store, 2, 1, &bare);
    let other = put(&mut store, 2, 2, &mail("cy@example.com", "ann@example.com", "no id"));
    let again = store.handle_arrivals(ann).unwrap().arrived;
    assert_eq!(again.iter().map(|each| each.id).collect::<Vec<_>>(), [other], "same date, sender and subject is the same mail");
}

#[test]
fn mail_never_handled_before_a_rebuild_still_is_news() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox] = store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
    store.skip_arrivals(ann).unwrap();
    store.add_rule(ann, Match::Subject, "urgent", Action::Flag).unwrap();
    let urgent = {
        let mut bytes = b"Message-ID: <urgent@example.com>\r\n".to_vec();
        bytes.extend(mail("bo@example.com", "ann@example.com", "urgent"));
        bytes
    };
    let put = |store: &mut Store, uidvalidity, uid| {
        store.put(inbox, &Key::Imap { uidvalidity, uid }, &[], Fetched::Whole(&urgent)).unwrap()
    };
    store.open_mailbox(inbox, 1).unwrap();
    // Stored, then the sync failed before handling it.
    put(&mut store, 1, 1);
    store.open_mailbox(inbox, 2).unwrap();
    let back = put(&mut store, 2, 1);
    let handled = store.handle_arrivals(ann).unwrap();
    assert_eq!(handled.ruled, 1, "the rule meets it once");
    assert_eq!(handled.arrived.iter().map(|each| each.id).collect::<Vec<_>>(), [back]);
}

#[test]
fn an_archive_rule_needs_an_archive_and_never_blocks_the_others() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox] = store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
    assert!(store.add_rule(ann, Match::Subject, "urgent", Action::Archive).is_err(), "nowhere to archive");
    store.set_folders(ann, &[("INBOX", Some(Role::Inbox)), ("Archive", Some(Role::Archive))]).unwrap();
    store.add_rule(ann, Match::Subject, "urgent", Action::Archive).unwrap();
    store.add_rule(ann, Match::Subject, "normal", Action::Flag).unwrap();
    // The server dropped its Archive folder later.
    store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap();
    store.skip_arrivals(ann).unwrap();
    let put = |store: &mut Store, uid, subject: &str| {
        let key = Key::Imap { uidvalidity: 1, uid };
        store.put(inbox, &key, &[], Fetched::Whole(&mail("bo@example.com", "ann@example.com", subject))).unwrap()
    };
    let urgent = put(&mut store, 1, "urgent");
    let normal = put(&mut store, 2, "normal");
    let handled = store.handle_arrivals(ann).unwrap();
    assert_eq!(handled.ruled, 1, "the flag rule still acts");
    assert_eq!(handled.refused.len(), 1, "the archive rule says why not");
    assert!(handled.refused[0].contains("Archive"), "{:?}", handled.refused);
    assert!(store.get(normal).unwrap().flags.iter().any(|flag| flag == r"\Flagged"));
    let mut news: Vec<_> = handled.arrived.iter().map(|each| each.id).collect();
    news.sort_by_key(|id| id.0);
    assert_eq!(news, [urgent, normal], "unarchived mail stays news");
}

#[test]
fn a_rebuild_tells_unnamed_mail_apart_by_its_body() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let [inbox] = store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
    store.skip_arrivals(ann).unwrap();
    let old = mail("bo@example.com", "ann@example.com", "no id");
    let mut fresh = old.clone();
    fresh.extend_from_slice(b"another body");
    let put = |store: &mut Store, uidvalidity, uid, bytes: &[u8]| {
        store.put(inbox, &Key::Imap { uidvalidity, uid }, &[], Fetched::Whole(bytes)).unwrap()
    };
    store.open_mailbox(inbox, 1).unwrap();
    put(&mut store, 1, 1, &old);
    assert_eq!(store.handle_arrivals(ann).unwrap().arrived.len(), 1);
    store.open_mailbox(inbox, 2).unwrap();
    put(&mut store, 2, 1, &old);
    let other = put(&mut store, 2, 2, &fresh);
    let again = store.handle_arrivals(ann).unwrap().arrived;
    assert_eq!(again.iter().map(|each| each.id).collect::<Vec<_>>(), [other], "same headers, another body: news");
}
