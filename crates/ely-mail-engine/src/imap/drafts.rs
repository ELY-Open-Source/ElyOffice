//! Copies placed on the server: drafts, sent mail (C12.13, C12.14).

use crate::Stage;
use imap_proto::{Response, ResponseCode, rfc4315::UidSetMember};

use super::{Arg, Imap};
use crate::{Error, Remote, Result, net::within};

impl Imap {
    /// Places `message` as a draft; RFC 4315 says where.
    pub async fn put_draft(&mut self, mailbox: &str, message: &[u8]) -> Result<Remote> {
        self.needs_uidplus().await?;
        let placed = self
            .append(mailbox, "(\\Draft \\Seen)", message, Stage::SavingDraft)
            .await?;
        let (uidvalidity, uid) =
            placed.ok_or_else(|| Error::Protocol("the saved draft came without its UID".into()))?;
        log::info!("mail: a draft was placed on the server");
        Ok(Remote {
            mailbox: mailbox.to_owned(),
            uidvalidity,
            uid,
        })
    }

    /// Files a sent message, read already.
    pub async fn put_sent(&mut self, mailbox: &str, message: &[u8]) -> Result<()> {
        self.append(mailbox, "(\\Seen)", message, Stage::SavingSentCopy)
            .await?;
        log::info!("mail: a sent copy was placed on the server");
        Ok(())
    }

    /// RFC 3501 APPEND; UIDPLUS adds where it went.
    async fn append(
        &mut self,
        mailbox: &str,
        flags: &str,
        message: &[u8],
        step: Stage,
    ) -> Result<Option<(u32, u32)>> {
        let mut placed = None;
        let args = [Arg::Text(mailbox), Arg::Atom(flags), Arg::Bytes(message)];
        within(
            step,
            self.conn.run("APPEND", &args, |response, _| {
                if let Response::Done { outcome, .. } = response
                    && let Some(ResponseCode::AppendUid(validity, uids)) = &outcome.code
                    && let [UidSetMember::Uid(uid)] = uids.as_slice()
                {
                    placed = Some((*validity, *uid));
                }
            }),
        )
        .await??
        .map_err(Error::Server)?;
        Ok(placed)
    }

    /// Removes that copy; false when renumbered, so unknown.
    pub async fn drop_draft(&mut self, remote: &Remote) -> Result<bool> {
        self.needs_uidplus().await?;
        let opened = self
            .open("SELECT", Stage::OpeningDrafts, &remote.mailbox)
            .await?;
        if opened.uidvalidity != remote.uidvalidity {
            log::warn!("mail: the drafts folder was renumbered; its old copy stays");
            return Ok(false);
        }
        let uid = remote.uid.to_string();
        let mark = [
            Arg::Atom(&uid),
            Arg::Atom("+FLAGS.SILENT"),
            Arg::Atom("(\\Deleted)"),
        ];
        within(
            Stage::MarkingOldDraft,
            self.conn.run("UID STORE", &mark, |_, _| {}),
        )
        .await??
        .map_err(Error::Server)?;
        within(
            Stage::RemovingOldDraft,
            self.conn.run("UID EXPUNGE", &[Arg::Atom(&uid)], |_, _| {}),
        )
        .await??
        .map_err(Error::Server)?;
        log::info!("mail: an old draft left the server");
        Ok(true)
    }

    /// Only UIDPLUS names one message to remove.
    async fn needs_uidplus(&mut self) -> Result<()> {
        let can = self.capabilities().await?;
        match can.iter().any(|it| it.eq_ignore_ascii_case("UIDPLUS")) {
            true => Ok(()),
            false => Err(Error::Unsupported("UIDPLUS")),
        }
    }
}
