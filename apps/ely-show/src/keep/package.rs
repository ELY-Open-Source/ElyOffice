//! The zip comment travels with the file, outside its parts.

use std::{
    fs::File,
    io::{self, Read as _, Seek as _, SeekFrom},
    path::Path,
};

use ely_ipc::Worded;

use super::{Held, parsed, written};

/// Starts a comment ElyShow wrote.
const OURS: &[u8] = b"ElyShow\n";
/// The zip's end record, before its comment.
const END: &[u8] = b"PK\x05\x06";
const END_LENGTH: usize = 22;

/// What a file's zip comment holds.
#[derive(Debug, PartialEq)]
pub enum Found {
    Nothing,
    Ours(Vec<Held>),
    /// Another program's comment, kept as it was.
    Theirs(Vec<u8>),
}

/// Where the end record starts in a package's tail.
fn end(tail: &[u8]) -> Option<usize> {
    let last = tail.len().checked_sub(END_LENGTH)?;
    (0..=last).rev().find(|at| {
        let record = &tail[*at..];
        let comment = usize::from(u16::from_le_bytes([record[20], record[21]]));
        record.starts_with(END) && comment == record.len() - END_LENGTH
    })
}

fn refused(key: &'static str) -> io::Error {
    Worded::error(io::ErrorKind::InvalidData, key, &[])
}

/// A package's bytes with `comment` as its comment.
pub fn attach(mut bytes: Vec<u8>, comment: &[u8]) -> io::Result<Vec<u8>> {
    let from = bytes.len().saturating_sub(END_LENGTH + usize::from(u16::MAX));
    // An encrypted deck is no zip: say so.
    let at = from + end(&bytes[from..]).ok_or_else(|| refused("show.keep.unzipped"))?;
    let old = &bytes[at + END_LENGTH..];
    if !old.is_empty() && !old.starts_with(OURS) {
        return Err(refused("show.keep.theirs"));
    }
    let length = u16::try_from(comment.len()).map_err(|_| refused("show.keep.full"))?;
    bytes.truncate(at + END_LENGTH - 2);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(comment);
    Ok(bytes)
}

/// A package's bytes with `held` as its comment.
pub fn seal(bytes: Vec<u8>, held: &[Held]) -> io::Result<Vec<u8>> {
    attach(bytes, &[OURS, written(held).to_string().as_bytes()].concat())
}

/// What the file at `path` keeps in its comment.
pub fn read(path: &Path) -> io::Result<Found> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    let most = (END_LENGTH + usize::from(u16::MAX)) as u64;
    file.seek(SeekFrom::Start(length.saturating_sub(most)))?;
    let mut tail = Vec::new();
    file.read_to_end(&mut tail)?;
    kept(&tail)
}

fn kept(tail: &[u8]) -> io::Result<Found> {
    let Some(comment) = end(tail).map(|at| &tail[at + END_LENGTH..]).filter(|comment| !comment.is_empty()) else {
        return Ok(Found::Nothing);
    };
    let Some(json) = comment.strip_prefix(OURS) else {
        return Ok(Found::Theirs(comment.to_vec()));
    };
    parsed(&serde_json::from_slice(json).map_err(io::Error::other)?).map(Found::Ours)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A zip with no entries.
    fn empty() -> Vec<u8> {
        let mut bytes = END.to_vec();
        bytes.resize(END_LENGTH, 0);
        bytes
    }

    fn key(error: &io::Error) -> Option<&'static str> {
        error.get_ref()?.downcast_ref::<Worded>().map(|worded| worded.key)
    }

    #[test]
    fn a_package_keeps_its_bytes_and_gains_a_comment() {
        let held = vec![Held { at: 2, score: None, timer: None }];
        let sealed = seal(empty(), &held).expect("sealed");
        assert_eq!(&sealed[..END_LENGTH - 2], &empty()[..END_LENGTH - 2]);
        assert_eq!(kept(&sealed).expect("reads"), Found::Ours(held.clone()));
        let again = seal(sealed.clone(), &held).expect("ours is replaced");
        assert_eq!(again, sealed);
        assert_eq!(kept(&empty()).expect("reads"), Found::Nothing);
        let not_zip = seal(b"not a zip".to_vec(), &held).expect_err("no end record");
        assert_eq!(key(&not_zip), Some("show.keep.unzipped"));
        let theirs = attach(empty(), b"hi").expect("any comment attaches");
        assert_eq!(kept(&theirs).expect("reads"), Found::Theirs(b"hi".to_vec()));
        let kept_theirs = seal(theirs.clone(), &held).expect_err("another's comment stays");
        assert_eq!(key(&kept_theirs), Some("show.keep.theirs"));
        let full = attach(empty(), &vec![b'x'; usize::from(u16::MAX) + 1]).expect_err("too long");
        assert_eq!(key(&full), Some("show.keep.full"));
    }
}
