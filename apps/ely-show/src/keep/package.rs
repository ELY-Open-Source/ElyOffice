//! The zip comment travels with the file, outside its parts.

use std::{
    fs::File,
    io::{self, Read as _, Seek as _, SeekFrom},
    path::Path,
};

use super::{Held, parsed, written};

/// Starts a comment ElyShow wrote.
const OURS: &[u8] = b"ElyShow\n";
/// The zip's end record, before its comment.
const END: &[u8] = b"PK\x05\x06";
const END_LENGTH: usize = 22;

/// Where the end record starts in a package's tail.
fn end(tail: &[u8]) -> Option<usize> {
    let last = tail.len().checked_sub(END_LENGTH)?;
    (0..=last).rev().find(|at| {
        let record = &tail[*at..];
        let comment = usize::from(u16::from_le_bytes([record[20], record[21]]));
        record.starts_with(END) && comment == record.len() - END_LENGTH
    })
}

/// A package's bytes with `held` as its comment.
pub fn seal(mut bytes: Vec<u8>, held: &[Held]) -> io::Result<Vec<u8>> {
    let refused = |why: &str| io::Error::new(io::ErrorKind::InvalidData, why.to_string());
    let from = bytes.len().saturating_sub(END_LENGTH + usize::from(u16::MAX));
    // An encrypted deck is no zip: say so.
    let at = from + end(&bytes[from..]).ok_or_else(|| refused("this file cannot keep countdowns or scoreboards"))?;
    let old = &bytes[at + END_LENGTH..];
    if !old.is_empty() && !old.starts_with(OURS) {
        return Err(refused("the package already has a comment"));
    }
    let comment = [OURS, written(held).to_string().as_bytes()].concat();
    let length = u16::try_from(comment.len()).map_err(|_| refused("too many countdowns and scoreboards to keep"))?;
    bytes.truncate(at + END_LENGTH - 2);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(&comment);
    Ok(bytes)
}

/// What the file at `path` keeps, if anything.
pub fn read(path: &Path) -> io::Result<Option<Vec<Held>>> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    let most = (END_LENGTH + usize::from(u16::MAX)) as u64;
    file.seek(SeekFrom::Start(length.saturating_sub(most)))?;
    let mut tail = Vec::new();
    file.read_to_end(&mut tail)?;
    kept(&tail)
}

fn kept(tail: &[u8]) -> io::Result<Option<Vec<Held>>> {
    let Some(json) = end(tail).and_then(|at| tail[at + END_LENGTH..].strip_prefix(OURS)) else {
        return Ok(None);
    };
    parsed(&serde_json::from_slice(json).map_err(io::Error::other)?).map(Some)
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

    #[test]
    fn a_package_keeps_its_bytes_and_gains_a_comment() {
        let held = vec![Held { at: 2, score: None, timer: None }];
        let sealed = seal(empty(), &held).expect("sealed");
        assert_eq!(&sealed[..END_LENGTH - 2], &empty()[..END_LENGTH - 2]);
        assert_eq!(kept(&sealed).expect("reads"), Some(held.clone()));
        let again = seal(sealed.clone(), &held).expect("ours is replaced");
        assert_eq!(again, sealed);
        assert_eq!(kept(&empty()).expect("reads"), None);
        assert!(seal(b"not a zip".to_vec(), &held).is_err());
        let mut theirs = empty();
        theirs[20] = 2;
        theirs.extend_from_slice(b"hi");
        assert!(seal(theirs, &held).is_err(), "another's comment stays");
    }
}
