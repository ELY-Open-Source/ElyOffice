//! Remotes, and what a forced push would overwrite (TREE-11, §13.3).

use std::{io, path::Path};

use ely_ipc::Worded;

use crate::git::Git;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Remote {
    pub name: String,
    pub urls: Vec<String>,
    /// Push URLs, when set apart from fetch ones.
    pub push: Vec<String>,
}

fn bad(what: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("an unread remote {what:?}"),
    )
}

fn words(bytes: Vec<u8>) -> io::Result<String> {
    String::from_utf8(bytes)
        .map(|text| text.trim_end_matches('\n').to_string())
        .map_err(|_| bad("not UTF-8"))
}

/// Each remote Git knows, with its URLs.
pub fn list(git: &Git, root: &Path) -> io::Result<Vec<Remote>> {
    let mut remotes: Vec<Remote> = words(git.run(root, &["remote"])?)?
        .lines()
        .map(|name| Remote {
            name: name.to_string(),
            urls: Vec::new(),
            push: Vec::new(),
        })
        .collect();
    let pattern = r"^remote\..*\.(url|pushurl)$";
    let said = words(git.run_or_one(root, &["config", "-z", "--get-regexp", pattern])?)?;
    // `-z`: key, newline, value, NUL.
    for entry in said.split('\0').filter(|entry| !entry.is_empty()) {
        let (key, value) = entry.split_once('\n').ok_or_else(|| bad(entry))?;
        let (name, field) = key
            .strip_prefix("remote.")
            .and_then(|key| key.rsplit_once('.'))
            .ok_or_else(|| bad(key))?;
        let Some(remote) = remotes.iter_mut().find(|remote| remote.name == name) else {
            return Err(bad(name));
        };
        match field {
            "url" => remote.urls.push(value.to_string()),
            _ => remote.push.push(value.to_string()),
        }
    }
    Ok(remotes)
}

/// A forced push: where, and what it would overwrite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Force {
    pub branch: String,
    pub remote: String,
    /// The branch on the remote: `refs/heads/main`.
    pub target: String,
    /// What the remote held when last fetched.
    pub lease: String,
    /// The commit shown; pushed whatever the branch is by then.
    pub source: String,
    /// Commits there and not here: `abc1234 subject`.
    pub lost: Vec<String>,
}

/// The current branch's forced push to its upstream.
pub fn force(git: &Git, root: &Path) -> io::Result<Force> {
    let branch = words(git.run_or_one(root, &["symbolic-ref", "--short", "-q", "HEAD"])?)?;
    if branch.is_empty() {
        return Err(Worded::error(io::ErrorKind::Other, "tree.error.detached", &[]));
    }
    let setting = |key: &str| -> io::Result<String> {
        words(git.run_or_one(
            root,
            &["config", "--get", &format!("branch.{branch}.{key}")],
        )?)
    };
    let (remote, target) = (setting("remote")?, setting("merge")?);
    if remote.is_empty() || target.is_empty() {
        return Err(Worded::error(io::ErrorKind::Other, "tree.error.no_upstream", &[("branch", &branch)]));
    }
    let upstream = format!("{branch}@{{upstream}}");
    let lease = words(git.run(root, &["rev-parse", "--verify", &upstream])?)?;
    let source = words(git.run(root, &["rev-parse", "--verify", "HEAD"])?)?;
    let lost = words(git.run(
        root,
        &["log", "--format=%h %s", &lease, "--not", &source, "--"],
    )?)?;
    Ok(Force {
        lost: lost.lines().map(str::to_string).collect(),
        branch,
        remote,
        target,
        lease,
        source,
    })
}

impl Force {
    /// With a lease: refused if the remote moved since.
    pub fn args(&self) -> Vec<String> {
        vec![
            "push".into(),
            "--progress".into(),
            format!("--force-with-lease={}:{}", self.target, self.lease),
            "--".into(),
            self.remote.clone(),
            format!("{}:{}", self.source, self.target),
        ]
    }
}
