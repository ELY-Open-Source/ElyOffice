use super::{account, old_folders};
use crate::{Change, Fetched, Key, Role, Store, push::Dropped};

#[test]
fn an_old_name_that_does_not_decode_still_goes() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let held = [("INBOX", Some(Role::Inbox)), ("caf\u{e9}", None)];
    let [inbox, old] = old_folders(&store, ann, &held).try_into().unwrap();
    let err = store.folders(ann).unwrap_err().to_string();
    assert!(err.contains("caf\u{e9}"), "{err}");
    store.open_mailbox(inbox, 7).unwrap();
    store.open_mailbox(old, 7).unwrap();
    let key = Key::Imap { uidvalidity: 7, uid: 1 };
    let message = store.put(inbox, &key, &[], Fetched::Whole(b"From: a@example.com\r\n\r\nhi\r\n")).unwrap();
    store.change(&[message], Change::Move(old)).unwrap();
    store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap();
    assert_eq!(store.take_dropped(ann).unwrap(), [Dropped::Vanished("caf\u{e9}".into())]);
    assert_eq!(store.folders(ann).unwrap().len(), 1);
}
