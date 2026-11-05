//! The queue of changes, sent before each sync (§12.3).

use std::{collections::BTreeMap, sync::Mutex};

use crate::{
    AccountId, Error, Imap, Result, Store,
    imap::Can,
    store::{Done, Pending, Settled},
    sync::lock,
};

/// Why a queued change was let go, not retried.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dropped {
    /// The server no longer has the message.
    Gone,
    /// The folder's UIDs were renumbered.
    Renumbered,
    /// The server's words.
    Refused(String),
    /// The server lacks the extension named.
    Unsupported(String),
    /// Moved with no new UID; a later change went.
    Unplaced,
    /// A folder it needed is no longer listed.
    Vanished(String),
}

/// One queued change and what became of it.
struct Entry {
    pending: Pending,
    settled: Settled,
    /// Settled already, mid-push.
    kept: bool,
}

/// Sends the account's queue; how many the server took.
pub(crate) async fn push(
    imap: &mut Imap,
    store: &Mutex<Store>,
    account: AccountId,
) -> Result<usize> {
    let queue = lock(store)?.queued_changes(account)?;
    let (empty, queue): (Vec<_>, Vec<_>) = queue.into_iter().partition(Pending::empty);
    // Undone here before it went: nothing to send.
    {
        let mut held = lock(store)?;
        for each in &empty {
            held.pushed(account, each, &stayed(false))?;
        }
    }
    if queue.is_empty() {
        return Ok(0);
    }
    let can = imap.can().await?;
    let mut groups: BTreeMap<(i64, u32), Vec<Entry>> = BTreeMap::new();
    for pending in queue {
        let key = (pending.mailbox.0, pending.uidvalidity);
        groups.entry(key).or_default().push(Entry {
            pending,
            settled: Settled::waiting(),
            kept: false,
        });
    }
    let mut took = 0;
    for mut group in groups.into_values() {
        // What is known stays known, whatever breaks after.
        let broke = one(imap, store, account, &mut group, can).await.err();
        took += settle(store, account, &group)?;
        if let Some(error) = broke {
            return Err(error);
        }
    }
    log::info!("mail: account {} pushed {took} changes", account.0);
    Ok(took)
}

async fn one(
    imap: &mut Imap,
    store: &Mutex<Store>,
    account: AccountId,
    group: &mut Vec<Entry>,
    can: Can,
) -> Result<()> {
    let (name, uidvalidity) = (group[0].pending.name.clone(), group[0].pending.uidvalidity);
    let opened = match refused(imap.select(&name).await)? {
        Ok(opened) => opened,
        Err(why) => {
            refuse_all(group, why);
            return Ok(());
        }
    };
    if opened.uidvalidity != uidvalidity {
        log::warn!("mail: a mailbox was renumbered; its changes go");
        drop_all(group, Dropped::Renumbered);
        return Ok(());
    }
    let uids: Vec<u32> = group.iter().map(|each| each.pending.uid).collect();
    let present = match refused(imap.present(&uids).await)? {
        Ok(present) => present,
        Err(why) => {
            refuse_all(group, why);
            return Ok(());
        }
    };
    for each in group.iter_mut() {
        if !present.contains(&each.pending.uid) {
            each.settled.done = match each.pending.expunge {
                true => Done::Erased,
                false => {
                    each.settled.dropped.push(Dropped::Gone);
                    Done::Moot
                }
            };
        }
    }
    flags(imap, group).await?;
    for at in live(group, |each| each.target.is_none() && !each.expunge) {
        group[at].settled.done = Done::Stayed {
            move_refused: false,
        };
    }
    let erased = live(group, |each| each.expunge);
    if !erased.is_empty() {
        let uids = uids_at(group, &erased);
        match refused(imap.erase(&uids, can).await)? {
            Ok(()) => erased
                .iter()
                .for_each(|&at| group[at].settled.done = Done::Erased),
            Err(why) => refuse(group, &erased, why, false),
        }
    }
    let mut targets: BTreeMap<i64, (String, Vec<usize>)> = BTreeMap::new();
    for at in live(group, |each| each.target.is_some()) {
        let (id, name) = group[at].pending.target.clone().expect("a target");
        targets.entry(id.0).or_insert((name, Vec::new())).1.push(at);
    }
    for (name, picked) in targets.into_values() {
        if can.moves {
            let uids = uids_at(group, &picked);
            match refused(imap.move_to(&uids, &name).await)? {
                Ok(moved) => {
                    for &at in &picked {
                        let new = moved.uids.get(&group[at].pending.uid);
                        let new = new.zip(moved.uidvalidity).map(|(uid, at)| (at, *uid));
                        group[at].settled.done = Done::Moved(new);
                    }
                }
                Err(why) => refuse(group, &picked, why, true),
            }
        } else if can.uidplus {
            copy(imap, store, account, group, &picked, &name, can).await?;
        } else {
            refuse(group, &picked, Dropped::Unsupported("MOVE".into()), true);
        }
    }
    find(imap, group).await
}

/// Flags in batches; refused ones learn the server's.
async fn flags(imap: &mut Imap, group: &mut [Entry]) -> Result<()> {
    for (flag, part) in [("\\Seen", seen as Part), ("\\Flagged", flagged)] {
        for on in [true, false] {
            let picked = live(group, |each| part(each) == Some(on));
            if picked.is_empty() {
                continue;
            }
            let uids = uids_at(group, &picked);
            if let Err(why) = refused(imap.mark(&uids, flag, on).await)? {
                for &at in &picked {
                    group[at].settled.dropped.push(why.clone());
                    group[at].settled.refused.push(flag);
                }
            }
        }
    }
    let undone: Vec<usize> = (0..group.len())
        .filter(|&at| !group[at].settled.refused.is_empty())
        .collect();
    if undone.is_empty() {
        return Ok(());
    }
    let uids = uids_at(group, &undone);
    match refused(imap.flags_of(&uids).await)? {
        Ok(server) => {
            for at in undone {
                group[at].settled.server = server.get(&group[at].pending.uid).cloned();
            }
        }
        // Unreadable: the last known state returns.
        Err(why) => log::warn!("mail: flags not read back after a refusal: {why:?}"),
    }
    Ok(())
}

/// COPY, kept at once; then the source goes.
async fn copy(
    imap: &mut Imap,
    store: &Mutex<Store>,
    account: AccountId,
    group: &mut Vec<Entry>,
    picked: &[usize],
    name: &str,
    can: Can,
) -> Result<()> {
    let uids = uids_at(group, picked);
    let moved = match refused(imap.copy_to(&uids, name).await)? {
        Ok(moved) => moved,
        Err(why) => {
            refuse(group, picked, why, true);
            return Ok(());
        }
    };
    let mut sources = Vec::new();
    for &at in picked {
        let new = moved.uids.get(&group[at].pending.uid);
        let new = new.zip(moved.uidvalidity).map(|(uid, at)| (at, *uid));
        group[at].settled.done = Done::Moved(new);
        // A retry only erases; it copies no more.
        let source = lock(store)?.copied(account, &group[at].pending, &group[at].settled)?;
        group[at].kept = true;
        sources.push(Entry {
            pending: source,
            settled: Settled::waiting(),
            kept: false,
        });
    }
    let first = group.len();
    group.extend(sources);
    let sources: Vec<usize> = (first..group.len()).collect();
    match refused(imap.erase(&uids, can).await)? {
        Ok(()) => sources
            .iter()
            .for_each(|&at| group[at].settled.done = Done::Erased),
        Err(why) => refuse(group, &sources, why, false),
    }
    Ok(())
}

/// A MOVE that named no UID: found by Message-ID.
async fn find(imap: &mut Imap, group: &mut [Entry]) -> Result<()> {
    let mut targets: BTreeMap<i64, (String, Vec<usize>)> = BTreeMap::new();
    for (at, each) in group.iter().enumerate() {
        if let (Done::Moved(None), false, Some(_), Some((id, name))) = (
            each.settled.done,
            each.kept,
            &each.pending.message_id,
            &each.pending.target,
        ) {
            let entry = targets.entry(id.0).or_insert((name.clone(), Vec::new()));
            entry.1.push(at);
        }
    }
    for (name, picked) in targets.into_values() {
        let Ok(opened) = refused(imap.select(&name).await)? else {
            continue;
        };
        for at in picked {
            let id = group[at].pending.message_id.clone().expect("a Message-ID");
            // One match is it; more cannot tell.
            if let Ok(found) = refused(imap.with_message_id(&id).await)?
                && let [uid] = found[..]
            {
                log::info!("mail: a moved message found by its Message-ID");
                group[at].settled.done = Done::Moved(Some((opened.uidvalidity, uid)));
            }
        }
    }
    Ok(())
}

fn settle(store: &Mutex<Store>, account: AccountId, group: &[Entry]) -> Result<usize> {
    let mut held = lock(store)?;
    let mut took = 0;
    for each in group {
        if !each.kept {
            held.pushed(account, &each.pending, &each.settled)?;
        }
        if each.kept || !matches!(each.settled.done, Done::Waiting | Done::Moot) {
            took += 1;
        }
    }
    Ok(took)
}

type Part = fn(&Pending) -> Option<bool>;

fn seen(each: &Pending) -> Option<bool> {
    each.seen
}

fn flagged(each: &Pending) -> Option<bool> {
    each.flagged
}

fn stayed(move_refused: bool) -> Settled {
    Settled {
        done: Done::Stayed { move_refused },
        ..Settled::waiting()
    }
}

/// Places of changes not yet settled that `wants`.
fn live(group: &[Entry], wants: impl Fn(&Pending) -> bool) -> Vec<usize> {
    (0..group.len())
        .filter(|&at| group[at].settled.done == Done::Waiting && wants(&group[at].pending))
        .collect()
}

fn uids_at(group: &[Entry], places: &[usize]) -> Vec<u32> {
    places.iter().map(|&at| group[at].pending.uid).collect()
}

fn refuse(group: &mut [Entry], places: &[usize], why: Dropped, moving: bool) {
    for &at in places {
        group[at].settled.done = Done::Stayed {
            move_refused: moving,
        };
        group[at].settled.dropped.push(why.clone());
    }
}

/// Refused before anything ran: every change goes back.
fn refuse_all(group: &mut [Entry], why: Dropped) {
    for each in group {
        for (flag, part) in [("\\Seen", seen as Part), ("\\Flagged", flagged)] {
            if part(&each.pending).is_some() {
                each.settled.refused.push(flag);
            }
        }
        each.settled.done = Done::Stayed {
            move_refused: each.pending.target.is_some(),
        };
        each.settled.dropped.push(why.clone());
    }
}

/// UIDs void: nothing here holds on the server.
fn drop_all(group: &mut [Entry], why: Dropped) {
    for each in group {
        each.settled.done = Done::Moot;
        each.settled.dropped.push(why.clone());
    }
}

/// A refusal ends the change; other errors stop.
fn refused<T>(done: Result<T>) -> Result<std::result::Result<T, Dropped>> {
    match done {
        Ok(value) => Ok(Ok(value)),
        Err(Error::Server(words)) => Ok(Err(Dropped::Refused(words))),
        Err(Error::Unsupported(name)) => Ok(Err(Dropped::Unsupported(name.into()))),
        Err(other) => Err(other),
    }
}
