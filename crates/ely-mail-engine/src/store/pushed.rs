//! Queued changes as the server takes them (§12.3).

use rusqlite::{OptionalExtension, Transaction, params};

use super::{AccountId, MailboxId, MessageId, Store, aside::Aside, messages, rebuilt::record};
use crate::{Error, Result, push::Dropped};

/// A queued change, as read before it goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending {
    pub id: i64,
    /// A newer change here waits for another push.
    pub version: i64,
    pub mailbox: MailboxId,
    pub name: String,
    pub uidvalidity: u32,
    pub uid: u32,
    pub seen: Option<bool>,
    pub flagged: Option<bool>,
    /// The server's flags as last known.
    pub was: [Option<bool>; 2],
    pub target: Option<(MailboxId, String)>,
    pub expunge: bool,
    /// Finds it where a move left no UID.
    pub message_id: Option<String>,
}

impl Pending {
    pub fn empty(&self) -> bool {
        self.seen.is_none() && self.flagged.is_none() && self.target.is_none() && !self.expunge
    }
}

/// What the server did with one change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Done {
    /// Unknown yet: it stays queued.
    Waiting,
    /// Flags set, or parts refused; nothing moved.
    Stayed {
        move_refused: bool,
    },
    /// Where it went, when known.
    Moved(Option<(u32, u32)>),
    Erased,
    /// Gone or renumbered: nothing to change.
    Moot,
}

/// One change's outcome, settled in one transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settled {
    pub done: Done,
    pub dropped: Vec<Dropped>,
    /// Flags the server refused, and its own flags.
    pub refused: Vec<&'static str>,
    pub server: Option<Vec<String>>,
}

impl Settled {
    pub fn waiting() -> Self {
        Self {
            done: Done::Waiting,
            dropped: Vec::new(),
            refused: Vec::new(),
            server: None,
        }
    }
}

impl Store {
    /// The account's queue, by mailbox.
    pub(crate) fn queued_changes(&self, account: AccountId) -> Result<Vec<Pending>> {
        let mut query = self.conn.prepare(
            "SELECT c.id, c.version, c.mailbox, b.name, c.uidvalidity, c.uid, c.seen, c.flagged,
                    c.target, t.name, c.expunge, m.message_id, c.seen_was, c.flagged_was
             FROM changes c JOIN mailboxes b ON b.id = c.mailbox
             LEFT JOIN mailboxes t ON t.id = c.target
             LEFT JOIN messages m ON m.mailbox = c.mailbox AND m.uidvalidity = c.uidvalidity
                 AND m.uid = c.uid
             WHERE b.account = ?1 ORDER BY c.mailbox, c.uidvalidity, c.uid",
        )?;
        let rows = query.query_map([account.0], |row| {
            let target = match (row.get::<_, Option<i64>>(8)?, row.get(9)?) {
                (Some(id), Some(name)) => Some((MailboxId(id), name)),
                _ => None,
            };
            Ok(Pending {
                id: row.get(0)?,
                version: row.get(1)?,
                mailbox: MailboxId(row.get(2)?),
                name: row.get(3)?,
                uidvalidity: row.get(4)?,
                uid: row.get(5)?,
                seen: row.get(6)?,
                flagged: row.get(7)?,
                was: [row.get(12)?, row.get(13)?],
                target,
                expunge: row.get(10)?,
                message_id: row.get(11)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Settles one pushed change; a newer one stays queued.
    pub(crate) fn pushed(
        &mut self,
        account: AccountId,
        pending: &Pending,
        settled: &Settled,
    ) -> Result<()> {
        self.settling(|tx, aside| settle(tx, aside, account, pending, settled))
    }

    /// A copy made: the source's removal waits apart.
    pub(crate) fn copied(
        &mut self,
        account: AccountId,
        pending: &Pending,
        settled: &Settled,
    ) -> Result<Pending> {
        self.settling(|tx, aside| {
            settle(tx, aside, account, pending, settled)?;
            tx.execute(
                "INSERT INTO changes (mailbox, uidvalidity, uid, version, expunge)
                 VALUES (?1, ?2, ?3, 1, 1)",
                params![pending.mailbox.0, pending.uidvalidity, pending.uid],
            )?;
            Ok(Pending {
                id: tx.last_insert_rowid(),
                version: 1,
                seen: None,
                flagged: None,
                was: [None, None],
                target: None,
                expunge: true,
                message_id: None,
                ..pending.clone()
            })
        })
    }

    /// Changes let go since last asked, oldest first.
    pub fn take_dropped(&mut self, account: AccountId) -> Result<Vec<Dropped>> {
        self.writable()?;
        let tx = self.conn.transaction()?;
        let rows: Vec<(String, Option<String>)> = {
            let mut query =
                tx.prepare("SELECT kind, words FROM dropped WHERE account = ?1 ORDER BY id")?;
            let rows = query.query_map([account.0], |row| Ok((row.get(0)?, row.get(1)?)))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        tx.execute("DELETE FROM dropped WHERE account = ?1", [account.0])?;
        let dropped = rows
            .into_iter()
            .map(|(kind, words)| from_row(&kind, words))
            .collect::<Result<_>>()?;
        tx.commit()?;
        Ok(dropped)
    }

    fn settling<T>(
        &mut self,
        work: impl FnOnce(&Transaction, &mut Aside) -> Result<T>,
    ) -> Result<T> {
        self.writable()?;
        let tx = self.conn.transaction()?;
        let mut aside = Aside::new(&self.dir);
        let result = (|| -> Result<T> {
            let value = work(&tx, &mut aside)?;
            tx.commit()?;
            Ok(value)
        })();
        aside.settle(result)
    }
}

fn settle(
    tx: &Transaction,
    aside: &mut Aside,
    account: AccountId,
    pending: &Pending,
    settled: &Settled,
) -> Result<()> {
    let now: Option<(i64, Option<i64>, bool)> = tx
        .query_row(
            "SELECT version, target, expunge FROM changes WHERE id = ?1",
            [pending.id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let newer = now.is_some_and(|(version, ..)| version != pending.version);
    let message: Option<MessageId> = tx
        .query_row(
            "SELECT id FROM messages WHERE mailbox = ?1 AND uidvalidity = ?2 AND uid = ?3",
            params![pending.mailbox.0, pending.uidvalidity, pending.uid],
            |row| row.get(0).map(MessageId),
        )
        .optional()?;
    let mut dropped = settled.dropped.clone();
    let known = settled.done != Done::Waiting;
    if let (false, true, Some(id)) = (newer, known, message) {
        for flag in &settled.refused {
            let on = refused_back(pending, settled, flag)?;
            messages::put_flag(tx, id, flag, on)?;
        }
    }
    let forget = |tx: &Transaction| tx.execute("DELETE FROM changes WHERE id = ?1", [pending.id]);
    match settled.done {
        Done::Waiting => return Ok(()),
        Done::Erased => {
            forget(tx)?;
            if let Some(id) = message {
                messages::delete(tx, aside, id)?;
            }
        }
        Done::Moot => {
            forget(tx)?;
            match (pending.expunge, message) {
                (true, Some(id)) => messages::delete(tx, aside, id)?,
                _ => unmove(tx, message)?,
            }
        }
        Done::Stayed { move_refused } => {
            if move_refused && !newer {
                unmove(tx, message)?;
            }
            if let (true, false, Some(id)) = (pending.expunge, newer, message) {
                // Refused: it shows again.
                tx.execute("UPDATE messages SET erasing = 0 WHERE id = ?1", [id.0])?;
            }
            if !newer {
                forget(tx)?;
            }
        }
        Done::Moved(None) => {
            // The target's sync brings it down.
            forget(tx)?;
            if let (Some(id), Some((target, _))) = (message, &pending.target) {
                // Handled mail coming down anew is no news.
                tx.execute(
                    &format!(
                        "{} WHERE m.id = ?2 AND m.id <= (SELECT handled FROM accounts WHERE id = ?3)",
                        record("?1")
                    ),
                    params![target.0, id.0, account.0],
                )?;
            }
            if let Some(id) = message {
                messages::delete(tx, aside, id)?;
            }
            if newer {
                log::warn!("mail: a change to a message moved without a new UID was let go");
                dropped.push(Dropped::Unplaced);
            }
        }
        Done::Moved(Some((uidvalidity, uid))) => {
            let (target, _) = pending
                .target
                .clone()
                .ok_or_else(|| Error::Store("a move without its target".into()))?;
            // A newer choice wins over where it went.
            let wanted = match now {
                Some((_, _, true)) if newer => target,
                Some((_, Some(wanted), _)) if newer => MailboxId(wanted),
                Some((_, None, _)) if newer => pending.mailbox,
                _ => target,
            };
            let moving = (wanted != target).then_some(wanted.0);
            if let Some(id) = message {
                tx.execute(
                    "UPDATE messages SET mailbox = ?1, uidvalidity = ?2, uid = ?3, moving = ?4
                     WHERE id = ?5",
                    params![target.0, uidvalidity, uid, moving, id.0],
                )?;
            }
            match newer {
                false => {
                    forget(tx)?;
                }
                true => {
                    tx.execute(
                        "UPDATE changes SET mailbox = ?1, uidvalidity = ?2, uid = ?3, target = ?4
                         WHERE id = ?5",
                        params![target.0, uidvalidity, uid, moving, pending.id],
                    )?;
                }
            }
        }
    }
    tx.execute(
        "DELETE FROM changes WHERE id = ?1 AND seen IS NULL AND flagged IS NULL
         AND target IS NULL AND NOT expunge",
        [pending.id],
    )?;
    for each in &dropped {
        let (kind, words) = to_row(each);
        tx.execute(
            "INSERT INTO dropped (account, kind, words) VALUES (?1, ?2, ?3)",
            params![account.0, kind, words],
        )?;
    }
    Ok(())
}

/// A refused flag as the server has it.
fn refused_back(pending: &Pending, settled: &Settled, flag: &str) -> Result<bool> {
    if let Some(server) = &settled.server {
        // RFC 3501 §2.3.2: system flags match in any case.
        return Ok(server.iter().any(|it| it.eq_ignore_ascii_case(flag)));
    }
    let at = usize::from(flag.eq_ignore_ascii_case("\\Flagged"));
    pending.was[at].ok_or_else(|| Error::Store(format!("{flag} queued without its old state")))
}

fn unmove(tx: &Transaction, message: Option<MessageId>) -> Result<()> {
    if let Some(id) = message {
        tx.execute("UPDATE messages SET moving = NULL WHERE id = ?1", [id.0])?;
    }
    Ok(())
}

fn to_row(dropped: &Dropped) -> (&'static str, Option<&str>) {
    match dropped {
        Dropped::Gone => ("gone", None),
        Dropped::Renumbered => ("renumbered", None),
        Dropped::Refused(words) => ("refused", Some(words)),
        Dropped::Unsupported(name) => ("unsupported", Some(name)),
        Dropped::Unplaced => ("unplaced", None),
        Dropped::Vanished(name) => ("vanished", Some(name)),
    }
}

fn from_row(kind: &str, words: Option<String>) -> Result<Dropped> {
    let said = || words.ok_or_else(|| Error::Store(format!("a let-go {kind} without its words")));
    Ok(match kind {
        "gone" => Dropped::Gone,
        "renumbered" => Dropped::Renumbered,
        "refused" => Dropped::Refused(said()?),
        "unsupported" => Dropped::Unsupported(said()?),
        "unplaced" => Dropped::Unplaced,
        "vanished" => Dropped::Vanished(said()?),
        other => return Err(Error::Store(format!("an unknown let-go kind: {other}"))),
    })
}
