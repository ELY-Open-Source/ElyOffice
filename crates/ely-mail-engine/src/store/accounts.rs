//! Accounts: added, listed, and removed with their cache.

use std::{fs, io::ErrorKind};

use rusqlite::params;

use super::{AccountId, CONTENT, Store, aside::Aside, disk};
use crate::{Account, Error, Result};

/// What removing an account deletes here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cached {
    pub messages: u64,
    /// Originals no other account shares.
    pub bytes: u64,
}

impl Store {
    /// Taken: the same address on the same incoming server.
    pub fn add_account(&self, account: &Account) -> Result<AccountId> {
        self.writable()?;
        let settings = serde_json::to_string(account)?;
        let added = self.conn.execute(
            "INSERT INTO accounts (address, server, settings) VALUES (?1, ?2, ?3)",
            params![account.address, account.incoming.host, settings],
        );
        match added {
            Ok(_) => Ok(AccountId(self.conn.last_insert_rowid())),
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE =>
            {
                Err(Error::Taken(account.address.clone()))
            }
            Err(err) => Err(err.into()),
        }
    }

    pub fn accounts(&self) -> Result<Vec<(AccountId, Account)>> {
        let mut query = self
            .conn
            .prepare("SELECT id, settings FROM accounts ORDER BY id")?;
        let rows = query.query_map([], |row| {
            Ok((AccountId(row.get(0)?), row.get::<_, String>(1)?))
        })?;
        rows.map(|row| {
            let (id, settings) = row?;
            Ok((id, serde_json::from_str(&settings)?))
        })
        .collect()
    }

    /// Accounts whose folders the side folds away.
    pub fn folded(&self) -> Result<Vec<AccountId>> {
        let mut query = self
            .conn
            .prepare("SELECT id FROM accounts WHERE folded = 1 ORDER BY id")?;
        let rows = query.query_map([], |row| Ok(AccountId(row.get(0)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn set_folded(&self, account: AccountId, folded: bool) -> Result<()> {
        self.writable()?;
        let changed = self.conn.execute(
            "UPDATE accounts SET folded = ?1 WHERE id = ?2",
            params![folded, account.0],
        )?;
        if changed != 1 {
            return Err(Error::Store(format!("no account {}", account.0)));
        }
        log::info!("mail: account {} folded: {folded}", account.0);
        Ok(())
    }

    pub fn cached(&self, account: AccountId) -> Result<Cached> {
        let messages: i64 = self.conn.query_row(
            "SELECT count(*) FROM messages JOIN mailboxes ON messages.mailbox = mailboxes.id
             WHERE mailboxes.account = ?1",
            [account.0],
            |row| row.get(0),
        )?;
        let mut bytes = 0;
        for hash in self.only_in(account)? {
            let path = self.dir.join(CONTENT).join(&hash[..2]).join(&hash);
            bytes += match fs::metadata(path) {
                Ok(meta) => meta.len(),
                Err(err) if err.kind() == ErrorKind::NotFound => 0,
                Err(err) => return Err(disk("measuring an original", err)),
            };
        }
        let messages = u64::try_from(messages)
            .map_err(|_| Error::Store(format!("a negative count: {messages}")))?;
        Ok(Cached { messages, bytes })
    }

    /// The account, its mailboxes, messages and unshared originals.
    pub fn remove_account(&mut self, account: AccountId) -> Result<()> {
        self.writable()?;
        let hashes = self.only_in(account)?;
        let tx = self.conn.transaction()?;
        let mut aside = Aside::new(&self.dir);
        let done = (|| -> Result<()> {
            let gone = tx.execute("DELETE FROM accounts WHERE id = ?1", [account.0])?;
            if gone != 1 {
                return Err(Error::Store(format!("no account {}", account.0)));
            }
            for hash in &hashes {
                aside.drop_unused(&tx, hash)?;
            }
            Ok(tx.commit()?)
        })();
        aside.settle(done)?;
        log::info!(
            "mail: account {} removed with {} originals",
            account.0,
            hashes.len()
        );
        Ok(())
    }

    fn only_in(&self, account: AccountId) -> Result<Vec<String>> {
        let mut query = self.conn.prepare(
            "SELECT DISTINCT content FROM messages JOIN mailboxes ON messages.mailbox = mailboxes.id
             WHERE mailboxes.account = ?1 AND content IS NOT NULL
             AND NOT EXISTS (SELECT 1 FROM messages AS other
                 JOIN mailboxes AS theirs ON other.mailbox = theirs.id
                 WHERE other.content = messages.content AND theirs.account <> ?1)
             ORDER BY content",
        )?;
        let rows = query.query_map([account.0], |row| row.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}
