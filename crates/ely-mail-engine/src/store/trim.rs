//! Downloads back to headers, by hand or by cap (§18.3).

use std::{
    collections::{HashMap, HashSet},
    fs,
};

use super::{CONTENT, MessageId, Store, disk, messages::Kept};
use crate::{Error, Result, read};

/// Server mail whose body may go: not offline-bound.
const SPARE: &str = "SELECT m.id, m.content FROM messages m JOIN mailboxes b ON b.id = m.mailbox
    LEFT JOIN mailboxes t ON t.id = m.moving
    WHERE m.content IS NOT NULL AND m.uid IS NOT NULL AND NOT m.erasing
    AND NOT b.offline AND NOT coalesce(t.offline, 0)";

impl Store {
    /// Every spare download to headers; syncs bring recent ones back.
    pub fn forget_downloads(&mut self) -> Result<usize> {
        self.writable()?;
        let spare = self.spare("")?;
        for (id, _) in &spare {
            self.keep_headers(*id)?;
        }
        log::info!("mail: {} downloaded messages forgotten", spare.len());
        Ok(spare.len())
    }

    /// Oldest spare downloads go until the rest fit, while `still`.
    pub fn trim_downloads(&mut self, cap: u64, still: impl Fn() -> bool) -> Result<usize> {
        self.writable()?;
        // Undated mail counts as oldest.
        let spare = self.spare(" ORDER BY m.date IS NOT NULL, m.date, m.id")?;
        // A file other mail holds is no spare room.
        let held: HashSet<String> = self
            .conn
            .prepare(&format!("SELECT content FROM messages WHERE content IS NOT NULL AND id NOT IN (SELECT id FROM ({SPARE}))"))?
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let mut left: HashMap<String, (usize, u64)> = HashMap::new();
        for (_, hash) in spare.iter().filter(|(_, hash)| !held.contains(hash)) {
            match left.get_mut(hash) {
                Some((uses, _)) => *uses += 1,
                None => {
                    let size = fs::metadata(self.path_of(hash)).map_err(|err| disk("measuring an original", err))?.len();
                    left.insert(hash.clone(), (1, size));
                }
            }
        }
        let mut total: u64 = left.values().map(|(_, size)| size).sum();
        let mut trimmed = 0;
        for (id, hash) in spare {
            if total <= cap {
                break;
            }
            // Trimming it frees nothing.
            if held.contains(&hash) {
                continue;
            }
            if !still() {
                log::info!("mail: a newer cap stopped the trim");
                break;
            }
            self.keep_headers(id)?;
            self.conn.execute("UPDATE messages SET trimmed = 1 WHERE id = ?1", [id.0])?;
            // Its file goes with the last spare message holding it.
            if let Some((uses, size)) = left.get_mut(&hash) {
                *uses -= 1;
                if *uses == 0 {
                    total -= *size;
                }
            }
            trimmed += 1;
        }
        log::info!("mail: {trimmed} downloads trimmed to fit {cap} bytes");
        Ok(trimmed)
    }

    fn spare(&self, order: &str) -> Result<Vec<(MessageId, String)>> {
        let rows = self
            .conn
            .prepare(&format!("{SPARE}{order}"))?
            .query_map([], |row| Ok((MessageId(row.get(0)?), row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    fn keep_headers(&mut self, id: MessageId) -> Result<()> {
        let stored = self.get(id)?;
        let original = self.original(id)?.ok_or_else(|| Error::Store(format!("message {} lost its original", id.0)))?;
        let mail = read(&original)?;
        let flags: Vec<&str> = stored.flags.iter().map(String::as_str).collect();
        let head = Kept::Head(mail.headers.as_bytes());
        self.write(stored.mailbox, &stored.key, &flags, &mail, head, Some(id))?;
        Ok(())
    }

    fn path_of(&self, hash: &str) -> std::path::PathBuf {
        self.dir.join(CONTENT).join(&hash[..2]).join(hash)
    }
}
