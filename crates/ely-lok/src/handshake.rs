//! Worker handshake: pinned-build and tile checks (LOK-01).

use std::{
    io,
    path::{Path, PathBuf},
};

use crate::client::{BUILD_ID, Worker};
use crate::ipc::{self, Pack, Unpack, tag};

/// The start-up check: pending, running, passed, or why it failed.
pub(crate) enum Check {
    Pending(PathBuf),
    Checking,
    Passed,
    Failed(String),
}

/// Where the kernel reads its buildid, from the install.
#[cfg(target_os = "macos")]
const VERSION_FILE: &str = "../Resources/versionrc";
#[cfg(windows)]
const VERSION_FILE: &str = "version.ini";
#[cfg(all(unix, not(target_os = "macos")))]
const VERSION_FILE: &str = "versionrc";

/// Which module the start check loads: the app's own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Probe {
    #[default]
    Text,
    Sheet,
    Slides,
}

const HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:style=\"urn:oasis:names:tc:opendocument:xmlns:style:1.0\" xmlns:fo=\"urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0\" xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\" xmlns:draw=\"urn:oasis:names:tc:opendocument:xmlns:drawing:1.0\" xmlns:svg=\"urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0\" office:version=\"1.3\" office:mimetype=\"application/vnd.oasis.opendocument.";

impl Probe {
    /// The sample's file name and text: red, top left.
    fn sample(self) -> (&'static str, String) {
        // A BGRA tile shows red in byte two.
        let (name, kind, body) = match self {
            Probe::Text => (
                "lok-verify.fodt",
                "text",
                "<office:automatic-styles><style:style style:name=\"T1\" style:family=\"text\"><style:text-properties fo:color=\"#ff0000\" fo:font-size=\"28pt\" fo:font-weight=\"bold\"/></style:style></office:automatic-styles><office:body><office:text><text:p><text:span text:style-name=\"T1\">verify</text:span></text:p></office:text></office:body>",
            ),
            Probe::Sheet => (
                "lok-verify.fods",
                "spreadsheet",
                "<office:automatic-styles><style:style style:name=\"ce1\" style:family=\"table-cell\"><style:table-cell-properties fo:background-color=\"#ff0000\"/></style:style></office:automatic-styles><office:body><office:spreadsheet><table:table table:name=\"S\"><table:table-row><table:table-cell table:style-name=\"ce1\"/></table:table-row></table:table></office:spreadsheet></office:body>",
            ),
            Probe::Slides => (
                "lok-verify.fodp",
                "presentation",
                "<office:automatic-styles><style:style style:name=\"gr1\" style:family=\"graphic\"><style:graphic-properties draw:fill=\"solid\" draw:fill-color=\"#ff0000\" draw:stroke=\"none\"/></style:style></office:automatic-styles><office:body><office:presentation><draw:page draw:name=\"p\"><draw:rect draw:style-name=\"gr1\" svg:x=\"0cm\" svg:y=\"0cm\" svg:width=\"4cm\" svg:height=\"4cm\"/></draw:page></office:presentation></office:body>",
            ),
        };
        (name, format!("{HEAD}{kind}\">{body}</office:document>"))
    }
}

impl Worker {
    /// Checks the pinned build and tiles, once (LOK-01).
    pub fn ready(&mut self) -> io::Result<()> {
        let profile = match &self.check {
            // The checks' own requests pass through here.
            Check::Passed | Check::Checking => return Ok(()),
            Check::Failed(why) => return Err(io::Error::other(format!("the worker failed its check: {why}"))),
            Check::Pending(profile) => profile.clone(),
        };
        // Its starter checked it on this kernel id.
        if self.child.adopted() {
            self.check = Check::Passed;
            log::info!("lok: the engine taken over was checked at its start");
            return Ok(());
        }
        self.check = Check::Checking;
        let checked = self.version_checked().and_then(|build| self.verify(&build, &profile).map(|()| build));
        match checked {
            Ok(build) => {
                self.check = Check::Passed;
                log::info!("lok: worker up, {build}");
                Ok(())
            }
            Err(error) => {
                let error = self.start_failed(error);
                self.check = Check::Failed(error.to_string());
                Err(error)
            }
        }
    }

    /// Stops a worker that failed its check; says why.
    fn start_failed(&mut self, error: io::Error) -> io::Error {
        *self.stopper.gate() = true;
        self.child.end();
        // No document but the check's own is open yet.
        let lines = Vec::from(self.stderr.said().lines).join(" | ");
        let (install, profile, _) = &self.start;
        let message = format!(
            "{error}; install {}; profile {}; {}; the worker said: {lines}",
            install.display(),
            profile.display(),
            version_read(install),
        );
        log::error!("lok: the worker did not start: {message}");
        io::Error::new(error.kind(), message)
    }

    /// Rejects builds that are not the pinned one (LOK-01).
    pub(crate) fn verify(&mut self, build: &str, profile_dir: &Path) -> io::Result<()> {
        if !build.contains(BUILD_ID) {
            return reject(format!("not the pinned build {BUILD_ID}: {build}"));
        }
        if !self.tile_self_check(profile_dir)? {
            return reject(format!(
                "the pinned build {BUILD_ID} paints no BGRA red (unpatched, or another order)"
            ));
        }
        Ok(())
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
                return reject(format!("version misses {key}: {build}"));
            };
            if have != want {
                return reject(format!("version {key} is {have}, not {want}"));
            }
        }
        Ok(build)
    }

    /// Paints the red sample; whether BGRA red came out.
    pub(crate) fn tile_self_check(&mut self, profile_dir: &Path) -> io::Result<bool> {
        let (name, sample) = self.probe.sample();
        let doc = profile_dir.join(name);
        std::fs::write(&doc, sample)?;
        let url = ipc::file_url_of(&dunce::canonicalize(&doc)?);
        let payload = self.request(tag::VERIFY, &Pack::new().str(&url).bytes())?;
        let red = Unpack::new(&payload).u64()?;
        Ok(red > 0)
    }
}

/// A rejected build; the check then stops the worker.
fn reject<T>(reason: String) -> io::Result<T> {
    Err(io::Error::new(io::ErrorKind::InvalidData, reason))
}

/// The kernel's version file, and the buildid it gives now.
fn version_read(install: &Path) -> String {
    let file = install.join(VERSION_FILE);
    let now = match std::fs::read_to_string(&file) {
        Ok(text) => match text.lines().find_map(|line| line.strip_prefix("buildid=")) {
            Some(id) => format!("buildid {id}"),
            None => "no buildid".to_string(),
        },
        Err(error) => error.to_string(),
    };
    format!("{} now gives {now}", file.display())
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

#[cfg(test)]
mod tests {
    use super::{VERSION_FILE, version_read};

    #[test]
    fn a_failed_start_names_the_version_file_and_what_it_gives() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let install = dir.path().join("program");
        let file = install.join(VERSION_FILE);
        let gone = version_read(&install);
        assert!(gone.starts_with(&format!("{} now gives ", file.display())), "{gone}");
        assert!(!gone.contains("buildid"), "{gone}");
        std::fs::create_dir_all(file.parent().expect("a folder")).expect("the folder is made");
        std::fs::write(&file, "[Version]\r\nAllLanguages=en-US\r\nbuildid=abc123\r\n").expect("written");
        assert_eq!(version_read(&install), format!("{} now gives buildid abc123", file.display()));
        std::fs::write(&file, "[Version]\r\n").expect("written");
        assert!(version_read(&install).ends_with("now gives no buildid"));
    }
}
