//! The diagnostics a user may export: shown first, redactable (§18.2).

use std::{
    io::{self, Read as _, Seek as _, SeekFrom},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use crate::AppId;

/// Only the newest part of a long log goes out.
const LOG_TAIL: u64 = 4 * 1024 * 1024;
const CRASH_AGE: Duration = Duration::from_secs(7 * 24 * 3600);

/// One file the bundle would carry.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub path: PathBuf,
    pub bytes: u64,
}

/// `app`'s log and its recent crash reports.
pub fn gather(app: AppId) -> io::Result<Vec<Part>> {
    let mut parts = Vec::new();
    let log = crate::data_dir(app).map_err(io::Error::other)?.join("ely.log");
    parts.extend(log_part(log)?);
    for dir in crash_dirs() {
        parts.extend(crashes_in(&dir, &crashing(app), SystemTime::now())?);
    }
    Ok(parts)
}

/// The log's newest part; only a missing log is nothing.
fn log_part(log: PathBuf) -> io::Result<Option<Part>> {
    match std::fs::metadata(&log) {
        Ok(meta) => Ok(Some(Part { bytes: meta.len().min(LOG_TAIL), path: log })),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io::Error::new(error.kind(), format!("{}: {error}", log.display()))),
    }
}

/// The processes whose crashes belong to `app`.
fn crashing(app: AppId) -> Vec<String> {
    let mut names = vec![app.slug().to_string()];
    match app {
        AppId::Doc | AppId::Table | AppId::Show => names.push("ely-lok-worker".into()),
        AppId::Pdf => names.push("ely-pdf-worker".into()),
        _ => {}
    }
    names
}

/// Where the system keeps crash reports.
fn crash_dirs() -> Vec<PathBuf> {
    let home = directories::BaseDirs::new().map(|base| base.home_dir().to_path_buf());
    match (cfg!(target_os = "macos"), home) {
        (true, Some(home)) => vec![home.join("Library/Logs/DiagnosticReports")],
        _ => Vec::new(),
    }
}

fn crashes_in(dir: &Path, names: &[String], now: SystemTime) -> io::Result<Vec<Part>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        // `ely-doc-2026-10-08-013420.ips`, helpers with their suffix.
        let ours = names.iter().any(|own| name.starts_with(&format!("{own}-")) || name.starts_with(&format!("{own} ")));
        let meta = entry.metadata()?;
        let fresh = meta.modified().is_ok_and(|at| now.duration_since(at).is_ok_and(|age| age < CRASH_AGE));
        if ours && fresh && meta.is_file() {
            found.push(Part { path: entry.path(), bytes: meta.len() });
        }
    }
    found.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(found)
}

/// Redaction replaces every path, URL and user name.
#[derive(Clone, Debug, Default)]
pub struct Redact {
    pub names: Vec<String>,
}

impl Redact {
    /// This account's names and the set author; unreadable ones fail.
    pub fn of_account() -> io::Result<Self> {
        Self::of(crate::user::account(), crate::settings::Shared::load())
    }

    fn of(account: io::Result<String>, shared: io::Result<crate::settings::Shared>) -> io::Result<Self> {
        let mut names = vec![account?, shared?.author];
        names.extend(std::env::var("USER").ok());
        names.extend(std::env::var("USERNAME").ok());
        Ok(Self { names })
    }

    pub fn apply(&self, text: &str) -> String {
        let mut out: String = text.split_inclusive('\n').map(path_free).collect();
        // Longest first, so short names never split long ones.
        let mut names: Vec<&str> = self.names.iter().map(|name| name.trim()).filter(|name| !name.is_empty()).collect();
        names.sort_by_key(|name| std::cmp::Reverse(name.len()));
        for name in names {
            out = out.replace(name, "<user>");
        }
        out
    }
}

/// Paths and URLs become `<path>`, unquoted ones to line end.
fn path_free(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut at = 0;
    while at < chars.len() {
        let before = at.checked_sub(1).map(|i| chars[i]);
        let opens = before.is_none_or(|c| c.is_whitespace() || "\"'([=:<".contains(c));
        if !(opens && starts_path(&chars[at..])) {
            out.push(chars[at]);
            at += 1;
            continue;
        }
        // Quoted: up to the quote; else the whole line.
        let end = match before.filter(|c| *c == '"' || *c == '\'') {
            Some(quote) => closing(&chars, at, quote),
            None => chars.len() - usize::from(chars.last() == Some(&'\n')),
        };
        out.push_str("<path>");
        at = end;
    }
    out
}

/// Where `quote` closes; a backslash escapes in double quotes (JSON).
fn closing(chars: &[char], from: usize, quote: char) -> usize {
    let mut at = from;
    while at < chars.len() {
        match chars[at] {
            '\\' if quote == '"' => at += 2,
            c if c == quote => return at,
            _ => at += 1,
        }
    }
    chars.len()
}

/// `/x`, `~/x`, `C:\x`, `\\host`, or `scheme://`.
fn starts_path(rest: &[char]) -> bool {
    let at = |n: usize| rest.get(n).copied();
    let drive = at(0).is_some_and(|c| c.is_ascii_alphabetic()) && at(1) == Some(':') && matches!(at(2), Some('\\' | '/'));
    let scheme = {
        let letters = rest.iter().take_while(|c| c.is_ascii_alphanumeric() || **c == '+' || **c == '-').count();
        letters > 1 && rest[letters..].starts_with(&[':', '/', '/'])
    };
    let rooted = matches!(at(0), Some('/') | Some('\\')) && at(1).is_some_and(|c| !c.is_whitespace());
    rooted || (at(0) == Some('~') && at(1) == Some('/')) || drive || scheme
}

/// One text file holding every part, redacted when asked.
pub fn bundle(app: AppId, parts: &[Part], redact: Option<&Redact>) -> io::Result<String> {
    let mut text = format!(
        "{} {} diagnostics\nsystem: {} {}\n",
        app.name(),
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    for part in parts {
        let mut file = std::fs::File::open(&part.path)?;
        let length = file.metadata()?.len();
        let skipped = length.saturating_sub(part.bytes);
        // A cut line may hold half a path: skip it.
        let mut before = *b"\n";
        if skipped > 0 {
            file.seek(SeekFrom::Start(skipped - 1))?;
            file.read_exact(&mut before)?;
        }
        file.seek(SeekFrom::Start(skipped))?;
        let mut body = Vec::new();
        file.take(part.bytes).read_to_end(&mut body)?;
        if before[0] != b'\n' {
            let whole = body.iter().position(|b| *b == b'\n').map_or(body.len(), |n| n + 1);
            body.drain(..whole);
        }
        let name = part.path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        text.push_str(&format!("\n===== {name} =====\n"));
        text.push_str(&String::from_utf8_lossy(&body));
    }
    Ok(match redact {
        Some(redact) => redact.apply(&text),
        None => text,
    })
}

#[cfg(test)]
mod tests;
