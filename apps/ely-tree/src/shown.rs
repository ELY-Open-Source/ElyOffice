//! The middle's file, read off the UI thread (TREE-03).

use std::{io, path::Path, rc::Rc, sync::Arc};

use ely_gpui_component::git::{ChangeSection, DiffLine, LineKind, Stretch, mark_words};
use gpui::{Image, ImageFormat};

use crate::{
    diff::{self, Body, FileDiff, Space},
    git::Git,
    status::{Entry, State},
};

/// Picture formats shown side by side, by extension.
const PICTURES: [(&str, ImageFormat); 9] = [
    ("png", ImageFormat::Png),
    ("jpg", ImageFormat::Jpeg),
    ("jpeg", ImageFormat::Jpeg),
    ("gif", ImageFormat::Gif),
    ("webp", ImageFormat::Webp),
    ("bmp", ImageFormat::Bmp),
    ("tif", ImageFormat::Tiff),
    ("tiff", ImageFormat::Tiff),
    ("ico", ImageFormat::Ico),
];

/// What a read found, before it becomes a view.
pub enum Found {
    Text {
        diff: FileDiff,
        seen: Vec<u8>,
        old: Option<String>,
    },
    Picture {
        format: ImageFormat,
        old: Option<Vec<u8>>,
        new: Option<Vec<u8>>,
    },
    Binary {
        old: Option<u64>,
        new: Option<u64>,
    },
    Folder,
    Conflict(crate::conflict::Stages),
}

/// What the middle draws.
pub enum View {
    Text {
        diff: Arc<FileDiff>,
        /// The output read, to see a change before a patch.
        seen: Arc<Vec<u8>>,
        stretches: Rc<Vec<Stretch>>,
        /// Each stretch's hunk, if it is one.
        hunks: Vec<Option<usize>>,
    },
    Picture {
        old: Option<Arc<Image>>,
        new: Option<Arc<Image>>,
    },
    Binary {
        old: Option<u64>,
        new: Option<u64>,
    },
    Folder,
    Conflict(crate::conflict::Stages),
}

/// How every diff is asked for, before what it compares.
pub fn base(space: Space) -> Vec<String> {
    // Blank context lines keep their leading space.
    let mut args: Vec<String> = [
        "-c",
        "diff.suppressBlankEmpty=false",
        "diff",
        "--no-color",
        "--no-ext-diff",
        "--no-textconv",
        "--src-prefix=a/",
        "--dst-prefix=b/",
        "-U3",
    ]
    .map(String::from)
    .to_vec();
    args.extend(space.flag().map(String::from));
    args
}

/// The arguments that read `entry` in `section`.
pub fn args(entry: &Entry, section: ChangeSection, space: Space) -> Vec<String> {
    let mut args = base(space);
    match section {
        ChangeSection::Staged => {
            args.extend(["--cached", "-M", "--"].map(String::from));
            args.push(entry.path.clone());
            args.extend(entry.from.clone());
        }
        ChangeSection::Untracked => {
            args.extend(["--no-index", "--", "/dev/null"].map(String::from));
            args.push(entry.path.clone());
        }
        _ => {
            args.push("--".into());
            args.push(entry.path.clone());
        }
    }
    args
}

/// What Git compares: a link's own path, else the file.
fn work_bytes(path: &Path) -> io::Result<Vec<u8>> {
    match std::fs::symlink_metadata(path)?.file_type().is_symlink() {
        true => Ok(std::fs::read_link(path)?
            .into_os_string()
            .into_encoded_bytes()),
        false => std::fs::read(path),
    }
}

fn blob(git: &Git, root: &Path, name: &str) -> io::Result<Vec<u8>> {
    git.run(root, &["cat-file", "blob", name])
}

fn size(git: &Git, root: &Path, name: &str) -> io::Result<u64> {
    let said = git.run(root, &["cat-file", "-s", name])?;
    String::from_utf8_lossy(&said)
        .trim()
        .parse()
        .map_err(io::Error::other)
}

/// Old and new blobs; None means the work tree.
fn sides(
    entry: &Entry,
    section: ChangeSection,
    born: bool,
) -> (Option<String>, Option<Option<String>>) {
    let deleted = |letter| matches!(entry.state, State::Tracked { staged, unstaged } if (if section == ChangeSection::Staged { staged } else { unstaged }) == letter);
    let old = match section {
        ChangeSection::Staged if born && !deleted('A') => Some(format!(
            "HEAD:{}",
            entry.from.as_deref().unwrap_or(&entry.path)
        )),
        ChangeSection::Unstaged => Some(format!(":{}", entry.path)),
        _ => None,
    };
    let new = match (section, deleted('D')) {
        (_, true) => None,
        (ChangeSection::Staged, false) => Some(Some(format!(":{}", entry.path))),
        _ => Some(None),
    };
    (old, new)
}

/// Reads what `entry` changed, as Git sees it.
pub fn read(
    git: &Git,
    root: &Path,
    entry: &Entry,
    section: ChangeSection,
    space: Space,
    born: bool,
) -> io::Result<Found> {
    if matches!(entry.state, State::Conflicted(_)) {
        return Ok(Found::Conflict(crate::conflict::read(
            git,
            root,
            &entry.path,
        )?));
    }
    if entry.path.ends_with('/') {
        return Ok(Found::Folder);
    }
    let args = args(entry, section, space);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let seen = match section {
        ChangeSection::Untracked => git.run_or_one(root, &args)?,
        _ => git.run(root, &args)?,
    };
    let (old, new) = sides(entry, section, born);
    compare(git, root, seen, &entry.path, old, new)
}

/// Makes what Git printed for `path` into a view's parts.
pub fn compare(
    git: &Git,
    root: &Path,
    seen: Vec<u8>,
    path: &str,
    old: Option<String>,
    new: Option<Option<String>>,
) -> io::Result<Found> {
    let diff = diff::parse(&seen)?;
    let work = root.join(path);
    let extension = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    // A link is its path, never a picture.
    let picture = PICTURES
        .iter()
        .find(|(e, _)| extension.as_deref() == Some(*e) && !diff.link())
        .map(|(_, f)| *f);
    match (&diff.body, picture) {
        (Body::Binary, Some(format)) => {
            let old = old.map(|name| blob(git, root, &name)).transpose()?;
            let new = match new {
                Some(Some(name)) => Some(blob(git, root, &name)?),
                Some(None) => Some(work_bytes(&work)?),
                None => None,
            };
            Ok(Found::Picture { format, old, new })
        }
        (Body::Binary | Body::Undecodable, _) => {
            let old = old.map(|name| size(git, root, &name)).transpose()?;
            let new = match new {
                Some(Some(name)) => Some(size(git, root, &name)?),
                Some(None) => Some(std::fs::symlink_metadata(&work)?.len()),
                None => None,
            };
            Ok(Found::Binary { old, new })
        }
        (Body::Text(_), _) => {
            let old = old.map(|name| blob(git, root, &name)).transpose()?;
            let new = match new {
                Some(Some(name)) => Some(blob(git, root, &name)?),
                Some(None) => Some(work_bytes(&work)?),
                None => None,
            };
            // Not UTF-8 throughout: shown by size.
            let text = |bytes: &Option<Vec<u8>>| {
                bytes
                    .as_deref()
                    .is_none_or(|b| std::str::from_utf8(b).is_ok())
            };
            if !text(&old) || !text(&new) {
                let size = |bytes: Option<Vec<u8>>| bytes.map(|b| b.len() as u64);
                return Ok(Found::Binary {
                    old: size(old),
                    new: size(new),
                });
            }
            let old = old
                .map(String::from_utf8)
                .transpose()
                .map_err(io::Error::other)?;
            Ok(Found::Text { diff, seen, old })
        }
    }
}

fn line(raw: &diff::Raw, old: Option<usize>, new: Option<usize>) -> DiffLine {
    DiffLine {
        kind: raw.kind,
        old,
        new,
        text: raw.text.trim_end_matches('\r').to_string().into(),
        words: Vec::new(),
    }
}

/// Hunks from Git, the unchanged lines between folded.
pub fn stretches(
    diff: &FileDiff,
    old: Option<&str>,
) -> io::Result<(Vec<Stretch>, Vec<Option<usize>>)> {
    let Body::Text(hunks) = &diff.body else {
        return Ok((Vec::new(), Vec::new()));
    };
    // An empty file has no lines, not one empty line.
    let olds: Vec<&str> = match old {
        Some(text) if !text.is_empty() => text
            .strip_suffix('\n')
            .unwrap_or(text)
            .split('\n')
            .collect(),
        _ => Vec::new(),
    };
    let (mut out, mut places) = (Vec::new(), Vec::new());
    // The next old line not yet shown, from one.
    let mut next = 1;
    let fold = |from: usize, to: usize, shift: isize| -> io::Result<Vec<DiffLine>> {
        (from..to)
            .map(|n| {
                let text = olds
                    .get(n - 1)
                    .ok_or_else(|| io::Error::other("the old file is shorter than its diff"))?;
                Ok(DiffLine {
                    kind: LineKind::Same,
                    old: Some(n),
                    new: n.checked_add_signed(shift),
                    text: text.trim_end_matches('\r').to_string().into(),
                    words: Vec::new(),
                })
            })
            .collect()
    };
    // Two sections share no old text to fold.
    let folds = old.is_some() && diff.sections() < 2;
    // New less old line numbers, after the last hunk.
    let mut shift = 0isize;
    for (at, hunk) in hunks.iter().enumerate() {
        let (mut o, mut n) = (diff_start(hunk.old), diff_start(hunk.new));
        let before = n as isize - o as isize;
        if folds && o > next {
            out.push(Stretch::Folded(fold(next, o, before)?));
            places.push(None);
        }
        let mut lines = Vec::new();
        for raw in &hunk.lines {
            lines.push(match raw.kind {
                LineKind::Same => line(raw, Some(o), Some(n)),
                LineKind::Removed => line(raw, Some(o), None),
                LineKind::Added => line(raw, None, Some(n)),
            });
            o += usize::from(raw.kind != LineKind::Added);
            n += usize::from(raw.kind != LineKind::Removed);
        }
        mark_words(&mut lines);
        let header = format!(
            "@@ -{},{} +{},{} @@ {}",
            hunk.old.0, hunk.old.1, hunk.new.0, hunk.new.1, hunk.tail
        );
        out.push(Stretch::Hunk {
            header: header.trim_end().to_string().into(),
            lines,
        });
        places.push(Some(at));
        next = o;
        shift = n as isize - o as isize;
    }
    if folds && next <= olds.len() {
        out.push(Stretch::Folded(fold(next, olds.len() + 1, shift)?));
        places.push(None);
    }
    Ok((out, places))
}

/// A range's first line number; an empty range starts after.
fn diff_start((start, count): (usize, usize)) -> usize {
    if count == 0 { start + 1 } else { start }
}

/// Makes the view on the UI thread.
pub fn view(found: Found) -> io::Result<View> {
    Ok(match found {
        Found::Text { diff, seen, old } => {
            let (stretches, hunks) = stretches(&diff, old.as_deref())?;
            View::Text {
                diff: Arc::new(diff),
                seen: Arc::new(seen),
                stretches: Rc::new(stretches),
                hunks,
            }
        }
        Found::Picture { format, old, new } => {
            let image = |bytes: Vec<u8>| Arc::new(Image::from_bytes(format, bytes));
            View::Picture {
                old: old.map(image),
                new: new.map(image),
            }
        }
        Found::Binary { old, new } => View::Binary { old, new },
        Found::Folder => View::Folder,
        Found::Conflict(stages) => View::Conflict(stages),
    })
}
