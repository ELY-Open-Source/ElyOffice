//! Folder names in modified UTF-7, RFC 3501 §5.1.3.

use crate::{Error, Result};

/// The readable name; only the canonical spelling passes.
pub(crate) fn decode(name: &str) -> Result<String> {
    let bad = |why: &str| Error::Protocol(format!("the folder name {name:?} is not modified UTF-7: {why}"));
    let mut shown = String::with_capacity(name.len());
    let mut rest = name.as_bytes();
    let mut after_run = false;
    while let Some((&byte, tail)) = rest.split_first() {
        rest = tail;
        if byte != b'&' {
            if !(0x20..=0x7e).contains(&byte) {
                return Err(bad("a byte outside printable ASCII"));
            }
            shown.push(char::from(byte));
            after_run = false;
            continue;
        }
        let end = rest
            .iter()
            .position(|it| *it == b'-')
            .ok_or_else(|| bad("a shift without its end"))?;
        let run = &rest[..end];
        rest = &rest[end + 1..];
        if run.is_empty() {
            shown.push('&');
            after_run = false;
            continue;
        }
        if after_run {
            return Err(bad("a shift right after another"));
        }
        for char in char::decode_utf16(units(run).map_err(bad)?) {
            let char = char.map_err(|_| bad("a broken surrogate pair"))?;
            if (' '..='~').contains(&char) {
                return Err(bad("printable ASCII in BASE64"));
            }
            shown.push(char);
        }
        after_run = true;
    }
    Ok(shown)
}

/// UTF-16 units from modified BASE64; spare bits are zero.
fn units(run: &[u8]) -> std::result::Result<Vec<u16>, &'static str> {
    let mut units = Vec::with_capacity(run.len() * 3 / 8);
    let (mut held, mut bits) = (0u32, 0u32);
    for &byte in run {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b',' => 63,
            _ => return Err("a byte outside modified BASE64"),
        };
        held = held << 6 | u32::from(value);
        bits += 6;
        if bits >= 16 {
            bits -= 16;
            units.push((held >> bits) as u16);
            held &= (1 << bits) - 1;
        }
    }
    if bits >= 6 || held != 0 {
        return Err("BASE64 that does not end on a whole unit");
    }
    Ok(units)
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn canonical_names_read_as_written() {
        assert_eq!(decode("&g0l6P3ux-").unwrap(), "草稿箱");
        assert_eq!(decode("&UXZO1mWHTvZZOQ-").unwrap(), "其他文件夹");
        assert_eq!(decode("~peter/mail/&U,BTFw-/&ZeVnLIqe-").unwrap(), "~peter/mail/台北/日本語");
        assert_eq!(decode("A&-B &- C").unwrap(), "A&B & C");
        assert_eq!(decode("&2D3eAA-").unwrap(), "😀");
        assert_eq!(decode("INBOX").unwrap(), "INBOX");
    }

    #[test]
    fn other_spellings_are_refused() {
        for name in [
            "&ZeU",
            "&ZeU-&ZeU-",
            "&AGE-",
            "&ZeV-",
            "&ZeUA-",
            "&2D0-",
            "&Ze!-",
            "caf\u{e9}",
            "a\tb",
        ] {
            assert!(decode(name).is_err(), "{name:?} passed");
        }
    }
}
