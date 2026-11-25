//! Shared steps for tests of the change queue.

use std::sync::{Arc, Mutex};

use fake::{certs, expect, send, serve};
use ely_mail_engine::{
    Account, AccountId, Change, Error, Fetched, Filter, Key, MailboxId, MessageId, Role, Scope,
    Secret, Security, Store, push::Dropped, sync,
};

const MAIL: &[u8] = b"From: a@example.com\r\nSubject: Hi\r\n\r\nhello\r\n";

pub struct Here {
    pub _dir: tempfile::TempDir,
    pub store: Arc<Mutex<Store>>,
    pub id: AccountId,
    pub inbox: MailboxId,
    pub work: MailboxId,
}

/// INBOX and Work at UIDVALIDITY 7; INBOX holds `uids`.
pub fn here(uids: &[u32]) -> (Here, Vec<MessageId>) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let id = store
        .add_account(&fake::account("ann@example.com"))
        .unwrap();
    let folders = [("INBOX", Some(Role::Inbox)), ("Work", None)];
    let [inbox, work] = store.set_folders(id, &folders).unwrap().try_into().unwrap();
    store.open_mailbox(inbox, 7).unwrap();
    store.open_mailbox(work, 7).unwrap();
    let ids = uids
        .iter()
        .map(|&uid| {
            let key = Key::Imap {
                uidvalidity: 7,
                uid,
            };
            store.put(inbox, &key, &[], Fetched::Whole(MAIL)).unwrap()
        })
        .collect();
    let here = Here {
        _dir: dir,
        store: Arc::new(Mutex::new(store)),
        id,
        inbox,
        work,
    };
    (here, ids)
}

pub fn change(here: &Here, ids: &[MessageId], change: Change) {
    here.store.lock().unwrap().change(ids, change).unwrap();
}

/// Logs in, runs `steps`, logs out; what was let go.
pub async fn push(here: &Here, steps: Vec<fake::Step>) -> Result<Vec<Dropped>, Error> {
    let certs = certs();
    let mut script = vec![
        send("* OK ready\n"),
        expect("{tag} LOGIN \"ann@example.com\" \"pw\""),
        send("{tag} OK done\n"),
    ];
    script.extend(steps);
    script.push(expect("{tag} LOGOUT"));
    script.push(send("* BYE\n{tag} OK bye\n"));
    let (incoming, task) = serve(&certs, Security::Tls, script).await;
    let account = Account {
        incoming,
        ..fake::account("ann@example.com")
    };
    let secret = Secret::Password("pw".into());
    let done = sync::changes(&account, here.id, secret, &certs.trusted, &here.store).await;
    if done.is_ok() {
        task.await.unwrap().unwrap();
    }
    done.map(|_| dropped(here))
}

pub fn dropped(here: &Here) -> Vec<Dropped> {
    here.store.lock().unwrap().take_dropped(here.id).unwrap()
}

pub fn caps(list: &str) -> Vec<fake::Step> {
    vec![
        expect("{tag} CAPABILITY"),
        send(&format!("* CAPABILITY IMAP4rev1 {list}\n{{tag}} OK caps\n")),
    ]
}

pub fn select(validity: u32, exists: u32) -> Vec<fake::Step> {
    vec![
        expect("{tag} SELECT \"INBOX\""),
        send(&format!(
            "* {exists} EXISTS\n* OK [UIDVALIDITY {validity}] v\n{{tag}} OK [READ-WRITE] in\n"
        )),
    ]
}

pub fn present(asked: &str, uids: &[u32]) -> Vec<fake::Step> {
    let mut lines: String = uids
        .iter()
        .map(|uid| format!("* {uid} FETCH (UID {uid})\n"))
        .collect();
    lines.push_str("{tag} OK fetched\n");
    vec![
        expect(&format!("{{tag}} UID FETCH {asked} (UID)")),
        send(&lines),
    ]
}

pub fn steps(parts: Vec<Vec<fake::Step>>) -> Vec<fake::Step> {
    parts.into_iter().flatten().collect()
}

pub fn waiting(here: &Here) -> u64 {
    here.store.lock().unwrap().synced(here.id).unwrap().waiting
}

pub fn listed(here: &Here, mailbox: MailboxId) -> Vec<MessageId> {
    let store = here.store.lock().unwrap();
    let list = store.list(Scope::Mailbox(mailbox), Filter::default(), &[]);
    list.unwrap().into_iter().map(|each| each.id).collect()
}
