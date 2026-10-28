//! Stage and discard, one write at a time (§13.3).

use std::{io, sync::Arc};

use ely_gpui_component::{
    git::{ChangeAction, ChangeSection},
    primitives::Severity,
};
use gpui::{AppContext as _, Context, SharedString};

use crate::{
    asks::Ask,
    git::Git,
    ops, recover,
    status::{Entry, State},
    tree::{Tree, text},
};

/// A write to the index or the work tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Write {
    Stage(Entry),
    Unstage(Entry),
    Discard(Entry),
    StageAll,
    UnstageAll,
    /// The staged changes, or amending the last commit.
    Commit {
        message: String,
        amend: bool,
    },
    /// A merge result into its work file, between Git's writes.
    Save {
        path: String,
        text: String,
    },
    /// A branch, tag, history or conflict command, as given.
    Git(Vec<String>),
    /// Talks to a remote: progress shows, and it can stop.
    Remote(Vec<String>),
    /// `reset --keep to`, once HEAD is still where shown.
    Move {
        branch: Option<String>,
        head: String,
        to: String,
    },
    /// The shown files, then folders left empty.
    Clean {
        files: Vec<String>,
        dirs: Vec<String>,
        ignored: bool,
    },
    /// `stash <verb> stash@{at}`, once it is still `oid`.
    Stash {
        verb: &'static str,
        at: usize,
        oid: String,
    },
    /// `git config` for this repository, or for the user.
    Config {
        key: String,
        value: String,
        global: bool,
    },
    /// Picked lines; `check` rereads the diff they were cut from.
    Patch {
        patch: String,
        reverse: bool,
        check: Vec<String>,
        seen: Arc<Vec<u8>>,
    },
}

impl Write {
    /// Git's arguments; `born` is false before the first commit.
    pub fn args(&self, born: bool) -> Vec<String> {
        // Unstaging a rename restores its old path too.
        let paths = |entry: &Entry, both: bool| {
            let mut paths = vec!["--".to_string(), entry.path.clone()];
            paths.extend(entry.from.clone().filter(|_| both));
            paths
        };
        let mut args: Vec<String> = match (self, born) {
            (Write::Stage(_), _) => vec!["add".into(), "--all".into()],
            (Write::Unstage(_), true) => vec!["restore".into(), "--staged".into()],
            // `-f`: the index may differ from the file.
            (Write::Unstage(_), false) => vec![
                "rm".into(),
                "--cached".into(),
                "-f".into(),
                "-r".into(),
                "-q".into(),
            ],
            (
                Write::Discard(Entry {
                    state: State::Untracked,
                    ..
                }),
                _,
            ) => vec!["clean".into(), "-f".into(), "-d".into(), "-q".into()],
            (Write::Discard(_), _) => vec!["restore".into(), "--worktree".into()],
            (Write::StageAll, _) => vec!["add".into(), "--update".into()],
            (Write::UnstageAll, true) => vec!["reset".into(), "-q".into()],
            (Write::UnstageAll, false) => vec![
                "rm".into(),
                "--cached".into(),
                "-f".into(),
                "-r".into(),
                "-q".into(),
                "--".into(),
                ".".into(),
            ],
            (Write::Git(args) | Write::Remote(args), _) => args.clone(),
            (Write::Move { to, .. }, _) => vec!["reset".into(), "--keep".into(), to.clone()],
            // Files only: Git keeps tracked ones and stops at links.
            (Write::Clean { files, ignored, .. }, _) => {
                let mut args = vec!["clean".into(), "-f".into()];
                args.extend(ignored.then(|| "-x".into()));
                args.push("--".into());
                args.extend(files.iter().cloned());
                args
            }
            (Write::Stash { verb, at, .. }, _) => {
                vec!["stash".into(), (*verb).into(), format!("stash@{{{at}}}")]
            }
            (Write::Save { .. }, _) => Vec::new(),
            (Write::Commit { amend, .. }, _) => {
                // `whitespace`: lines starting `#` stay, whatever the config.
                let mut args = vec![
                    "commit".into(),
                    "--cleanup=whitespace".into(),
                    "-F".into(),
                    "-".into(),
                ];
                args.extend(amend.then(|| "--amend".into()));
                args
            }
            (Write::Config { key, value, global }, _) => {
                let mut args = vec!["config".into()];
                args.extend(global.then(|| "--global".into()));
                args.extend([key.clone(), value.clone()]);
                args
            }
            (Write::Patch { reverse, .. }, _) => {
                // `nowarn`: the picked bytes go in as they are.
                let mut args = vec![
                    "apply".into(),
                    "--cached".into(),
                    "--whitespace=nowarn".into(),
                ];
                args.extend(reverse.then(|| "--reverse".into()));
                args
            }
        };
        match self {
            Write::Unstage(entry) => args.extend(paths(entry, true)),
            Write::Stage(entry) | Write::Discard(entry) => args.extend(paths(entry, false)),
            Write::StageAll
            | Write::UnstageAll
            | Write::Commit { .. }
            | Write::Git(_)
            | Write::Remote(_)
            | Write::Stash { .. }
            | Write::Move { .. }
            | Write::Clean { .. }
            | Write::Save { .. }
            | Write::Config { .. }
            | Write::Patch { .. } => {}
        }
        args
    }
}

fn strs(args: &[String]) -> Vec<&str> {
    args.iter().map(String::as_str).collect()
}

pub(crate) fn run(
    git: &Git,
    root: &std::path::Path,
    write: &Write,
    args: &[String],
) -> io::Result<()> {
    match write {
        Write::Save { path, text } => {
            let file = root.join(path);
            // A link here now would carry the text elsewhere.
            if std::fs::symlink_metadata(&file).is_ok_and(|meta| meta.file_type().is_symlink()) {
                return Err(io::Error::other(format!(
                    "{path} is now a link; nothing was saved"
                )));
            }
            std::fs::write(&file, text)
        }
        // The message goes in on stdin, never through a shell.
        Write::Commit { message, .. } => git
            .run_input(root, &strs(args), message.as_bytes())
            .map(drop),
        Write::Patch {
            patch, check, seen, ..
        } => {
            // A patch from an older diff is refused.
            if git.run(root, &strs(check))? != **seen {
                return Err(io::Error::other(
                    "the file changed since it was shown; nothing was staged",
                ));
            }
            git.run_input(root, &strs(args), patch.as_bytes()).map(drop)
        }
        Write::Stash { at, oid, .. } => {
            // Another stash since shifts every number.
            let now = git.run_or_one(
                root,
                &["rev-parse", "-q", "--verify", &format!("stash@{{{at}}}")],
            )?;
            if String::from_utf8_lossy(&now).trim() != oid {
                return Err(io::Error::other(
                    "the stash list changed since it was shown; nothing was done",
                ));
            }
            git.run(root, &strs(args)).map(drop)
        }
        Write::Move { branch, head, .. } => {
            if recover::head_now(git, root)? != (branch.clone(), head.clone()) {
                return Err(io::Error::other(
                    "HEAD moved since the preview; nothing was done",
                ));
            }
            recover::nothing_staged(git, root)?;
            git.run(root, &strs(args)).map(drop)
        }
        Write::Clean { files, dirs, .. } => {
            for file in files {
                as_shown(root, file, false)?;
            }
            for dir in dirs {
                as_shown(root, dir, true)?;
            }
            if !files.is_empty() {
                git.run(root, &strs(args))?;
            }
            // A folder that gained a file since stays.
            for dir in dirs {
                match std::fs::remove_dir(root.join(dir)) {
                    Err(error) if error.kind() == io::ErrorKind::DirectoryNotEmpty => {
                        log::info!("tree: {dir} kept; it is not empty");
                    }
                    Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
                    _ => {}
                }
            }
            Ok(())
        }
        Write::Remote(_) => git.start(root, &strs(args))?.follow(|_| {}).map(drop),
        _ => git.run(root, &strs(args)).map(drop),
    }
}

/// A shown path: same kind, reached through no link.
fn as_shown(root: &std::path::Path, path: &str, folder: bool) -> io::Result<()> {
    let changed = || {
        io::Error::other(format!(
            "{path} changed since the preview; nothing was removed"
        ))
    };
    let mut at = root.to_path_buf();
    let parts: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    for (ix, part) in parts.iter().enumerate() {
        at.push(part);
        // Gone since the preview is a change; others name paths.
        let kind = match std::fs::symlink_metadata(&at) {
            Ok(meta) => meta.file_type(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(changed()),
            Err(error) => {
                return Err(io::Error::new(
                    error.kind(),
                    format!("{}: {error}", at.display()),
                ));
            }
        };
        let last = ix + 1 == parts.len();
        if (!last && !kind.is_dir()) || (last && kind.is_dir() != folder) {
            return Err(changed());
        }
    }
    Ok(())
}

/// Git found another process's lock file in place.
fn locked(said: &str) -> bool {
    said.contains("File exists") && (said.contains(".lock") || said.contains("could not lock"))
}

impl Tree {
    /// A press on a file in the changes list.
    pub fn file_action(
        &mut self,
        path: &SharedString,
        section: ChangeSection,
        action: ChangeAction,
        cx: &mut Context<Self>,
    ) {
        let Some(opened) = self.active() else { return };
        let id = opened.id;
        let Some(Ok(status)) = &opened.status else {
            return;
        };
        let Some(entry) = status
            .entries
            .iter()
            .find(|entry| entry.path == path.as_ref())
            .cloned()
        else {
            return log::warn!("tree: {path} is no longer listed");
        };
        match action {
            ChangeAction::Open if section == ChangeSection::Ignored => {
                self.open_file(&entry.path, cx)
            }
            ChangeAction::Open => self.show(&entry.path, section, cx),
            ChangeAction::Stage => self.write(id, Write::Stage(entry), cx),
            ChangeAction::Unstage => self.write(id, Write::Unstage(entry), cx),
            // Each discard asks; one yes is not a standing one.
            ChangeAction::Discard => {
                self.ask = Some(Ask::Discard { repo: id, entry });
                cx.notify();
            }
        }
    }

    /// Opens a repository file with its own app.
    pub fn open_file(&self, path: &str, cx: &mut Context<Self>) {
        let Some(opened) = self.active() else { return };
        let file = opened.repo.root.join(path);
        if file.exists() {
            cx.open_with_system(&file);
        } else {
            let words = text(cx, "tree.gone", &[("path", path)]);
            self.toast(words, Severity::Warning, cx);
        }
    }

    /// Queues `write` behind the repository's other writes.
    pub fn write(&mut self, id: u64, write: Write, cx: &mut Context<Self>) {
        let Some(opened) = self.opened_mut(id) else {
            return;
        };
        opened.writes.push_back(write);
        self.next_write(id, cx);
    }

    fn next_write(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(git) = self.git.as_ref().ok().cloned() else {
            return;
        };
        let Some(opened) = self.opened_mut(id).filter(|opened| !opened.writing) else {
            return;
        };
        let Some(write) = opened.writes.pop_front() else {
            return;
        };
        opened.writing = true;
        if let Write::Remote(args) = write {
            return self.start_remote(id, args, cx);
        }
        let root = opened.repo.root.clone();
        let task = cx.background_spawn(async move {
            // Asked now, not cached: a commit may just have landed.
            let done = git.born(&root).and_then(|born| {
                let args = write.args(born);
                log::info!("tree: {:?} in {}", args.first(), root.display());
                run(&git, &root, &write, &args)
            });
            let stopped = match (&write, &done) {
                (Write::Git(args), Err(error)) => {
                    ops::stopped(&git, &root, args, &error.to_string())
                }
                _ => false,
            };
            (write, done, stopped)
        });
        cx.spawn(async move |this, cx| {
            let (write, done, stopped) = task.await;
            this.update(cx, |tree, cx| tree.written(id, &write, done, stopped, cx))
                .ok();
        })
        .detach();
    }

    pub(crate) fn written(
        &mut self,
        id: u64,
        write: &Write,
        done: io::Result<()>,
        stopped: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(opened) = self.opened_mut(id) else {
            return;
        };
        opened.writing = false;
        if done.is_err() {
            // Later writes counted on this one.
            opened.writes.clear();
        }
        match (write, done) {
            // A held lock waits for a retry; Ely deletes none.
            (_, Err(error)) if locked(&error.to_string()) => {
                self.ask = Some(Ask::Locked {
                    repo: id,
                    write: write.clone(),
                    detail: error.to_string().into(),
                });
                cx.notify();
            }
            (Write::Commit { message, .. }, Ok(())) => self.committed(id, message, cx),
            // Hooks and signing speak at length; show all of it.
            (Write::Commit { .. }, Err(error)) => {
                self.ask = Some(Ask::Failed {
                    title: text(cx, "tree.commit.failed", &[]),
                    detail: error.to_string().into(),
                });
                cx.notify();
            }
            (Write::Config { .. }, Ok(())) => self.read_me(id, cx),
            (Write::Save { path, text }, Ok(())) => self.merge_saved(id, path, text, cx),
            // A file write, not Git: the result did not save.
            (Write::Save { .. }, Err(error)) => {
                self.failed("tree.error.save", &error.to_string(), cx)
            }
            // A conflict stop is no failure; Git says which.
            (
                Write::Git(_)
                | Write::Remote(_)
                | Write::Stash { .. }
                | Write::Move { .. }
                | Write::Clean { .. },
                Err(error),
            ) => self.git_failed(stopped, error.to_string(), cx),
            (_, Ok(())) => {}
            (_, Err(error)) => self.failed("tree.error.write", &error.to_string(), cx),
        }
        self.read(id, cx);
        self.next_write(id, cx);
    }
}

#[cfg(test)]
mod tests;
