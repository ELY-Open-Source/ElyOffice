//! Mail a rebuild brings back is known again, not news.

use std::path::Path;

use rusqlite::{Transaction, params};
use sha2::{Digest, Sha256};

use super::messages::load_at;
use crate::{Error, Mail, Result, read};

/// Marks mail known in `mailbox`; unnamed whole mail keeps body.
pub(super) fn record(mailbox: &str) -> String {
    format!(
        "INSERT OR IGNORE INTO rebuilt (mailbox, ident, body)
    SELECT {mailbox}, m.ident, CASE WHEN m.message_id IS NULL OR m.message_id = ''
        THEN CASE WHEN m.content IS NOT NULL AND m.missing = '[]' THEN m.content ELSE '' END
        ELSE '' END
    FROM messages m"
    )
}

/// Mail a rebuild brought back; bodies compared only when whole.
pub(super) const REBUILT: &str = "EXISTS (SELECT 1 FROM rebuilt r
    WHERE r.mailbox = m.mailbox AND r.ident = m.ident
    AND (r.body = '' OR m.content IS NULL OR m.missing <> '[]' OR r.body = m.content))";

/// Message-ID, else the header block: the same however kept.
pub(super) fn ident(mail: &Mail) -> String {
    match mail.message_id.as_deref().filter(|id| !id.is_empty()) {
        Some(id) => id.to_owned(),
        None => {
            let head = mail.headers.trim_end_matches(['\r', '\n']);
            let hash: String = Sha256::digest(head.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect();
            format!("unnamed:{hash}")
        }
    }
}

/// Gives every message stored before v18 its identity.
pub(super) fn fill(tx: &Transaction, dir: &Path) -> Result<usize> {
    let rows: Vec<(i64, Option<String>, Option<Vec<u8>>)> = tx
        .prepare("SELECT id, content, head FROM messages")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (id, content, head) in &rows {
        let bytes = match (content, head) {
            (Some(hash), _) => load_at(dir, hash)?,
            (None, Some(head)) => head.clone(),
            (None, None) => return Err(Error::Store(format!("message {id} kept neither original nor headers"))),
        };
        tx.execute("UPDATE messages SET ident = ?1 WHERE id = ?2", params![ident(&read(&bytes)?), id])?;
    }
    Ok(rows.len())
}
