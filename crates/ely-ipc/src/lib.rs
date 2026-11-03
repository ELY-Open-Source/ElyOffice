//! Framed requests between an app and its engine worker.

mod relay;
mod worded;
pub use relay::{Relay, Said, relay};
pub use worded::{Then, Worded};

use std::io::{self, Read, Write};
use std::process::Command;

/// A child that opens no console window on Windows.
pub fn windowless(command: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        // CREATE_NO_WINDOW
        command.creation_flags(0x0800_0000);
    }
    command
}

/// A frame: u32 length, u8 tag, u64 request id, payload.
pub const MAX_FRAME: usize = 64 * 1024 * 1024;

/// Frames no request asked for, such as pushed events.
pub const UNASKED: u64 = 0;

/// Writes one frame and flushes; replies carry the request's id.
pub fn write_frame(out: &mut impl Write, tag: u8, id: u64, payload: &[u8]) -> io::Result<()> {
    // The reader's cap, checked before a byte goes out.
    if payload.len() > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("a {} byte frame is over the {MAX_FRAME} byte cap", payload.len()),
        ));
    }
    let len = payload.len() as u32;
    out.write_all(&len.to_le_bytes())?;
    out.write_all(&[tag])?;
    out.write_all(&id.to_le_bytes())?;
    out.write_all(payload)?;
    out.flush()
}

/// A reply with another request's id means the pairing broke.
pub fn check_reply(asked: u64, got: u64) -> io::Result<()> {
    match asked == got {
        true => Ok(()),
        false => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("the worker answered request {got}, not {asked}"),
        )),
    }
}

/// Reads (tag, id, payload); None at a clean stream end.
pub fn read_frame(input: &mut impl Read) -> io::Result<Option<(u8, u64, Vec<u8>)>> {
    read_capped(input, MAX_FRAME)
}

/// As `read_frame`, refusing payloads over `cap` before allocating.
pub fn read_capped(input: &mut impl Read, cap: usize) -> io::Result<Option<(u8, u64, Vec<u8>)>> {
    let mut len = [0u8; 4];
    // Only a stream ending between frames ends cleanly.
    let mut got = 0;
    while got < len.len() {
        match input.read(&mut len[got..]) {
            Ok(0) if got == 0 => return Ok(None),
            Ok(0) => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "a cut frame header")),
            Ok(read) => got += read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    let len = u32::from_le_bytes(len) as usize;
    if len > cap {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("a {len} byte frame is over the {cap} byte cap"),
        ));
    }
    let mut tag = [0u8; 1];
    input.read_exact(&mut tag)?;
    let mut id = [0u8; 8];
    input.read_exact(&mut id)?;
    let mut payload = vec![0u8; len];
    input.read_exact(&mut payload)?;
    Ok(Some((tag[0], u64::from_le_bytes(id), payload)))
}

/// A payload writer.
#[derive(Default)]
pub struct Pack {
    bytes: Vec<u8>,
}

impl Pack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn str(self, text: &str) -> Self {
        self.blob(text.as_bytes())
    }

    /// Length-prefixed raw bytes.
    pub fn blob(mut self, data: &[u8]) -> Self {
        self.bytes
            .extend_from_slice(&(data.len() as u32).to_le_bytes());
        self.bytes.extend_from_slice(data);
        self
    }

    pub fn u64(mut self, value: u64) -> Self {
        self.bytes.extend_from_slice(&value.to_le_bytes());
        self
    }

    pub fn i64(mut self, value: i64) -> Self {
        self.bytes.extend_from_slice(&value.to_le_bytes());
        self
    }

    pub fn bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// A payload reader.
pub struct Unpack<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Unpack<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn take(&mut self, len: usize) -> io::Result<&'a [u8]> {
        let end = self.at + len;
        let Some(slice) = self.bytes.get(self.at..end) else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "a short frame"));
        };
        self.at = end;
        Ok(slice)
    }

    pub fn str(&mut self) -> io::Result<String> {
        String::from_utf8(self.blob()?)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "payload is no text"))
    }

    /// Length-prefixed raw bytes.
    pub fn blob(&mut self) -> io::Result<Vec<u8>> {
        let len = u32::from_le_bytes(self.take(4)?.try_into().expect("a u32")) as usize;
        Ok(self.take(len)?.to_vec())
    }

    pub fn u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().expect("a u64")))
    }

    pub fn i64(&mut self) -> io::Result<i64> {
        Ok(i64::from_le_bytes(
            self.take(8)?.try_into().expect("an i64"),
        ))
    }

    /// The unread tail.
    pub fn rest(&self) -> &'a [u8] {
        &self.bytes[self.at..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_and_payloads_round_trip_and_short_ones_fail() {
        let payload = Pack::new().str("页 one").u64(7).i64(-3).blob(&[1, 2]).bytes();
        let mut wire = Vec::new();
        write_frame(&mut wire, 9, 41, &payload).expect("writes");
        let mut read = &wire[..];
        let (tag, id, back) = read_frame(&mut read).expect("reads").expect("a frame");
        assert_eq!((tag, id), (9, 41));
        assert!(check_reply(41, id).is_ok());
        assert_eq!(check_reply(42, id).expect_err("another request").kind(), io::ErrorKind::InvalidData);
        let mut unpack = Unpack::new(&back);
        assert_eq!(unpack.str().expect("text"), "页 one");
        assert_eq!((unpack.u64().expect("u64"), unpack.i64().expect("i64")), (7, -3));
        assert_eq!(unpack.blob().expect("blob"), [1, 2]);
        assert!(unpack.rest().is_empty());
        assert!(unpack.u64().is_err(), "a short payload is an error");
        assert!(read_frame(&mut &[][..]).expect("a clean end").is_none());
        let mut nothing = Vec::new();
        let over = write_frame(&mut nothing, 9, 1, &vec![0; MAX_FRAME + 1]).expect_err("over the cap");
        assert_eq!(over.kind(), io::ErrorKind::InvalidInput);
        assert!(nothing.is_empty(), "nothing written");
        for cut in 1..4 {
            let failed = read_frame(&mut &wire[..cut]).expect_err("a cut header");
            assert_eq!(failed.kind(), io::ErrorKind::UnexpectedEof, "{cut} bytes");
        }
    }
}
