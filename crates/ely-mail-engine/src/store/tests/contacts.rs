use super::account;
use crate::store::{Fetched, Key, Letter, Store, Unsent, open, schema::STEPS};
use crate::{Imported, Role, vcard};

fn people(store: &Store) -> Vec<(Option<String>, String, u32)> {
    store.contacts().unwrap().into_iter().map(|each| (each.name, each.address, each.sent)).collect()
}

fn named(name: &str, address: &str, sent: u32) -> (Option<String>, String, u32) {
    (Some(name.into()), address.into(), sent)
}

fn bare(address: &str, sent: u32) -> (Option<String>, String, u32) {
    (None, address.into(), sent)
}

fn sent_to(to: &[&str], cc: &[&str], bcc: &[&str]) -> Unsent {
    let own = |list: &[&str]| list.iter().map(|each| each.to_string()).collect();
    Unsent {
        to: own(to),
        cc: own(cc),
        bcc: own(bcc),
        subject: "s".into(),
        words: "w".into(),
        markdown: false,
        earlier: None,
        in_reply_to: None,
        references: Vec::new(),
        headers_only: false,
        missing: 0,
    }
}

fn letter() -> Letter {
    Letter { envelope: vec!["x@example.com".into()], message_id: "m@example.com".into(), bytes: b"b".to_vec() }
}

#[test]
fn contacts_are_added_named_and_removed() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_contact(" Ann Ross ", " ann@example.com ").unwrap();
    store.add_contact("", "bo@example.com").unwrap();
    assert_eq!(store.add_contact("Ann R.", "ANN@example.com").unwrap(), ann, "one per address");
    store.add_contact("", "Ann@Example.com").unwrap();
    store.add_contact("cy@example.com", "cy@example.com").unwrap();
    assert_eq!(
        people(&store),
        [named("Ann R.", "ann@example.com", 0), bare("bo@example.com", 0), bare("cy@example.com", 0)]
    );
    assert!(store.add_contact("x", "not an address").is_err());
    assert!(store.add_contact("two\nlines", "d@example.com").is_err());
    store.remove_contact(ann).unwrap();
    assert!(store.remove_contact(ann).is_err(), "a gone contact says so");
    assert_eq!(people(&store).len(), 2);
}

#[test]
fn sending_counts_every_recipient() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let id = store.add_account(&account("me@example.com")).unwrap();
    store.add_contact("Bo", "bo@example.com").unwrap();
    let first = sent_to(&["BO@example.com"], &["cy@example.com"], &["dee@example.com"]);
    store.enqueue(id, None, &first, &[], &letter()).unwrap();
    store.enqueue(id, None, &sent_to(&["cy@example.com"], &[], &[]), &[], &letter()).unwrap();
    assert_eq!(
        people(&store),
        [bare("cy@example.com", 2), named("Bo", "bo@example.com", 1), bare("dee@example.com", 1)]
    );
}

#[test]
fn one_message_counts_an_address_once() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let id = store.add_account(&account("me@example.com")).unwrap();
    let twice = sent_to(&["ann@example.com", "Ann@example.com"], &["ANN@example.com"], &["ann@example.com"]);
    store.enqueue(id, None, &twice, &[], &letter()).unwrap();
    assert_eq!(people(&store), [bare("ann@example.com", 1)]);
}

#[test]
fn an_import_is_all_or_nothing_and_exports_back() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store.add_contact("Old", "ann@example.com").unwrap();
    store.add_contact("Bo", "bo@example.com").unwrap();
    let cards = vcard::read(
        "BEGIN:VCARD\nFN:Ann Ross\nEMAIL:ann@example.com\nEMAIL:ann@work.example\nEND:VCARD\n\
         BEGIN:VCARD\nEMAIL:bo@example.com\nEND:VCARD\nBEGIN:VCARD\nFN:No Mail\nEND:VCARD\n",
    )
    .unwrap();
    let imported = store.import_contacts(&cards).unwrap();
    assert_eq!(imported, Imported { added: 1, renamed: 1, unchanged: 1, without_address: 1 });
    let bad = vcard::read("BEGIN:VCARD\nEMAIL:new@example.com\nEMAIL:broken\nEND:VCARD\n").unwrap();
    assert!(store.import_contacts(&bad).unwrap_err().to_string().contains("broken"));
    let kept = people(&store);
    assert_eq!(
        kept,
        [named("Ann Ross", "ann@example.com", 0), named("Ann Ross", "ann@work.example", 0), named("Bo", "bo@example.com", 0)]
    );
    let back = Store::open(tempfile::tempdir().unwrap().path()).map(|mut other| {
        other.import_contacts(&vcard::read(&store.export_contacts().unwrap()).unwrap()).unwrap();
        people(&other)
    });
    assert_eq!(back.unwrap(), kept);
}

#[test]
fn upgrading_seeds_contacts_from_sent_folders() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path(), &STEPS[..14]).unwrap();
    let id = super::old_account(&store, "me@example.com");
    let [inbox, sent] = super::old_folders(&store, id, &[("INBOX", Some(Role::Inbox)), ("Sent", Some(Role::Sent))])
        .try_into()
        .unwrap();
    let mut put = |mailbox, uid, headers: &str| {
        let bytes = format!("{headers}Subject: s\r\nDate: Tue, 2 Jan 2024 10:00:00 +0000\r\n\r\nhi\r\n");
        let key = Key::Imap { uidvalidity: 1, uid };
        super::old_put(&mut store, mailbox, &key, Fetched::Whole(bytes.as_bytes()));
    };
    put(sent, 1, "From: me@example.com\r\nTo: Bo Lee <bo@example.com>\r\nCc: cy@example.com\r\n");
    put(sent, 2, "From: me@example.com\r\nTo: BO@example.com\r\n");
    put(inbox, 3, "From: dee@example.com\r\nTo: me@example.com\r\n");
    drop(store);
    let store = open(dir.path(), STEPS).unwrap();
    assert_eq!(store.read_only(), None);
    let mut seeded = people(&store);
    seeded.iter_mut().for_each(|(_, address, _)| address.make_ascii_lowercase());
    assert_eq!(seeded, [named("Bo Lee", "bo@example.com", 2), bare("cy@example.com", 1)]);
}
