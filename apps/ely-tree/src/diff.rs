//! Git's own diff, read and cut into patches (TREE-03, TREE-04).

use std::{collections::BTreeSet, io};

use ely_gpui_component::git::LineKind;

/// How whitespace counts: exact, `-b`, or `-w`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Space {
    #[default]
    Exact,
    Changes,
    All,
}

impl Space {
    pub fn flag(self) -> Option<&'static str> {
        match self {
            Space::Exact => None,
            Space::Changes => Some("--ignore-space-change"),
            Space::All => Some("--ignore-all-space"),
        }
    }
}

/// One line as Git printed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Raw {
    pub kind: LineKind,
    pub text: String,
    /// False when Git said "No newline at end of file".
    pub newline: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk {
    /// Start and count, as in `@@ -s,c +s,c @@`.
    pub old: (usize, usize),
    pub new: (usize, usize),
    pub tail: String,
    pub lines: Vec<Raw>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Body {
    Text(Vec<Hunk>),
    Binary,
    /// Not UTF-8; shown as a file, staged as a file.
    Undecodable,
}

/// One file's diff: its header lines and body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDiff {
    pub head: Vec<String>,
    pub body: Body,
}

impl FileDiff {
    /// One per `diff --git` header; a type change prints two.
    pub fn sections(&self) -> usize {
        self.head
            .iter()
            .filter(|line| line.starts_with("diff --git "))
            .count()
    }

    /// Hunks and lines stage only for a plain content change.
    pub fn partial(&self) -> bool {
        const WHOLE: [&str; 7] = [
            "new file",
            "deleted file",
            "rename ",
            "copy ",
            "old mode",
            "new mode",
            "similarity",
        ];
        matches!(self.body, Body::Text(_))
            && !self
                .head
                .iter()
                .any(|line| WHOLE.iter().any(|w| line.starts_with(w)))
    }

    /// Either side is a symbolic link (mode 120000).
    pub fn link(&self) -> bool {
        const MODES: [&str; 5] = ["new file mode ", "deleted file mode ", "old mode ", "new mode ", "index "];
        self.head.iter().any(|line| {
            MODES.iter().any(|m| line.starts_with(m)) && line.ends_with(" 120000")
        })
    }
}

fn bad(line: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("an unread diff line {line:?}"),
    )
}

/// `-s,c` or `+s` as start and count.
fn range(part: &str, sign: char, line: &str) -> io::Result<(usize, usize)> {
    let part = part.strip_prefix(sign).ok_or_else(|| bad(line))?;
    let (start, count) = part.split_once(',').unwrap_or((part, "1"));
    Ok((
        start.parse().map_err(|_| bad(line))?,
        count.parse().map_err(|_| bad(line))?,
    ))
}

/// Reads one file's `git diff` output.
pub fn parse(bytes: &[u8]) -> io::Result<FileDiff> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Ok(FileDiff {
            head: Vec::new(),
            body: Body::Undecodable,
        });
    };
    let mut head = Vec::new();
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut lines = text.split('\n').peekable();
    // A type change prints one path as two sections.
    let mut in_head = true;
    let mut binary = false;
    while let Some(line) = lines.next() {
        if line.is_empty() && lines.peek().is_none() {
            break;
        }
        if let Some(rest) = line.strip_prefix("@@ ") {
            let (ranges, tail) = rest.split_once(" @@").ok_or_else(|| bad(line))?;
            let (old, new) = ranges.split_once(' ').ok_or_else(|| bad(line))?;
            hunks.push(Hunk {
                old: range(old, '-', line)?,
                new: range(new, '+', line)?,
                tail: tail.trim_start().to_string(),
                lines: Vec::new(),
            });
            in_head = false;
            continue;
        }
        in_head |= line.starts_with("diff --git ");
        let Some(hunk) = hunks.last_mut().filter(|_| !in_head) else {
            // Later sections still give their headers.
            if line.starts_with("Binary files ") {
                binary = true;
                continue;
            }
            head.push(line.to_string());
            continue;
        };
        let (kind, rest) = match line.split_at_checked(1) {
            Some((" ", rest)) => (LineKind::Same, rest),
            Some(("+", rest)) => (LineKind::Added, rest),
            Some(("-", rest)) => (LineKind::Removed, rest),
            Some(("\\", _)) => {
                hunk.lines.last_mut().ok_or_else(|| bad(line))?.newline = false;
                continue;
            }
            _ => return Err(bad(line)),
        };
        hunk.lines.push(Raw {
            kind,
            text: rest.to_string(),
            newline: true,
        });
    }
    let body = if binary {
        Body::Binary
    } else {
        Body::Text(hunks)
    };
    Ok(FileDiff { head, body })
}

/// Lines before a range: the start, less one unless empty.
fn before((start, count): (usize, usize)) -> usize {
    if count == 0 { start } else { start - 1 }
}

fn start(before: usize, count: usize) -> usize {
    if count == 0 { before } else { before + 1 }
}

/// Patch of chosen `(hunk, line)`s; `reverse` unstages.
pub fn patch(diff: &FileDiff, chosen: &BTreeSet<(usize, usize)>, reverse: bool) -> Option<String> {
    let Body::Text(hunks) = &diff.body else {
        return None;
    };
    let mut out = String::new();
    // New less old lines in the hunks written so far.
    let mut shift: isize = 0;
    for (at, hunk) in hunks.iter().enumerate() {
        let picked = |line: usize| chosen.contains(&(at, line));
        if !hunk
            .lines
            .iter()
            .enumerate()
            .any(|(line, raw)| raw.kind != LineKind::Same && picked(line))
        {
            continue;
        }
        // Kept lines: sign, text, newline.
        let mut cut: Vec<(char, &str, bool)> = Vec::new();
        for (line, raw) in hunk.lines.iter().enumerate() {
            let sign = match (raw.kind, picked(line), reverse) {
                (LineKind::Same, _, _)
                | (LineKind::Removed, false, false)
                | (LineKind::Added, false, true) => ' ',
                (LineKind::Removed, true, _) => '-',
                (LineKind::Added, true, _) => '+',
                (LineKind::Removed, false, true) | (LineKind::Added, false, false) => continue,
            };
            cut.push((sign, &raw.text, raw.newline));
        }
        let mut body = String::new();
        let (mut olds, mut news) = (0, 0);
        let mut line = |sign: char, text: &str, newline: bool| {
            olds += usize::from(sign != '+');
            news += usize::from(sign != '-');
            body.push(sign);
            body.push_str(text);
            body.push('\n');
            if !newline {
                body.push_str("\\ No newline at end of file\n");
            }
        };
        for (at, &(sign, text, newline)) in cut.iter().enumerate() {
            // A line gains a newline where lines follow it.
            let followed = |side: char| {
                cut[at + 1..]
                    .iter()
                    .any(|(next, ..)| *next == ' ' || *next == side)
            };
            let (old, new) = (newline || followed('-'), newline || followed('+'));
            match sign {
                '-' => line('-', text, old),
                '+' => line('+', text, new),
                _ if old == new => line(' ', text, old),
                _ => {
                    line('-', text, old);
                    line('+', text, new);
                }
            }
        }
        // The index side keeps Git's numbers; the other follows.
        let (old, new) = if reverse {
            let fixed = before(hunk.new);
            let other = fixed.checked_add_signed(-shift)?;
            ((start(other, olds), olds), (start(fixed, news), news))
        } else {
            let fixed = before(hunk.old);
            let other = fixed.checked_add_signed(shift)?;
            ((start(fixed, olds), olds), (start(other, news), news))
        };
        shift += news as isize - olds as isize;
        out.push_str(&format!(
            "@@ -{},{} +{},{} @@\n{body}",
            old.0, old.1, new.0, new.1
        ));
    }
    if out.is_empty() {
        return None;
    }
    Some(format!("{}\n{out}", diff.head.join("\n")))
}
