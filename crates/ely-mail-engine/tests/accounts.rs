use base64::{Engine, engine::general_purpose::STANDARD};
use ely_mail_engine::{
    Account, Auth, Cached, Error, Fetched, Key, Protocol, Secret, Security, Server, Store,
};
use fake::{Certs, account, certs, expect, send, serve};

fn pop_oauth(address: &str) -> Account {
    Account {
        address: address.into(),
        user: "bo".into(),
        auth: Auth::Microsoft,
        protocol: Protocol::Pop { keep: true },
        incoming: Server {
            host: "pop.example.com".into(),
            port: 995,
            security: Security::Tls,
        },
        outgoing: Server {
            host: "smtp.example.com".into(),
            port: 587,
            security: Security::StartTls,
        },
    }
}

#[test]
fn accounts_come_back_as_added() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let ann = account("ann@example.com");
    let bo = pop_oauth("bo@example.com");
    let first = store.add_account(&ann).unwrap();
    let second = store.add_account(&bo).unwrap();
    drop(store);
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(store.accounts().unwrap(), [(first, ann), (second, bo)]);
}

#[test]
fn an_address_is_added_once() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store.add_account(&account("ann@example.com")).unwrap();
    let err = store
        .add_account(&pop_oauth("ANN@Example.com"))
        .unwrap_err();
    assert!(
        matches!(&err, Error::Taken(address) if address == "ANN@Example.com"),
        "{err}"
    );
    assert_eq!(store.accounts().unwrap().len(), 1);
}

#[test]
fn removing_an_account_keeps_what_another_shares() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let ann = store.add_account(&account("ann@example.com")).unwrap();
    let bo = store.add_account(&account("bo@example.com")).unwrap();
    let (ann_inbox, bo_inbox) = (
        store.mailbox(ann, "INBOX").unwrap(),
        store.mailbox(bo, "INBOX").unwrap(),
    );
    let shared = b"Subject: Both\r\n\r\nshared\r\n";
    let own = b"Subject: Ann only\r\n\r\nmine and mine alone\r\n";
    let key = |uid| Key::Imap {
        uidvalidity: 1,
        uid,
    };
    store
        .put(ann_inbox, &key(1), &[], Fetched::Whole(shared))
        .unwrap();
    store
        .put(ann_inbox, &key(2), &[], Fetched::Whole(own))
        .unwrap();
    let kept = store
        .put(bo_inbox, &key(1), &[], Fetched::Whole(shared))
        .unwrap();
    assert_eq!(
        store.cached(ann).unwrap(),
        Cached {
            messages: 2,
            bytes: own.len() as u64
        }
    );
    store.remove_account(ann).unwrap();
    assert_eq!(store.accounts().unwrap().len(), 1);
    assert_eq!(store.original(kept).unwrap().unwrap(), shared);
    assert_eq!(store.sizes().unwrap().originals, shared.len() as u64);
    let err = store.remove_account(ann).unwrap_err();
    assert!(matches!(err, Error::Store(_)), "{err}");
}

/// IMAP and SMTP scripts that log in and leave.
async fn servers(
    certs: &Certs,
    imap_answer: &str,
) -> (Account, Vec<tokio::task::JoinHandle<Result<(), String>>>) {
    let mut imap = vec![
        send("* OK ready\n"),
        expect("{tag} LOGIN \"ann@example.com\" \"pw\""),
        send(imap_answer),
    ];
    if imap_answer.contains(" OK ") {
        imap.extend([expect("{tag} LOGOUT"), send("* BYE\n{tag} OK bye\n")]);
    }
    let (incoming, imap) = serve(certs, Security::Tls, imap).await;
    let smtp = vec![
        send("220 mx ready\n"),
        expect("EHLO [127.0.0.1]"),
        send("250-mx\n250 AUTH PLAIN\n"),
        expect(&format!(
            "AUTH PLAIN {}",
            STANDARD.encode("\0ann@example.com\0pw")
        )),
        send("235 ok\n"),
        expect("QUIT"),
        send("221 bye\n"),
    ];
    let (outgoing, smtp) = serve(certs, Security::Tls, smtp).await;
    let account = Account {
        incoming,
        outgoing,
        ..account("ann@example.com")
    };
    (account, vec![imap, smtp])
}

#[tokio::test]
async fn a_check_logs_in_to_both_servers() {
    let certs = certs();
    let (account, scripts) = servers(&certs, "{tag} OK in\n").await;
    let checked = account
        .check(Secret::Password("pw".into()), &certs.trusted)
        .await;
    assert!(checked.passed());
    for script in scripts {
        script.await.unwrap().unwrap();
    }
}

#[tokio::test]
async fn each_server_answers_for_itself() {
    let certs = certs();
    let (account, scripts) =
        servers(&certs, "{tag} NO [AUTHENTICATIONFAILED] bad password\n").await;
    let checked = account
        .check(Secret::Password("pw".into()), &certs.trusted)
        .await;
    assert!(!checked.passed());
    match checked.incoming {
        Err(Error::Rejected(text)) => assert!(text.contains("bad password"), "{text}"),
        other => panic!("incoming: {:?}", other.err()),
    }
    checked.outgoing.unwrap();
    for script in scripts {
        script.await.unwrap().unwrap();
    }
}

#[test]
fn what_no_server_could_carry_is_refused() {
    account("ann@example.com").valid().unwrap();
    let broken = [
        Account {
            address: "ann".into(),
            ..account("ann@example.com")
        },
        Account {
            user: String::new(),
            ..account("ann@example.com")
        },
        Account {
            user: "ann\r\nRSET".into(),
            ..account("ann@example.com")
        },
        Account {
            incoming: Server {
                host: "imap example.com".into(),
                port: 993,
                security: Security::Tls,
            },
            ..account("ann@example.com")
        },
        Account {
            outgoing: Server {
                host: "smtp.example.com".into(),
                port: 0,
                security: Security::Tls,
            },
            ..account("ann@example.com")
        },
    ];
    for account in broken {
        let err = account.valid().unwrap_err();
        assert!(matches!(err, Error::Invalid(_)), "{err}");
    }
}

/// What the vault answers for a 0.1.10 account.
struct Kept(Result<Option<String>, String>);

impl ely_mail_engine::Backend for Kept {
    fn get(&self, _: &str) -> Result<Option<String>, String> {
        self.0.clone()
    }
    fn set(&self, _: &str, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn delete(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::test]
async fn a_gmail_account_from_oauth_loads_and_asks_to_be_added_again() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let gmail = Account {
        auth: Auth::Google,
        ..account("ann@gmail.com")
    };
    store.add_account(&gmail).unwrap();
    drop(store);
    let kept = Store::open(dir.path()).unwrap().accounts().unwrap();
    assert_eq!(kept[0].1.auth, Auth::Google, "0.1.10's accounts still load");
    // A kept token, none (session only), and a locked keychain.
    for answer in [Ok(Some("refresh-1".into())), Ok(None), Err("locked".into())] {
        let vault = ely_mail_engine::Vault::new(Box::new(Kept(answer.clone())));
        let err = ely_mail_engine::sync::secret(&kept[0].1, &vault).await.err().unwrap();
        assert!(matches!(err, Error::SignInAgain(_)), "{answer:?}: {err}");
        assert!(
            err.to_string().contains("app password; remove this account and add it again"),
            "{answer:?}: {err}"
        );
    }
}
