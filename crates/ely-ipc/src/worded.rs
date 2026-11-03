//! Errors Ely words: a catalog key the window translates.

use std::io;

/// Marks a worded error in a worker's reply.
const WIRE: &str = "worded:";
/// Parts of the wire form: key, then names and values.
const UNIT: char = '\u{1f}';

/// An error Ely words: a catalog key and its arguments.
#[derive(Debug)]
pub struct Worded {
    pub key: &'static str,
    pub args: Vec<(String, String)>,
}

impl Worded {
    /// An io error of `kind` the window words by `key`.
    pub fn error(kind: io::ErrorKind, key: &'static str, args: &[(&str, &str)]) -> io::Error {
        let args = args.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect();
        io::Error::new(kind, Self { key, args })
    }

    /// `error`, then removing its partial files failed with `left`.
    pub fn then(error: io::Error, left: io::Error) -> io::Error {
        io::Error::other(Then { error, left })
    }

    /// `error` as reply text; a worded one keeps its key..
    pub fn wire(error: &io::Error) -> String {
        let Some(worded) = error.get_ref().and_then(|inner| inner.downcast_ref::<Self>()) else {
            return error.to_string();
        };
        let mut text = format!("{WIRE}{}", worded.key);
        for (name, value) in &worded.args {
            text.extend([UNIT.to_string(), name.clone(), UNIT.to_string(), value.clone()]);
        }
        text
    }

    /// A reply `wire` wrote with one of `keys`.
    pub fn read(text: &str, keys: &[&'static str]) -> Option<io::Error> {
        let mut parts = text.strip_prefix(WIRE)?.split(UNIT);
        let named = parts.next()?;
        let key = *keys.iter().find(|key| **key == named)?;
        let rest: Vec<&str> = parts.collect();
        if !rest.len().is_multiple_of(2) {
            return None;
        }
        let args = rest.chunks(2).map(|pair| (pair[0].to_string(), pair[1].to_string())).collect();
        Some(io::Error::other(Self { key, args }))
    }
}

/// Logs name the key, never the arguments.
impl std::fmt::Display for Worded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.key)
    }
}

impl std::error::Error for Worded {}

/// A failure, then a failed cleanup; each worded alone.
#[derive(Debug)]
pub struct Then {
    pub error: io::Error,
    pub left: io::Error,
}

/// Logs name the key, never the inner texts.
impl std::fmt::Display for Then {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("error.then")
    }
}

impl std::error::Error for Then {}

#[cfg(test)]
mod tests {
    use super::Worded;
    use std::io;

    #[test]
    fn a_worded_reply_reads_back_and_others_stay_text() {
        let sent = Worded::error(io::ErrorKind::InvalidData, "pdf.error.kind", &[("name", "a: b")]);
        let wire = Worded::wire(&sent);
        let back = Worded::read(&wire, &["pdf.error.kind"]).expect("a known key");
        let worded = back.get_ref().and_then(|inner| inner.downcast_ref::<Worded>()).expect("worded");
        assert_eq!((worded.key, worded.args.as_slice()), ("pdf.error.kind", &[("name".to_string(), "a: b".to_string())][..]));
        assert!(Worded::read("worded:pdf.error.kind\u{1f}name", &["pdf.error.kind"]).is_none(), "a cut pair stays text");
        assert!(Worded::read(&wire, &["pdf.error.other"]).is_none(), "an unknown key stays text");
        assert_eq!(Worded::wire(&io::Error::other("plain")), "plain");
        assert!(Worded::read("plain", &["pdf.error.kind"]).is_none());
        assert_eq!(sent.to_string(), "pdf.error.kind", "logs name the key");
    }
}
