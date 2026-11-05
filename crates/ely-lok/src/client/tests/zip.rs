//! Reads and writes zips in process.

use std::io::Read;

fn u16_at(bytes: &[u8], at: usize) -> usize {
    u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize
}

fn u32_at(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize
}

/// Every entry by the central directory, stored or deflated.
pub(super) fn entries(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let end = (0..bytes.len() - 21)
        .rev()
        .find(|&at| bytes[at..].starts_with(b"PK\x05\x06"))
        .expect("an end of central directory");
    let mut at = u32_at(bytes, end + 16);
    (0..u16_at(bytes, end + 10))
        .map(|_| {
            assert!(bytes[at..].starts_with(b"PK\x01\x02"), "a central record at {at}");
            let (method, packed, size) = (u16_at(bytes, at + 10), u32_at(bytes, at + 20), u32_at(bytes, at + 24));
            let name_len = u16_at(bytes, at + 28);
            let name = String::from_utf8(bytes[at + 46..at + 46 + name_len].to_vec()).expect("a UTF-8 name");
            let local = u32_at(bytes, at + 42);
            assert!(bytes[local..].starts_with(b"PK\x03\x04"), "a local header for {name}");
            let start = local + 30 + u16_at(bytes, local + 26) + u16_at(bytes, local + 28);
            let data = &bytes[start..start + packed];
            let body = match method {
                0 => data.to_vec(),
                8 => {
                    let mut out = Vec::with_capacity(size);
                    flate2::read::DeflateDecoder::new(data).read_to_end(&mut out).expect("inflates");
                    out
                }
                other => panic!("{name}: method {other}"),
            };
            assert_eq!(body.len(), size, "{name}'s size");
            at += 46 + name_len + u16_at(bytes, at + 30) + u16_at(bytes, at + 32);
            (name, body)
        })
        .collect()
}

/// A zip that stores these entries uncompressed.
pub(super) fn stored(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let (mut out, mut central) = (Vec::new(), Vec::new());
    for (name, body) in entries {
        let mut crc = flate2::Crc::new();
        crc.update(body);
        let size = u32::try_from(body.len()).expect("a small entry").to_le_bytes();
        let name_len = u16::try_from(name.len()).expect("a short name").to_le_bytes();
        // Version 2.0, no flags, stored, 1980-01-01.
        let common = [&[20, 0, 0, 0, 0, 0, 0, 0, 33, 0][..], &crc.sum().to_le_bytes(), &size, &size, &name_len, &[0, 0]].concat();
        let offset = u32::try_from(out.len()).expect("a small zip").to_le_bytes();
        out.extend([&b"PK\x03\x04"[..], &common, name.as_bytes(), body].concat());
        central.extend([&b"PK\x01\x02\x14\x00"[..], &common, &[0; 10], &offset, name.as_bytes()].concat());
    }
    let count = u16::try_from(entries.len()).expect("few entries").to_le_bytes();
    let at = u32::try_from(out.len()).expect("a small zip").to_le_bytes();
    let length = u32::try_from(central.len()).expect("a small zip").to_le_bytes();
    out.extend(central);
    out.extend([&b"PK\x05\x06\0\0\0\0"[..], &count, &count, &length, &at, &[0, 0]].concat());
    out
}
