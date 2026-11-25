//! Pinned folders and folded accounts outlive the window.

use super::{STEPS, account, open};
use crate::{Role, Store};

#[test]
fn a_pin_and_a_fold_are_kept_and_reread() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let bob = store.add_account(&account("bob@example.com")).unwrap();
    let [inbox, sent] = store
        .set_folders(
            ann,
            &[("INBOX", Some(Role::Inbox)), ("Sent", Some(Role::Sent))],
        )
        .unwrap()
        .try_into()
        .unwrap();
    store.set_pinned(sent, true).unwrap();
    store.set_folded(bob, true).unwrap();
    drop(store);
    let store = Store::open(dir.path()).unwrap();
    let pinned: Vec<_> = store
        .folders(ann)
        .unwrap()
        .into_iter()
        .map(|folder| (folder.id, folder.pinned))
        .collect();
    assert_eq!(pinned, [(inbox, false), (sent, true)]);
    assert_eq!(store.folded().unwrap(), [bob]);
    store.set_folded(bob, false).unwrap();
    assert!(store.folded().unwrap().is_empty());
}

#[test]
fn a_pin_or_fold_of_nothing_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    assert!(store.set_pinned(crate::MailboxId(9), true).is_err());
    assert!(store.set_folded(crate::AccountId(9), true).is_err());
}

#[test]
fn version_12_takes_the_new_columns_unpinned_and_open() {
    let dir = tempfile::tempdir().unwrap();
    let store = open(dir.path(), &STEPS[..12]).unwrap();
    let ann = super::old_account(&store, "ann@example.com");
    super::old_folders(&store, ann, &[("INBOX", Some(Role::Inbox))]);
    drop(store);
    let store = Store::open(dir.path()).unwrap();
    assert!(!store.folders(ann).unwrap()[0].pinned);
    assert!(store.folded().unwrap().is_empty());
}
