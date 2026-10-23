//! IMAP and POP3 sync (§12.3).

use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, MutexGuard},
};

use crate::{
    Account, AccountId, Error, Fetched, Held, Imap, Key, MailboxId, Pop, Protocol, Result, Role,
    Secret, Store, TlsConnector, Vault, oauth, push,
};

/// Bodies come down for mail this recent.
pub const BODY_DAYS: i64 = 90;

/// UIDs per FETCH.
const BATCH: usize = 100;

/// What one sync changed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub folders: usize,
    pub headers: usize,
    pub bodies: usize,
    pub removed: usize,
    /// POP3 messages deleted from the server.
    pub deleted: usize,
    /// Changes made here that the server took.
    pub pushed: usize,
}

/// The password, or an access token from the refresh token.
pub async fn secret(account: &Account, vault: &Vault) -> Result<Secret> {
    let name = account.secret_name();
    let provider = account.auth.provider().transpose()?;
    let kept = vault
        .get(&name)?
        .ok_or_else(|| Error::Vault(format!("no saved secret for {name}")))?;
    let Some(provider) = provider else {
        return Ok(Secret::Password(kept));
    };
    let tokens = oauth::refresh(&provider, &kept).await?;
    if let Some(next) = &tokens.refresh {
        vault.put(&name, next);
    }
    Ok(Secret::Token(tokens.access))
}

/// One pass over all folders; `step` follows folders, then each.
pub async fn imap(
    account: &Account,
    id: AccountId,
    secret: Secret,
    tls: &TlsConnector,
    store: &Mutex<Store>,
    now: i64,
    step: &(dyn Fn() + Sync),
) -> Result<Report> {
    let mut imap = Imap::connect(&account.incoming, &account.login(secret), tls).await?;
    // §12.3: changes go first, checked against the server.
    let pushed = push::push(&mut imap, store, id).await?;
    let folders: Vec<_> = imap
        .folders()
        .await?
        .into_iter()
        .filter(|folder| folder.selectable)
        .collect();
    let named: Vec<_> = folders
        .iter()
        .map(|folder| (folder.name.as_str(), folder.role))
        .collect();
    let boxes = lock(store)?.set_folders(id, &named)?;
    step();
    let day = since(now)?;
    let mut report = Report {
        folders: folders.len(),
        pushed,
        ..Report::default()
    };
    for (folder, mailbox) in folders.iter().zip(boxes) {
        one(&mut imap, store, mailbox, &folder.name, &day, &mut report).await?;
        step();
    }
    imap.logout().await?;
    lock(store)?.set_synced(id, now)?;
    log::info!(
        "mail: account {} synced: {} folders, {} headers, {} bodies, {} removed",
        id.0,
        report.folders,
        report.headers,
        report.bodies,
        report.removed
    );
    Ok(report)
}

/// Only the queue of changes; no mail comes down.
pub async fn changes(
    account: &Account,
    id: AccountId,
    secret: Secret,
    tls: &TlsConnector,
    store: &Mutex<Store>,
) -> Result<usize> {
    let mut imap = Imap::connect(&account.incoming, &account.login(secret), tls).await?;
    let pushed = push::push(&mut imap, store, id).await?;
    imap.logout().await?;
    Ok(pushed)
}

/// New mail by UIDL; copies stay; `step` after the inbox.
pub async fn pop(
    account: &Account,
    id: AccountId,
    secret: Secret,
    tls: &TlsConnector,
    store: &Mutex<Store>,
    now: i64,
    step: &(dyn Fn() + Sync),
) -> Result<Report> {
    let Protocol::Pop { keep } = account.protocol else {
        return Err(Error::Invalid("a POP3 sync of an IMAP account".into()));
    };
    let mut pop = Pop::connect(&account.incoming, &account.login(secret), tls).await?;
    let inbox = match lock(store)?.set_folders(id, &[("INBOX", Some(Role::Inbox))])?[..] {
        [inbox] => inbox,
        _ => return Err(Error::Store("a POP3 account has one inbox".into())),
    };
    step();
    let mut held = lock(store)?.uidls(inbox)?;
    let mut report = Report {
        folders: 1,
        ..Report::default()
    };
    let listed = pop.uids().await?;
    lock(store)?.forget_gone(
        inbox,
        &listed.iter().map(|(_, uidl)| uidl.clone()).collect(),
    )?;
    for (number, uidl) in listed {
        if !held.contains(&uidl) {
            let bytes = pop.fetch(number).await?;
            let key = Key::Pop { uidl: uidl.clone() };
            if put(store, inbox, key, &[], Fetched::Whole(&bytes))? {
                report.bodies += 1;
            }
            held.insert(uidl);
        }
        if !keep {
            pop.delete(number).await?;
            report.deleted += 1;
        }
    }
    pop.quit().await?;
    lock(store)?.set_synced(id, now)?;
    log::info!(
        "mail: account {} synced: {} new, {} deleted on the server",
        id.0,
        report.bodies,
        report.deleted
    );
    Ok(report)
}

async fn one(
    imap: &mut Imap,
    store: &Mutex<Store>,
    mailbox: MailboxId,
    name: &str,
    day: &str,
    report: &mut Report,
) -> Result<()> {
    let opened = imap.examine(name).await?;
    let held = lock(store)?.open_mailbox(mailbox, opened.uidvalidity)?;
    if opened.exists == 0 {
        return remove(store, held.values(), report);
    }
    let server: HashMap<u32, Vec<String>> = imap.flags().await?.into_iter().collect();
    let gone = held
        .iter()
        .filter(|(uid, _)| !server.contains_key(uid))
        .map(|(_, held)| held);
    remove(store, gone, report)?;
    for (uid, held) in &held {
        // The store keeps changes not yet sent.
        if let Some(flags) = server.get(uid).filter(|flags| **flags != held.flags) {
            lock(store)?.set_flags(held.id, &strs(flags))?;
        }
    }
    let key = |uid| Key::Imap {
        uidvalidity: opened.uidvalidity,
        uid,
    };
    if lock(store)?.offline(mailbox)? {
        let mut whole: Vec<u32> = server
            .keys()
            .filter(|uid| !held.get(uid).is_some_and(|held| held.whole))
            .copied()
            .collect();
        whole.sort_unstable();
        for batch in whole.chunks(BATCH) {
            for got in imap.bodies(batch).await? {
                let fetched = Fetched::Whole(&got.bytes);
                let flags = strs(&got.flags);
                if put(store, mailbox, key(got.uid), &flags, fetched)? {
                    report.bodies += 1;
                }
            }
        }
        return Ok(());
    }
    let recent: HashSet<u32> = imap.since(day).await?.into_iter().collect();
    let mut headers = Vec::new();
    let mut bodies = Vec::new();
    for uid in server.keys() {
        match (held.get(uid), recent.contains(uid)) {
            // Trimmed for the cap, a body waits for a press.
            (None | Some(Held { body: false, trimmed: false, .. }), true) => bodies.push(*uid),
            (None, false) => headers.push(*uid),
            _ => {}
        }
    }
    headers.sort_unstable();
    bodies.sort_unstable();
    for batch in headers.chunks(BATCH) {
        for got in imap.headers(batch).await? {
            let fetched = Fetched::Headers(&got.bytes);
            if put(store, mailbox, key(got.uid), &strs(&got.flags), fetched)? {
                report.headers += 1;
            }
        }
    }
    for batch in bodies.chunks(BATCH) {
        let (whole, partial): (Vec<_>, Vec<_>) = imap
            .structures(batch)
            .await?
            .into_iter()
            .partition(|(_, node)| node.complete());
        let whole: Vec<u32> = whole.into_iter().map(|(uid, _)| uid).collect();
        if !whole.is_empty() {
            for got in imap.bodies(&whole).await? {
                let fetched = Fetched::Whole(&got.bytes);
                if put(store, mailbox, key(got.uid), &strs(&got.flags), fetched)? {
                    report.bodies += 1;
                }
            }
        }
        // §12.3: attachments wait for a request.
        for (uid, node) in partial {
            let Some(got) = imap.text(uid, &node).await? else {
                continue;
            };
            let missing = node.missing();
            let text = Fetched::Text {
                bytes: &got.bytes,
                missing: &missing,
            };
            if put(store, mailbox, key(uid), &strs(&got.flags), text)? {
                report.bodies += 1;
            }
        }
    }
    Ok(())
}

/// Whether it was kept; deleted meanwhile, it is not.
fn put(
    store: &Mutex<Store>,
    mailbox: MailboxId,
    key: Key,
    flags: &[&str],
    fetched: Fetched,
) -> Result<bool> {
    match lock(store)?.put(mailbox, &key, flags, fetched) {
        Ok(_) => Ok(true),
        Err(Error::Deleted) => Ok(false),
        Err(err) => Err(err),
    }
}

fn remove<'a>(
    store: &Mutex<Store>,
    gone: impl Iterator<Item = &'a Held>,
    report: &mut Report,
) -> Result<()> {
    for held in gone {
        lock(store)?.remove(held.id)?;
        report.removed += 1;
    }
    Ok(())
}

fn strs(flags: &[String]) -> Vec<&str> {
    flags.iter().map(String::as_str).collect()
}

/// `SINCE` takes a date in English: `19-Jul-2026`.
fn since(now: i64) -> Result<String> {
    let start = jiff::Timestamp::from_second(now - BODY_DAYS * 86_400)
        .map_err(|err| Error::Store(format!("a sync time: {err}")))?;
    Ok(start
        .to_zoned(jiff::tz::TimeZone::UTC)
        .strftime("%-d-%b-%Y")
        .to_string())
}

/// The store, or why a task broke it.
pub fn lock(store: &Mutex<Store>) -> Result<MutexGuard<'_, Store>> {
    store
        .lock()
        .map_err(|_| Error::Store("a store task panicked".into()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_window_starts_ninety_days_back() {
        // 2026-10-19 12:00 UTC
        assert_eq!(super::since(1_792_411_200).unwrap(), "21-Jul-2026");
    }
}
