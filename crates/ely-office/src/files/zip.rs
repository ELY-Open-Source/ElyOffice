//! A zip's central directory and one entry's bytes.

use std::io::{self, Read};

fn bad() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "a damaged zip directory")
}

fn u16_at(bytes: &[u8], at: usize) -> io::Result<usize> {
    bytes.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize).ok_or_else(bad)
}

fn u32_at(bytes: &[u8], at: usize) -> io::Result<usize> {
    bytes.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize).ok_or_else(bad)
}

/// One central directory record.
struct Entry {
    name: String,
    method: usize,
    size: usize,
    local: usize,
}

fn entries(bytes: &[u8]) -> io::Result<Vec<Entry>> {
    let end = (0..bytes.len().saturating_sub(21))
        .rev()
        .find(|&at| bytes[at..].starts_with(b"PK\x05\x06"))
        .ok_or_else(bad)?;
    let (count, mut at) = (u16_at(bytes, end + 10)?, u32_at(bytes, end + 16)?);
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        if !bytes.get(at..).is_some_and(|rest| rest.starts_with(b"PK\x01\x02")) {
            return Err(bad());
        }
        let name_len = u16_at(bytes, at + 28)?;
        let skip = u16_at(bytes, at + 30)? + u16_at(bytes, at + 32)?;
        let name = bytes.get(at + 46..at + 46 + name_len).ok_or_else(bad)?;
        entries.push(Entry {
            name: String::from_utf8_lossy(name).into_owned(),
            method: u16_at(bytes, at + 10)?,
            size: u32_at(bytes, at + 20)?,
            local: u32_at(bytes, at + 42)?,
        });
        at += 46 + name_len + skip;
    }
    Ok(entries)
}

/// The entry names in a zip's central directory.
pub(super) fn names(bytes: &[u8]) -> io::Result<Vec<String>> {
    Ok(entries(bytes)?.into_iter().map(|entry| entry.name).collect())
}

/// Entry `name`'s bytes, stored or deflated; none if absent.
pub(super) fn read(bytes: &[u8], name: &str) -> io::Result<Option<Vec<u8>>> {
    let Some(entry) = entries(bytes)?.into_iter().find(|entry| entry.name == name) else {
        return Ok(None);
    };
    let header = bytes.get(entry.local..).filter(|rest| rest.starts_with(b"PK\x03\x04")).ok_or_else(bad)?;
    let start = 30 + u16_at(header, 26)? + u16_at(header, 28)?;
    let data = header.get(start..start + entry.size).ok_or_else(bad)?;
    match entry.method {
        0 => Ok(Some(data.to_vec())),
        8 => {
            let mut out = Vec::new();
            flate2::read::DeflateDecoder::new(data).read_to_end(&mut out)?;
            Ok(Some(out))
        }
        method => Err(io::Error::new(io::ErrorKind::InvalidData, format!("{name} uses zip method {method}"))),
    }
}
