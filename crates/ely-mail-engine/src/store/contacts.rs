//! Contacts: kept by hand, imported, or met by sending (MAIL-12).

use rusqlite::{OptionalExtension, Transaction, params};

use super::{Store, now};
use crate::{Error, Result, compose::is_address, vcard};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ContactId(pub i64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contact {
    pub id: ContactId,
    pub name: Option<String>,
    pub address: String,
    /// Messages sent to it from here.
    pub sent: u32,
}

/// What an import changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Imported {
    pub added: usize,
    pub renamed: usize,
    pub unchanged: usize,
    /// Cards with no address, left out.
    pub without_address: usize,
}

impl Store {
    /// Every contact, the most written to first.
    pub fn contacts(&self) -> Result<Vec<Contact>> {
        let mut query = self.conn.prepare(
            "SELECT id, name, address, sent FROM contacts
             ORDER BY sent DESC, coalesce(name, address) COLLATE NOCASE, address COLLATE NOCASE",
        )?;
        let rows = query.query_map([], |row| {
            Ok(Contact { id: ContactId(row.get(0)?), name: row.get(1)?, address: row.get(2)?, sent: row.get(3)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Adds `address`, or names the one already kept.
    pub fn add_contact(&mut self, name: &str, address: &str) -> Result<ContactId> {
        self.writable()?;
        let (name, address) = checked(name, address)?;
        let tx = self.conn.transaction()?;
        let (id, _) = keep(&tx, name.as_deref(), &address)?;
        tx.commit()?;
        log::info!("mail: contact {} kept", id.0);
        Ok(id)
    }

    pub fn remove_contact(&mut self, id: ContactId) -> Result<()> {
        self.writable()?;
        match self.conn.execute("DELETE FROM contacts WHERE id = ?1", [id.0])? {
            1 => {
                log::info!("mail: contact {} removed", id.0);
                Ok(())
            }
            _ => Err(Error::Store(format!("no contact {}", id.0))),
        }
    }

    /// Every card's addresses, all or none.
    pub fn import_contacts(&mut self, cards: &[vcard::Card]) -> Result<Imported> {
        self.writable()?;
        let mut imported = Imported::default();
        let tx = self.conn.transaction()?;
        for card in cards {
            if card.emails.is_empty() {
                imported.without_address += 1;
            }
            for email in &card.emails {
                let (name, address) = checked(card.name.as_deref().unwrap_or(""), email)?;
                match keep(&tx, name.as_deref(), &address)? {
                    (_, Kept::Added) => imported.added += 1,
                    (_, Kept::Renamed) => imported.renamed += 1,
                    (_, Kept::Same) => imported.unchanged += 1,
                }
            }
        }
        tx.commit()?;
        log::info!("mail: contacts imported: {imported:?}");
        Ok(imported)
    }

    /// Every contact as vCard 3.0.
    pub fn export_contacts(&self) -> Result<String> {
        let contacts = self.contacts()?;
        let people = contacts.iter().map(|each| (each.name.as_deref(), each.address.as_str()));
        Ok(vcard::write(people))
    }
}

enum Kept {
    Added,
    Renamed,
    Same,
}

/// The trimmed name, none when it is the address.
fn checked(name: &str, address: &str) -> Result<(Option<String>, String)> {
    let address = address.trim();
    if !is_address(address) {
        return Err(Error::Invalid(format!("“{address}” is not an email address")));
    }
    let name = name.trim();
    if name.contains(['\r', '\n']) {
        return Err(Error::Invalid("a contact's name is one line".into()));
    }
    let name = (!name.is_empty() && !name.eq_ignore_ascii_case(address)).then(|| name.to_owned());
    Ok((name, address.to_owned()))
}

/// Inserts the address, or names the one kept.
fn keep(tx: &Transaction, name: Option<&str>, address: &str) -> Result<(ContactId, Kept)> {
    let found: Option<(i64, Option<String>)> = tx
        .query_row("SELECT id, name FROM contacts WHERE address = ?1", [address], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .optional()?;
    match found {
        None => {
            tx.execute("INSERT INTO contacts (address, name) VALUES (?1, ?2)", params![address, name])?;
            Ok((ContactId(tx.last_insert_rowid()), Kept::Added))
        }
        // A nameless card keeps the name already there.
        Some((id, old)) if name.is_none() || name == old.as_deref() => Ok((ContactId(id), Kept::Same)),
        Some((id, _)) => {
            tx.execute("UPDATE contacts SET name = ?2 WHERE id = ?1", params![id, name])?;
            Ok((ContactId(id), Kept::Renamed))
        }
    }
}

/// Counts a message sent to each address.
pub(super) fn remember<'a>(tx: &Transaction, addresses: impl IntoIterator<Item = &'a String>) -> Result<()> {
    let at = now()?;
    let mut counted: Vec<String> = Vec::new();
    for address in addresses {
        // Once per message, however many fields name it.
        let address = address.trim();
        if counted.iter().any(|seen| seen.eq_ignore_ascii_case(address)) {
            continue;
        }
        counted.push(address.to_owned());
        tx.execute(
            "INSERT INTO contacts (address, sent, last) VALUES (?1, 1, ?2)
             ON CONFLICT (address) DO UPDATE SET sent = sent + 1, last = excluded.last",
            params![address, at],
        )?;
    }
    Ok(())
}
