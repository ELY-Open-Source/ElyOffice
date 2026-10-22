//! Local rules: new inbox mail moved, flagged or archived (MAIL-11).

use rusqlite::params;

use super::{AccountId, Change, MailboxId, MessageId, Store, rebuilt::REBUILT};
use crate::{Error, Mailbox, Result, Role, imap::utf7};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RuleId(pub i64);

/// What a rule reads of a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Match {
    /// A sender's name or address.
    From,
    /// A To or Cc name or address.
    To,
    Subject,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Move(MailboxId),
    Flag,
    Archive,
}

/// Mail whose `field` holds `words`, any case, gets `action`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub id: RuleId,
    pub field: Match,
    pub words: String,
    pub action: Action,
    /// The folder a move goes to, as people read it.
    pub folder: Option<String>,
}

impl Match {
    fn name(self) -> &'static str {
        match self {
            Self::From => "from",
            Self::To => "to",
            Self::Subject => "subject",
        }
    }

    fn named(name: &str) -> Result<Self> {
        match name {
            "from" => Ok(Self::From),
            "to" => Ok(Self::To),
            "subject" => Ok(Self::Subject),
            other => Err(Error::Store(format!("an unknown rule field {other:?}"))),
        }
    }
}

/// What a full sync brought that is news.
#[derive(Clone, Debug, PartialEq)]
pub struct Arrivals {
    /// Unread inbox mail left after the rules.
    pub arrived: Vec<super::Summary>,
    /// Messages the rules moved or flagged.
    pub ruled: usize,
    /// Why rules that could not act did not.
    pub refused: Vec<String>,
}

/// A new message as rules read it.
struct New {
    id: MessageId,
    from: Vec<Mailbox>,
    to: Vec<Mailbox>,
    subject: String,
}

impl New {
    fn holds(&self, field: Match, words: &str) -> bool {
        let has = |text: &str| text.to_lowercase().contains(words);
        let named = |people: &[Mailbox]| {
            people.iter().any(|one| has(&one.email) || one.name.as_deref().is_some_and(has))
        };
        match field {
            Match::From => named(&self.from),
            Match::To => named(&self.to),
            Match::Subject => has(&self.subject),
        }
    }
}

impl Store {
    /// `account`'s rules, in the order they were made.
    pub fn rules(&self, account: AccountId) -> Result<Vec<Rule>> {
        let mut query = self
            .conn
            .prepare(
                "SELECT r.id, r.field, r.words, r.action, r.target, b.name
                 FROM rules r LEFT JOIN mailboxes b ON b.id = r.target WHERE r.account = ?1 ORDER BY r.id",
            )?;
        let rows = query.query_map([account.0], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;
        rows.map(|row| {
            let (id, field, words, action, target, folder) = row?;
            let action = match (action.as_str(), target) {
                ("move", Some(_)) if folder.is_none() => {
                    return Err(Error::Store(format!("rule {id} moves to no folder")));
                }
                ("move", Some(target)) => Action::Move(MailboxId(target)),
                ("flag", None) => Action::Flag,
                ("archive", None) => Action::Archive,
                (action, _) => return Err(Error::Store(format!("an unknown rule action {action:?}"))),
            };
            let folder = folder.as_deref().map(utf7::decode).transpose()?;
            Ok(Rule { id: RuleId(id), field: Match::named(&field)?, words, action, folder })
        })
        .collect()
    }

    /// Adds a rule; a move names the account's own folder.
    pub fn add_rule(&mut self, account: AccountId, field: Match, words: &str, action: Action) -> Result<RuleId> {
        self.writable()?;
        let words = words.trim().to_lowercase();
        if words.is_empty() {
            return Err(Error::Invalid("a rule needs words to look for".into()));
        }
        let pop = self
            .accounts()?
            .into_iter()
            .find(|(each, _)| *each == account)
            .ok_or_else(|| Error::Store(format!("no account {}", account.0)))?
            .1
            .protocol;
        let pop = matches!(pop, crate::Protocol::Pop { .. });
        // POP3 keeps one folder: it can only flag.
        if pop && action != Action::Flag {
            return Err(Error::Invalid("a POP3 account's rules can only flag".into()));
        }
        let (name, target) = match action {
            Action::Move(target) => {
                // An imported folder is not on the server.
                if !self.folders(account)?.iter().any(|folder| folder.id == target && !folder.local) {
                    return Err(Error::Invalid("a rule moves only to its account's server folders".into()));
                }
                ("move", Some(target.0))
            }
            Action::Flag => ("flag", None),
            Action::Archive => {
                let archives = |folder: &crate::LocalFolder| matches!(folder.role, Some(Role::Archive | Role::All));
                if !self.folders(account)?.iter().any(|folder| !folder.local && archives(folder)) {
                    return Err(Error::Invalid("the server has no Archive folder".into()));
                }
                ("archive", None)
            }
        };
        self.conn.execute(
            "INSERT INTO rules (account, field, words, action, target) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![account.0, field.name(), words, name, target],
        )?;
        let id = RuleId(self.conn.last_insert_rowid());
        log::info!("mail: account {} gained rule {}", account.0, id.0);
        Ok(id)
    }

    pub fn remove_rule(&mut self, id: RuleId) -> Result<()> {
        self.writable()?;
        match self.conn.execute("DELETE FROM rules WHERE id = ?1", [id.0])? {
            1 => Ok(log::info!("mail: rule {} removed", id.0)),
            _ => Err(Error::Store(format!("no rule {}", id.0))),
        }
    }

    /// After a full sync: rules, then news, past `handled`.
    pub fn handle_arrivals(&mut self, account: AccountId) -> Result<Arrivals> {
        let after = MessageId(self.conn.query_row("SELECT handled FROM accounts WHERE id = ?1", [account.0], |row| row.get(0))?);
        let newest = self.newest()?;
        let (ruled, refused) = self.apply_rules(account, after)?;
        let arrived = self.arrived(account, after)?;
        self.set_handled(account, newest)?;
        Ok(Arrivals { arrived, ruled, refused })
    }

    /// The first sync brings old mail, none of it news.
    pub fn skip_arrivals(&mut self, account: AccountId) -> Result<()> {
        let newest = self.newest()?;
        self.set_handled(account, newest)
    }

    fn set_handled(&mut self, account: AccountId, newest: MessageId) -> Result<()> {
        self.writable()?;
        let tx = self.conn.transaction()?;
        let changed = tx.execute("UPDATE accounts SET handled = ?2 WHERE id = ?1", params![account.0, newest.0])?;
        if changed != 1 {
            return Err(Error::Store(format!("no account {}", account.0)));
        }
        // Handled, a rebuild is past.
        tx.execute(
            "DELETE FROM rebuilt WHERE mailbox IN (SELECT id FROM mailboxes WHERE account = ?1)",
            [account.0],
        )?;
        Ok(tx.commit()?)
    }

    /// Rules on inbox mail past `after`: acted count, refusals.
    fn apply_rules(&mut self, account: AccountId, after: MessageId) -> Result<(usize, Vec<String>)> {
        let rules = self.rules(account)?;
        if rules.is_empty() {
            return Ok((0, Vec::new()));
        }
        let new = self.new_inbox_mail(account, after)?;
        let mut acting: Vec<(Action, Vec<MessageId>)> = Vec::new();
        for message in &new {
            // The first rule that matches decides.
            let Some(rule) = rules.iter().find(|rule| message.holds(rule.field, &rule.words)) else {
                continue;
            };
            match acting.iter_mut().find(|(action, _)| *action == rule.action) {
                Some((_, ids)) => ids.push(message.id),
                None => acting.push((rule.action, vec![message.id])),
            }
        }
        let (mut count, mut refused) = (0, Vec::new());
        for (action, ids) in acting {
            let change = match action {
                Action::Move(target) => Change::Move(target),
                Action::Flag => Change::Flagged(true),
                Action::Archive => Change::Archive,
            };
            // A rule that cannot act leaves the others be.
            match self.change(&ids, change) {
                Ok(_) => count += ids.len(),
                Err(Error::Invalid(reason)) => {
                    log::warn!("mail: a rule could not act on {} messages: {reason}", ids.len());
                    refused.push(reason);
                }
                Err(error) => return Err(error),
            }
        }
        log::info!("mail: rules acted on {count} of {} new messages", new.len());
        Ok((count, refused))
    }

    fn new_inbox_mail(&self, account: AccountId, after: MessageId) -> Result<Vec<New>> {
        let mut query = self.conn.prepare(&format!(
            "SELECT m.id, m.sender, m.recipients, m.copies, m.subject
             FROM messages m JOIN mailboxes b ON b.id = coalesce(m.moving, m.mailbox)
             WHERE NOT m.erasing AND b.role = 'inbox' AND b.account = ?1 AND m.id > ?2
             AND NOT {REBUILT}
             ORDER BY m.id"
        ))?;
        let rows = query.query_map(params![account.0, after.0], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?;
        rows.map(|row| {
            let (id, from, to, cc, subject) = row?;
            let mut to: Vec<Mailbox> = serde_json::from_str(&to)?;
            to.extend(serde_json::from_str::<Vec<Mailbox>>(&cc)?);
            Ok(New { id: MessageId(id), from: serde_json::from_str(&from)?, to, subject: subject.unwrap_or_default() })
        })
        .collect()
    }
}
