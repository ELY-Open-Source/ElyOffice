//! One address on two servers is two accounts (#96).

use super::{STEPS, account, old_account, open};
use crate::{Account, Error, Store};

fn on(address: &str, host: &str) -> Account {
    let mut account = account(address);
    account.incoming.host = host.into();
    account.secret = Account::secret_for(address, &account.incoming);
    account
}

#[test]
fn one_address_on_two_servers_is_two_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let qq = store.add_account(&on("ann@qq.com", "imap.qq.com")).unwrap();
    let microsoft = store.add_account(&on("ann@qq.com", "outlook.office365.com")).unwrap();
    assert_ne!(qq, microsoft);
    let again = store.add_account(&on("ANN@qq.com", "IMAP.qq.com")).unwrap_err();
    assert!(matches!(&again, Error::Taken(address) if address == "ANN@qq.com"), "{again}");
    let names: Vec<String> = store.accounts().unwrap().into_iter().map(|(_, it)| it.secret).collect();
    assert_eq!(names, ["account:ann@qq.com:imap.qq.com", "account:ann@qq.com:outlook.office365.com"]);
}

#[test]
fn an_older_store_keeps_its_vault_names_and_ids() {
    let dir = tempfile::tempdir().unwrap();
    let old = open(dir.path(), &STEPS[..18]).unwrap();
    let ann = old_account(&old, "Ann@example.com");
    let bob = old_account(&old, "bob@example.com");
    old.conn.execute("DELETE FROM accounts WHERE id = ?1", [bob.0]).unwrap();
    drop(old);
    let store = Store::open(dir.path()).unwrap();
    let [(id, kept)] = store.accounts().unwrap().try_into().unwrap();
    assert_eq!(id, ann);
    // Saved passwords stay where they were.
    assert_eq!(kept.secret, "account:Ann@example.com");
    let taken = store.add_account(&account("ann@example.com")).unwrap_err();
    assert!(matches!(taken, Error::Taken(_)), "{taken}");
    let other = store.add_account(&on("ann@example.com", "mail.example.net")).unwrap();
    assert!(other.0 > bob.0, "account {} came back", bob.0);
}
