//! Loading documents: by URL, with a password, or as text.

use std::io;

use super::{Doc, Worker};
use crate::ipc::{Pack, Unpack, tag};

/// A load refused: the file's package is broken.
#[derive(Debug)]
pub struct Damaged;

impl std::fmt::Display for Damaged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(crate::ipc::DAMAGED)
    }
}

impl std::error::Error for Damaged {}

impl Worker {
    /// Loads `url`; macros never run (LOK-11).
    pub fn load(&mut self, url: &str) -> io::Result<Doc> {
        self.unlock(url, "")
    }

    /// Loads text `url` as a sheet, read as `how` says.
    pub fn import(&mut self, url: &str, how: &crate::sheet::csv::Import) -> io::Result<Doc> {
        use crate::sheet::csv::{COLUMNS, ENCODINGS};
        let at = |found: Option<usize>| found.expect("a listed choice") as u64;
        let pack = Pack::new()
            .str(url)
            .u64(u64::from(how.separator))
            .u64(how.quote.map_or(0, u64::from))
            .u64(at(ENCODINGS.iter().position(|e| *e == how.encoding)))
            .u64(how.columns.len() as u64);
        let pack = how.columns.iter().fold(pack, |pack, column| {
            pack.u64(at(COLUMNS.iter().position(|c| c == column)))
        });
        let payload = self.request(tag::IMPORT, &pack.bytes())?;
        self.opened(&payload)
    }

    /// Loads `url` with its password; a locked one is PermissionDenied.
    pub fn unlock(&mut self, url: &str, password: &str) -> io::Result<Doc> {
        let request = Pack::new().str(url).str(password).bytes();
        let payload = self.request(tag::LOAD, &request).map_err(|error| {
            match error.to_string().as_str() {
                crate::ipc::LOCKED => io::Error::new(io::ErrorKind::PermissionDenied, error.to_string()),
                crate::ipc::DAMAGED => io::Error::new(io::ErrorKind::InvalidData, Damaged),
                _ => error,
            }
        })?;
        self.opened(&payload)
    }

    /// A load's reply: the document's number and kind.
    fn opened(&mut self, payload: &[u8]) -> io::Result<Doc> {
        let mut reply = Unpack::new(payload);
        let (id, kind) = (reply.u64()?, reply.u64()?);
        self.feed.opened(id);
        Ok(Doc { worker: self.session, id, rev: 0, kind })
    }
}
