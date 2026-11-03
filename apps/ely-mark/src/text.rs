//! File bytes to editor text and back (§14.3, QA-21).

use std::io;

use ely_ipc::Worded;

const BOM: &[u8] = b"\xEF\xBB\xBF";

/// A file as read: its bytes, BOM and line ending.
#[derive(Clone, Debug)]
pub struct Source {
    bytes: Vec<u8>,
    bom: bool,
    /// More CRLF than lone LF line ends.
    crlf: bool,
    /// The editor's text: no BOM, CRLF as LF.
    pub text: String,
}

impl Source {
    /// Reads UTF-8, with or without BOM; other bytes fail.
    pub fn decode(bytes: Vec<u8>) -> io::Result<Source> {
        if bytes.starts_with(b"\xFF\xFE") || bytes.starts_with(b"\xFE\xFF") {
            return Err(Worded::error(io::ErrorKind::InvalidData, "mark.error.utf16", &[]));
        }
        let bom = bytes.starts_with(BOM);
        let body = &bytes[if bom { BOM.len() } else { 0 }..];
        let text = std::str::from_utf8(body).map_err(|error| {
            let at = error.valid_up_to();
            Worded::error(io::ErrorKind::InvalidData, "mark.error.not_utf8", &[("at", &at.to_string())])
        })?;
        let crlfs = text.matches("\r\n").count();
        let crlf = crlfs > text.matches('\n').count() - crlfs;
        let text = text.replace("\r\n", "\n");
        Ok(Source {
            bytes,
            bom,
            crlf,
            text,
        })
    }

    /// The bytes for `text`; unedited text is the file itself.
    pub fn encode(&self, text: &str) -> Vec<u8> {
        if text == self.text {
            return self.bytes.clone();
        }
        let text = match self.crlf {
            true => text.replace('\n', "\r\n"),
            false => text.to_owned(),
        };
        let mut bytes = Vec::with_capacity(text.len() + BOM.len());
        if self.bom {
            bytes.extend_from_slice(BOM);
        }
        bytes.extend_from_slice(text.as_bytes());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::Source;

    #[test]
    fn unedited_text_writes_the_same_bytes() {
        for bytes in [
            &b"# a\r\n\r\nb\n"[..],
            b"\xEF\xBB\xBF# a\r\n",
            b"a\rb\n",
            b"",
            b"no end",
        ] {
            let source = Source::decode(bytes.to_vec()).expect("reads");
            assert_eq!(source.encode(&source.text.clone()), bytes);
        }
    }

    #[test]
    fn edits_keep_the_bom_and_line_ending() {
        let source = Source::decode(b"\xEF\xBB\xBF# a\r\n\r\nb\r\n".to_vec()).expect("reads");
        assert_eq!(source.text, "# a\n\nb\n");
        assert_eq!(source.encode("# a\n\nc\n"), b"\xEF\xBB\xBF# a\r\n\r\nc\r\n");
        let plain = Source::decode(b"a\nb\n".to_vec()).expect("reads");
        assert_eq!(plain.encode("a\nc\n"), b"a\nc\n");
    }

    #[test]
    fn mixed_line_ends_follow_the_most() {
        let source = Source::decode(b"a\r\nb\r\nc\n".to_vec()).expect("reads");
        assert_eq!(source.encode("a\nb\nd\n"), b"a\r\nb\r\nd\r\n");
    }

    #[test]
    fn other_encodings_fail_with_the_place() {
        let error = Source::decode(b"ok \xFF".to_vec()).expect_err("not UTF-8");
        let said = error.get_ref().and_then(|inner| inner.downcast_ref::<ely_ipc::Worded>()).expect("worded");
        assert_eq!((said.key, said.args.as_slice()), ("mark.error.not_utf8", &[("at".to_string(), "3".to_string())][..]));
        assert!(Source::decode(b"\xFF\xFEa\0".to_vec()).is_err());
    }
}
