//! Reflog, moving a branch back, cleaning with a preview (TREE-13).

use std::{io, path::Path};

use ely_ipc::Worded;

use crate::git::Git;

fn bad(what: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("an unread record {what:?}"),
    )
}

/// NUL-split records of `n` fields; a field may be empty.
pub fn records(bytes: &[u8], n: usize) -> io::Result<Vec<Vec<&str>>> {
    let text = std::str::from_utf8(bytes).map_err(|_| bad("not UTF-8"))?;
    let mut fields: Vec<&str> = text.split('\0').collect();
    if fields.last() == Some(&"") {
        fields.pop();
    }
    if !fields.len().is_multiple_of(n) {
        return Err(bad(text));
    }
    Ok(fields.chunks(n).map(<[&str]>::to_vec).collect())
}

/// Where HEAD was; newest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moved {
    pub oid: String,
    /// `HEAD@{3}`.
    pub name: String,
    /// What moved it, as Git wrote.
    pub said: String,
    pub at: i64,
    /// Its count among identical moves, from the oldest.
    pub seq: usize,
    /// How many identical moves there are.
    pub twins: usize,
}

impl Moved {
    /// The same event; a lost twin leaves it unknown.
    pub fn same(&self, other: &Moved) -> bool {
        (&self.oid, &self.said, self.at, self.seq, self.twins)
            == (&other.oid, &other.said, other.at, other.seq, other.twins)
    }
}

/// How many reflog entries the panel lists.
const KEEP: usize = 200;

pub fn reflog(git: &Git, root: &Path) -> io::Result<Vec<Moved>> {
    if !git.born(root)? {
        return Ok(Vec::new());
    }
    // With a date mode, `%gd` carries when HEAD moved.
    let format = "--format=%H%x00%gd%x00%gs";
    // One read of all: twins count from the oldest.
    let out = git.run_or_one(
        root,
        &["reflog", "show", "-z", "--date=unix", format, "HEAD", "--"],
    )?;
    let mut moves = records(&out, 3)?
        .into_iter()
        .enumerate()
        .map(|(at, fields)| {
            let [oid, when, said] = fields[..] else {
                return Err(bad(&fields.join(" ")));
            };
            let time = when
                .strip_prefix("HEAD@{")
                .and_then(|when| when.strip_suffix('}'))
                .and_then(|when| when.parse().ok())
                .ok_or_else(|| bad(when))?;
            Ok(Moved {
                oid: oid.to_string(),
                name: format!("HEAD@{{{at}}}"),
                said: said.to_string(),
                at: time,
                seq: 0,
                twins: 0,
            })
        })
        .collect::<io::Result<Vec<_>>>()?;
    // New twins come newest; Git may drop any one.
    let mut seen = std::collections::HashMap::new();
    for moved in moves.iter_mut().rev() {
        let count = seen.entry((moved.oid.clone(), moved.said.clone(), moved.at)).or_insert(0);
        moved.seq = *count;
        *count += 1;
    }
    for moved in &mut moves {
        moved.twins = seen[&(moved.oid.clone(), moved.said.clone(), moved.at)];
    }
    moves.truncate(KEEP);
    Ok(moves)
}

/// Moving HEAD's branch to a commit: what leaves it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Move {
    /// `refs/heads/main`, or none when detached.
    pub branch: Option<String>,
    pub head: String,
    pub to: String,
    /// Commits left behind: `abc1234 subject`.
    pub left: Vec<String>,
}

fn words(bytes: Vec<u8>) -> String {
    String::from_utf8_lossy(&bytes).trim().to_string()
}

/// HEAD's branch ref, if any, and its commit.
pub fn head_now(git: &Git, root: &Path) -> io::Result<(Option<String>, String)> {
    let branch = words(git.run_or_one(root, &["symbolic-ref", "-q", "HEAD"])?);
    let head = words(git.run(root, &["rev-parse", "--verify", "HEAD"])?);
    Ok(((!branch.is_empty()).then_some(branch), head))
}

/// `reset --keep` would drop staged work; refuse instead.
pub fn nothing_staged(git: &Git, root: &Path) -> io::Result<()> {
    if git
        .run(root, &["diff", "--cached", "--name-only", "-z"])?
        .is_empty()
    {
        return Ok(());
    }
    Err(Worded::error(io::ErrorKind::Other, "tree.error.staged", &[]))
}

pub fn plan_move(git: &Git, root: &Path, to: &str) -> io::Result<Move> {
    nothing_staged(git, root)?;
    let (branch, head) = head_now(git, root)?;
    let range = format!("{to}..{head}");
    let left = words(git.run(root, &["log", "--format=%h %s", &range, "--"])?);
    Ok(Move {
        branch,
        head,
        to: to.to_string(),
        left: left.lines().map(str::to_string).collect(),
    })
}

/// Undoes Git's C quoting of a path.
fn unquote(said: &str) -> io::Result<String> {
    let Some(inner) = said.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else {
        return Ok(said.to_string());
    };
    let (mut out, mut chars) = (Vec::new(), inner.chars());
    while let Some(c) = chars.next() {
        if c != '\\' {
            let mut buffer = [0; 4];
            out.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
            continue;
        }
        let byte = match chars.next().ok_or_else(|| bad(said))? {
            'a' => 7,
            'b' => 8,
            't' => b'\t',
            'n' => b'\n',
            'v' => 11,
            'f' => 12,
            'r' => b'\r',
            '"' => b'"',
            '\\' => b'\\',
            digit @ '0'..='3' => {
                let rest: String = chars.by_ref().take(2).collect();
                u8::from_str_radix(&format!("{digit}{rest}"), 8).map_err(|_| bad(said))?
            }
            _ => return Err(bad(said)),
        };
        out.push(byte);
    }
    String::from_utf8(out).map_err(|_| bad(said))
}

/// The flags a clean runs with, preview and real.
pub fn clean_flags(ignored: bool) -> Vec<String> {
    let mut flags = vec!["-d".to_string()];
    flags.extend(ignored.then(|| "-x".to_string()));
    flags
}

/// What a clean removes, fixed when shown.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Clean {
    /// Each file, folders opened up.
    pub files: Vec<String>,
    /// Folders, deepest first; removed only once empty.
    pub dirs: Vec<String>,
}

/// Folders under `dir`, skipping other repositories, deepest first.
fn folders(root: &Path, dir: &str, out: &mut Vec<String>) -> io::Result<()> {
    for entry in std::fs::read_dir(root.join(dir))? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| bad("a folder name"))?;
        let inner = format!("{dir}{name}/");
        if kind.is_dir() && !entry.path().join(".git").exists() {
            folders(root, &inner, out)?;
        }
    }
    out.push(dir.to_string());
    Ok(())
}

/// What `git clean` would remove, file by file.
pub fn clean_preview(git: &Git, root: &Path, ignored: bool) -> io::Result<Clean> {
    let mut args: Vec<String> = ["-c", "core.quotePath=false", "clean", "-n"]
        .map(String::from)
        .into();
    args.extend(clean_flags(ignored));
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = String::from_utf8(git.run(root, &args)?).map_err(|_| bad("not UTF-8"))?;
    let mut clean = Clean::default();
    for path in out
        .lines()
        .filter_map(|line| line.strip_prefix("Would remove "))
    {
        let path = unquote(path)?;
        if !path.ends_with('/') {
            clean.files.push(path);
            continue;
        }
        let mut list = vec!["ls-files", "--others", "-z"];
        if !ignored {
            list.push("--exclude-standard");
        }
        list.extend(["--", &path]);
        let inside = git.run(root, &list)?;
        let inside = std::str::from_utf8(&inside).map_err(|_| bad("not UTF-8"))?;
        // Another repository inside stays, as clean leaves it.
        clean.files.extend(
            inside
                .split('\0')
                .filter(|file| !file.is_empty() && !file.ends_with('/'))
                .map(str::to_string),
        );
        folders(root, &path, &mut clean.dirs)?;
    }
    Ok(clean)
}

#[cfg(test)]
mod tests {
    #[test]
    fn quoted_paths_come_back_whole() {
        let read = |said| super::unquote(said).expect("reads");
        assert_eq!(read("plain name.txt"), "plain name.txt");
        assert_eq!(read(r#""tab\there.txt""#), "tab\there.txt");
        assert_eq!(read(r#""say \"hi\"\\.txt""#), "say \"hi\"\\.txt");
        assert_eq!(read(r#""\344\270\255.txt""#), "中.txt");
        assert!(super::unquote(r#""bad\q""#).is_err());
    }

    #[test]
    fn records_keep_empty_fields() {
        let read = super::records(b"a\0\0c\0d\0e\0f\0", 3).expect("reads");
        assert_eq!(read, [vec!["a", "", "c"], vec!["d", "e", "f"]]);
        assert!(super::records(b"a\0b\0", 3).is_err());
    }
}
