//! Flags, moves and removals on the server (C12.15).

use crate::Stage;
use std::collections::{HashMap, HashSet};

use imap_proto::{Response, ResponseCode, rfc4315::UidSetMember};

use super::{Arg, Imap, Opened, set};
use crate::{Error, Result, net::within};

/// The extensions that change how mail moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Can {
    /// RFC 6851 MOVE.
    pub moves: bool,
    /// RFC 4315: one message expunged by UID.
    pub uidplus: bool,
}

/// Where moved messages went, when the server said.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Moved {
    pub uidvalidity: Option<u32>,
    /// Old UID to new.
    pub uids: HashMap<u32, u32>,
}

impl Imap {
    pub async fn can(&mut self) -> Result<Can> {
        let can = self.capabilities().await?;
        let has = |name: &str| can.iter().any(|it| it.eq_ignore_ascii_case(name));
        Ok(Can {
            moves: has("MOVE"),
            uidplus: has("UIDPLUS"),
        })
    }

    /// SELECT: the mailbox opened to change.
    pub async fn select(&mut self, mailbox: &str) -> Result<Opened> {
        self.open("SELECT", Stage::OpeningMailboxToChange, mailbox)
            .await
    }

    /// Which of `uids` the open mailbox still holds.
    pub async fn present(&mut self, uids: &[u32]) -> Result<HashSet<u32>> {
        let got = self
            .uid_fetch(&set(uids), "(UID)", Stage::CheckingMessages)
            .await?;
        Ok(got.into_keys().filter(|uid| uids.contains(uid)).collect())
    }

    /// Sets or clears one flag on `uids`.
    pub async fn mark(&mut self, uids: &[u32], flag: &str, on: bool) -> Result<()> {
        let how = if on { "+FLAGS.SILENT" } else { "-FLAGS.SILENT" };
        let (uid_set, flags) = (set(uids), format!("({flag})"));
        let args = [Arg::Atom(&uid_set), Arg::Atom(how), Arg::Atom(&flags)];
        within(
            Stage::ChangingFlags,
            self.conn.run("UID STORE", &args, |_, _| {}),
        )
        .await??
        .map_err(Error::Server)
    }

    /// Each UID's flags; a UID gone meanwhile is absent.
    pub async fn flags_of(&mut self, uids: &[u32]) -> Result<HashMap<u32, Vec<String>>> {
        let got = self
            .uid_fetch(&set(uids), "(UID FLAGS)", Stage::ReadingFlags)
            .await?;
        Ok(got.into_iter().map(|(uid, it)| (uid, it.flags)).collect())
    }

    /// UIDs whose Message-ID header holds `<id>`.
    pub async fn with_message_id(&mut self, id: &str) -> Result<Vec<u32>> {
        let quoted = format!("<{id}>");
        let args = [
            Arg::Atom("HEADER"),
            Arg::Atom("Message-ID"),
            Arg::Text(&quoted),
        ];
        self.search(&args, Stage::FindingMovedMessage).await
    }

    /// RFC 6851 MOVE.
    pub async fn move_to(&mut self, uids: &[u32], target: &str) -> Result<Moved> {
        self.carry("UID MOVE", uids, target).await
    }

    /// RFC 4315 COPY; the source stays until erased.
    pub async fn copy_to(&mut self, uids: &[u32], target: &str) -> Result<Moved> {
        self.carry("UID COPY", uids, target).await
    }

    async fn carry(&mut self, verb: &str, uids: &[u32], target: &str) -> Result<Moved> {
        let uid_set = set(uids);
        let mut said = None;
        let args = [Arg::Atom(&uid_set), Arg::Text(target)];
        within(
            Stage::MovingMessages,
            self.conn.run(verb, &args, |response, _| {
                let outcome = match response {
                    Response::Data { outcome, .. } | Response::Done { outcome, .. } => outcome,
                    _ => return,
                };
                if let Some(ResponseCode::CopyUid(validity, from, to)) = &outcome.code {
                    said = Some(pairs(*validity, from, to, uids));
                }
            }),
        )
        .await??
        .map_err(Error::Server)?;
        let moved = said.transpose()?.unwrap_or_default();
        log::info!(
            "mail: {verb} of {} messages, {} renumbered",
            uids.len(),
            moved.uids.len()
        );
        Ok(moved)
    }

    /// Removes `uids` for good; only UIDPLUS names them.
    pub async fn erase(&mut self, uids: &[u32], can: Can) -> Result<()> {
        if !can.uidplus {
            return Err(Error::Unsupported("UIDPLUS"));
        }
        self.expunge(uids).await?;
        log::info!("mail: {} messages removed for good", uids.len());
        Ok(())
    }

    async fn expunge(&mut self, uids: &[u32]) -> Result<()> {
        self.mark(uids, "\\Deleted", true).await?;
        let uid_set = set(uids);
        within(
            Stage::RemovingMessages,
            self.conn
                .run("UID EXPUNGE", &[Arg::Atom(&uid_set)], |_, _| {}),
        )
        .await??
        .map_err(Error::Server)
    }
}

/// RFC 4315 §3: the sets pair in order.
fn pairs(
    validity: u32,
    from: &[UidSetMember],
    to: &[UidSetMember],
    asked: &[u32],
) -> Result<Moved> {
    let (from, to) = (expand(from, asked.len())?, expand(to, asked.len())?);
    if from.len() != to.len() {
        return Err(Error::Protocol("COPYUID sets differ in size".into()));
    }
    Ok(Moved {
        uidvalidity: Some(validity),
        uids: from
            .into_iter()
            .zip(to)
            .filter(|(old, _)| asked.contains(old))
            .collect(),
    })
}

/// No more than was asked; a server cannot balloon it.
fn expand(set: &[UidSetMember], most: usize) -> Result<Vec<u32>> {
    let mut out = Vec::new();
    for member in set {
        let range = match member {
            UidSetMember::Uid(uid) => *uid..=*uid,
            UidSetMember::UidRange(range) => range.clone(),
        };
        let count = range.end().saturating_sub(*range.start()) as usize + 1;
        if out.len() + count > most {
            return Err(Error::Protocol("COPYUID names more than was moved".into()));
        }
        out.extend(range);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use imap_proto::rfc4315::UidSetMember;

    use super::pairs;

    #[test]
    fn copyuid_pairs_in_order_and_cannot_balloon() {
        let from = [UidSetMember::UidRange(3..=4), UidSetMember::Uid(9)];
        let to = [UidSetMember::UidRange(40..=42)];
        let moved = pairs(7, &from, &to, &[3, 4, 9]).unwrap();
        assert_eq!(moved.uidvalidity, Some(7));
        assert_eq!(moved.uids[&4], 41);
        assert_eq!(moved.uids[&9], 42);
        let huge = [UidSetMember::UidRange(1..=u32::MAX)];
        assert!(pairs(7, &huge, &huge, &[1]).is_err());
    }
}
