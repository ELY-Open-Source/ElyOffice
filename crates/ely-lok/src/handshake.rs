//! Worker handshake: pinned-build and tile checks (LOK-01).

use std::{io, path::Path};

use crate::client::{BUILD_ID, Worker};
use crate::ipc::{self, Pack, Unpack, tag};

/// Red text: a BGRA tile shows red in byte two.
const VERIFY_DOC: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:style=\"urn:oasis:names:tc:opendocument:xmlns:style:1.0\" xmlns:fo=\"urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0\" xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" office:version=\"1.3\" office:mimetype=\"application/vnd.oasis.opendocument.text\"><office:automatic-styles><style:style style:name=\"T1\" style:family=\"text\"><style:text-properties fo:color=\"#ff0000\" fo:font-size=\"28pt\" fo:font-weight=\"bold\"/></style:style></office:automatic-styles><office:body><office:text><text:p><text:span text:style-name=\"T1\">verify</text:span></text:p></office:text></office:body></office:document>";

impl Worker {
    /// Rejects builds that are not the pinned one (LOK-01).
    pub(crate) fn verify(&mut self, build: &str, profile_dir: &Path) -> io::Result<()> {
        if !build.contains(BUILD_ID) {
            return self.reject(format!("not the pinned build {BUILD_ID}: {build}"));
        }
        if !self.tile_self_check(profile_dir)? {
            return self.reject(format!(
                "the pinned build {BUILD_ID} paints no BGRA red (unpatched, or another order)"
            ));
        }
        Ok(())
    }

    /// Kills the child and reports the rejection.
    pub(crate) fn reject<T>(&mut self, reason: String) -> io::Result<T> {
        log::warn!("lok: {reason}");
        *self.stopper.gate() = true;
        let _ = self.child.kill();
        let _ = self.child.wait();
        Err(io::Error::new(io::ErrorKind::InvalidData, reason))
    }

    /// The version fields of the hooked library, checked strictly (LOK-01).
    pub(crate) fn version_checked(&mut self) -> io::Result<String> {
        let build = self.build_id()?;
        let fields = [
            ("ProductName", "LibreOffice"),
            ("ProductVersion", "26.2"),
            ("BuildId", BUILD_ID),
        ];
        for (key, want) in fields {
            let Some(have) = version_field(&build, key) else {
                return self.reject(format!("version misses {key}: {build}"));
            };
            if have != want {
                return self.reject(format!("version {key} is {have}, not {want}"));
            }
        }
        Ok(build)
    }

    /// Paints the red text; whether BGRA red came out.
    pub(crate) fn tile_self_check(&mut self, profile_dir: &Path) -> io::Result<bool> {
        let doc = profile_dir.join("lok-verify.fodt");
        std::fs::write(&doc, VERIFY_DOC)?;
        let url = ipc::file_url_of(&dunce::canonicalize(&doc)?);
        let payload = self.request(tag::VERIFY, &Pack::new().str(&url).bytes())?;
        let red = Unpack::new(&payload).u64()?;
        Ok(red > 0)
    }
}

/// One top-level field of a strict JSON object parse.
fn version_field(text: &str, key: &str) -> Option<String> {
    let pairs = JsonFields { text, at: 0 };
    pairs.field(key)
}

/// A strict single-object JSON field walker.
struct JsonFields<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> JsonFields<'a> {
    fn field(mut self, want: &str) -> Option<String> {
        self.take('{')?;
        let mut found = None;
        loop {
            let key = self.string()?;
            self.take(':')?;
            let value = self.value()?;
            if key == want {
                found = value;
            }
            if self.take(',').is_some() {
                continue;
            }
            self.take('}')?;
            self.ws();
            return (self.at == self.text.len()).then_some(found).flatten();
        }
    }

    fn ws(&mut self) {
        self.at += self.text[self.at..]
            .find(|ch: char| !matches!(ch, ' ' | '\t' | '\n' | '\r'))
            .unwrap_or(self.text.len() - self.at);
    }

    fn take(&mut self, ch: char) -> Option<()> {
        self.ws();
        self.text[self.at..]
            .starts_with(ch)
            .then(|| self.at += ch.len_utf8())
    }

    fn string(&mut self) -> Option<String> {
        self.take('"')?;
        let mut out = String::new();
        let mut rest = &self.text[self.at..];
        loop {
            let at = rest.find(|ch: char| ch == '"' || ch == '\\' || ch < '\u{20}')?;
            if rest.as_bytes()[at] < 0x20 {
                return None;
            }
            out.push_str(&rest[..at]);
            if rest[at..].starts_with('"') {
                self.at += at + 1;
                return Some(out);
            }
            let escaped = rest[at + 1..].chars().next()?;
            match escaped {
                '"' | '\\' | '/' => out.push(escaped),
                'b' => out.push('\u{8}'),
                'f' => out.push('\u{c}'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'u' => {
                    let hex = rest[at + 2..].get(..4)?;
                    if !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
                        return None;
                    }
                    let code = u32::from_str_radix(hex, 16).ok()?;
                    out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                    rest = &rest[at + 6..];
                    self.at += at + 6;
                    continue;
                }
                _ => return None,
            }
            rest = &rest[at + 2..];
            self.at += at + 2;
        }
    }

    fn value(&mut self) -> Option<Option<String>> {
        self.ws();
        let rest = &self.text[self.at..];
        if rest.starts_with('"') {
            return self.string().map(Some);
        }
        for word in ["true", "false", "null"] {
            if rest.starts_with(word) {
                self.at += word.len();
                return Some(None);
            }
        }
        let end = rest
            .find(|ch: char| !(ch.is_ascii_digit() || matches!(ch, '-' | '+' | '.' | 'e' | 'E')))
            .unwrap_or(rest.len());
        let text = &rest[..end];
        if !Self::number_ok(text) {
            return None;
        }
        self.at += end;
        Some(None)
    }

    fn number_ok(text: &str) -> bool {
        let text = text.strip_prefix('-').unwrap_or(text);
        let (mantissa, exponent) = match text.split_once(['e', 'E']) {
            Some((mantissa, exponent)) => (mantissa, Some(exponent)),
            None => (text, None),
        };
        if let Some(exponent) = exponent {
            let exp = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
            if exp.is_empty() || !exp.chars().all(|ch| ch.is_ascii_digit()) {
                return false;
            }
        }
        let (int, dot, frac) = match mantissa.split_once('.') {
            Some((int, frac)) => (int, true, frac),
            None => (mantissa, false, ""),
        };
        if int.is_empty() || !int.chars().all(|ch| ch.is_ascii_digit()) {
            return false;
        }
        if int.len() > 1 && int.starts_with('0') {
            return false;
        }
        !dot || (!frac.is_empty() && frac.chars().all(|ch| ch.is_ascii_digit()))
    }
}
