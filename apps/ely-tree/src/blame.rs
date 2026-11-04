//! Who last changed each line, from `blame --porcelain` (TREE-06).

use std::{collections::HashMap, io, path::Path};

use ely_ipc::Worded;

use crate::git::Git;

/// A commit as blame names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Owner {
    pub commit: String,
    pub author: String,
    pub at: i64,
    pub subject: String,
}

impl Owner {
    /// Lines not committed yet carry an all-zero id.
    pub fn uncommitted(&self) -> bool {
        self.commit.bytes().all(|b| b == b'0')
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blamed {
    pub code: String,
    /// Each line's owner, as a place in `owners`.
    pub lines: Vec<usize>,
    pub owners: Vec<Owner>,
}

fn bad(line: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("an unread blame line {line:?}"),
    )
}

pub fn parse(bytes: &[u8]) -> io::Result<Blamed> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| Worded::error(io::ErrorKind::InvalidData, "tree.error.not_utf8", &[]))?;
    let mut blamed = Blamed {
        code: String::new(),
        lines: Vec::new(),
        owners: Vec::new(),
    };
    let mut known: HashMap<String, usize> = HashMap::new();
    let mut current: Option<usize> = None;
    for line in text.split('\n').filter(|line| !line.is_empty()) {
        if let Some(code) = line.strip_prefix('\t') {
            blamed.lines.push(current.ok_or_else(|| bad(line))?);
            blamed.code.push_str(code);
            blamed.code.push('\n');
            continue;
        }
        let (key, value) = line.split_once(' ').unwrap_or((line, ""));
        let header = key.len() >= 40 && key.bytes().all(|b| b.is_ascii_hexdigit());
        let owner = current.map(|at| &mut blamed.owners[at]);
        match (header, key, owner) {
            (true, _, _) => {
                let next = blamed.owners.len();
                let at = *known.entry(key.to_string()).or_insert(next);
                if at == next {
                    blamed.owners.push(Owner {
                        commit: key.into(),
                        author: String::new(),
                        at: 0,
                        subject: String::new(),
                    });
                }
                current = Some(at);
            }
            (false, "author", Some(owner)) => owner.author = value.to_string(),
            (false, "author-time", Some(owner)) => {
                owner.at = value.parse().map_err(|_| bad(line))?
            }
            (false, "summary", Some(owner)) => owner.subject = value.to_string(),
            // Mail, committer, filename and the rest are not shown.
            _ => {}
        }
    }
    Ok(blamed)
}

/// The work tree's file, line by line.
pub fn read(git: &Git, root: &Path, path: &str) -> io::Result<Blamed> {
    parse(&git.run(root, &["blame", "--porcelain", "--", path])?)
}
