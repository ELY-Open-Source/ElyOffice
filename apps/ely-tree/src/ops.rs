//! A merge, rebase, cherry-pick or revert under way (TREE-08, §13.3).

use std::{io, path::Path};

use crate::git::Git;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Merge,
    Rebase,
    CherryPick,
    Revert,
    /// `git am`, applying mailed patches.
    Am,
}

impl Op {
    /// The Git command that continues, aborts or skips it.
    pub fn verb(self) -> &'static str {
        match self {
            Op::Merge => "merge",
            Op::Rebase => "rebase",
            Op::CherryPick => "cherry-pick",
            Op::Revert => "revert",
            Op::Am => "am",
        }
    }

    /// A merge has no step to skip.
    pub fn skips(self) -> bool {
        self != Op::Merge
    }
}

/// The operation, and what its two sides stand for now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Doing {
    pub op: Op,
    /// Git's "ours": what HEAD holds as the operation stands.
    pub ours: String,
    /// Git's "theirs": what is being brought in.
    pub theirs: String,
}

/// Stopped on a conflict: Git said so and left some.
pub fn stopped(git: &Git, root: &Path, args: &[String], said: &str) -> bool {
    let operation = matches!(
        args.first().map(String::as_str),
        Some("merge" | "rebase" | "cherry-pick" | "revert" | "pull" | "am")
    );
    if !operation || !said.lines().any(|line| line.starts_with("CONFLICT (")) {
        return false;
    }
    match git.run(root, &["ls-files", "--unmerged", "-z"]) {
        Ok(unmerged) => !unmerged.is_empty(),
        Err(error) => {
            log::error!("tree: unmerged entries unread: {error}");
            false
        }
    }
}

/// A commit as `abc1234 subject`.
fn describe(git: &Git, root: &Path, rev: &str) -> io::Result<String> {
    let said = git.run(root, &["log", "-1", "--format=%h %s", rev, "--"])?;
    Ok(String::from_utf8_lossy(&said).trim().to_string())
}

/// The branch HEAD is on, or its commit when detached.
fn head(git: &Git, root: &Path) -> io::Result<String> {
    let branch = git.run_or_one(root, &["symbolic-ref", "--short", "-q", "HEAD"])?;
    match String::from_utf8_lossy(&branch).trim() {
        "" => describe(git, root, "HEAD"),
        branch => Ok(branch.to_string()),
    }
}

/// What is under way in the work tree of `git_dir`.
pub fn doing(git: &Git, root: &Path, git_dir: &Path) -> io::Result<Option<Doing>> {
    let has = |name: &str| git_dir.join(name).exists();
    let read = |name: &str| {
        std::fs::read_to_string(git_dir.join(name)).map(|text| text.trim().to_string())
    };
    // `git am` keeps rebase-apply too, without an onto.
    let doing = if has("rebase-apply/applying") {
        Doing {
            op: Op::Am,
            ours: head(git, root)?,
            theirs: format!(
                "{}/{}",
                read("rebase-apply/next")?,
                read("rebase-apply/last")?
            ),
        }
    } else if has("rebase-merge") || has("rebase-apply") {
        let dir = if has("rebase-merge") {
            "rebase-merge"
        } else {
            "rebase-apply"
        };
        // In a rebase, ours is the base; theirs, the replay.
        let onto = describe(git, root, &read(&format!("{dir}/onto"))?)?;
        let theirs = if has("REBASE_HEAD") {
            describe(git, root, "REBASE_HEAD")?
        } else {
            String::new()
        };
        Doing {
            op: Op::Rebase,
            ours: onto,
            theirs,
        }
    } else if has("MERGE_HEAD") {
        Doing {
            op: Op::Merge,
            ours: head(git, root)?,
            theirs: describe(git, root, "MERGE_HEAD")?,
        }
    } else if has("CHERRY_PICK_HEAD") {
        Doing {
            op: Op::CherryPick,
            ours: head(git, root)?,
            theirs: describe(git, root, "CHERRY_PICK_HEAD")?,
        }
    } else if has("REVERT_HEAD") {
        Doing {
            op: Op::Revert,
            ours: head(git, root)?,
            theirs: describe(git, root, "REVERT_HEAD")?,
        }
    } else {
        return Ok(None);
    };
    Ok(Some(doing))
}
