//! Repositories: open, start, clone, and the recent list (TREE-01).

use std::{
    io,
    path::{Path, PathBuf},
};

use ely_ipc::Worded;

use crate::{git::Git, running::Running};

/// A work tree and its Git directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repo {
    /// The real top of the work tree, links resolved.
    pub root: PathBuf,
    pub git_dir: PathBuf,
    /// Shared refs and objects; a linked worktree's lie elsewhere.
    pub common_dir: PathBuf,
}

fn text(bytes: Vec<u8>) -> io::Result<String> {
    String::from_utf8(bytes)
        .map(|s| s.trim_end_matches('\n').to_string())
        .map_err(io::Error::other)
}

/// The repository holding `path`; bare ones are refused.
pub fn open(git: &Git, path: &Path) -> io::Result<Repo> {
    if text(git.run(path, &["rev-parse", "--is-bare-repository"])?)? == "true" {
        let path = path.display().to_string();
        return Err(Worded::error(io::ErrorKind::Unsupported, "tree.error.bare", &[("path", &path)]));
    }
    let root = text(git.run(path, &["rev-parse", "--show-toplevel"])?)?;
    let git_dir = text(git.run(path, &["rev-parse", "--absolute-git-dir"])?)?;
    let common = ["rev-parse", "--path-format=absolute", "--git-common-dir"];
    let common_dir = text(git.run(path, &common)?)?;
    let repo = Repo {
        root: dunce::canonicalize(root)?,
        git_dir: dunce::canonicalize(git_dir)?,
        common_dir: dunce::canonicalize(common_dir)?,
    };
    log::info!("tree: opened {}", repo.root.display());
    Ok(repo)
}

/// A new repository in `dir`, then opened.
pub fn init(git: &Git, dir: &Path) -> io::Result<Repo> {
    std::fs::create_dir_all(dir)?;
    git.run(dir, &["init", "--quiet"])?;
    open(git, dir)
}

impl Git {
    /// Starts `git clone url into`; never asks on a terminal.
    pub fn clone_repo(&self, url: &str, into: &Path) -> io::Result<Running> {
        if into.exists() && std::fs::read_dir(into)?.next().is_some() {
            let into = into.display().to_string();
            return Err(Worded::error(io::ErrorKind::AlreadyExists, "tree.error.not_empty", &[("path", &into)]));
        }
        let parent = into
            .parent()
            .ok_or_else(|| io::Error::other("a clone needs a parent folder"))?;
        let into_text = into
            .to_str()
            .ok_or_else(|| io::Error::other("a path that is not UTF-8"))?;
        log::info!("tree: clone into {}", into.display());
        self.start(parent, &["clone", "--progress", "--", url, into_text])
    }
}
