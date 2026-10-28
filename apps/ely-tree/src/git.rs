//! Git as a program: argument arrays, never a shell (§13.1).

use std::{
    io::{self, Write as _},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

/// A Git binary whose version was checked.
#[derive(Clone, Debug)]
pub struct Git {
    pub path: PathBuf,
}

impl Git {
    /// `ELY_GIT` or the setting picks Git; else Ely's own.
    pub fn find() -> io::Result<Self> {
        let (path, bundled) = ely_platform::git::locate()?;
        if !path.is_file() {
            let hint = if bundled {
                "run scripts/build-git.sh"
            } else {
                "check ELY_GIT or the Git setting"
            };
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no Git at {} ({hint})", path.display()),
            ));
        }
        let version = match bundled {
            true => bundled_version(&path)?,
            false => ely_platform::git::system_version(&path)?,
        };
        log::info!(
            "tree: {version} at {} ({})",
            path.display(),
            if bundled { "bundled" } else { "system" }
        );
        Ok(Git { path })
    }

    /// Runs Git in `dir`; failure carries Git's words.
    pub fn run(&self, dir: &Path, args: &[&str]) -> io::Result<Vec<u8>> {
        let output = self.command(dir, args).output()?;
        checked(args, output)
    }

    /// Runs Git with `input` on its standard input.
    pub fn run_input(&self, dir: &Path, args: &[&str], input: &[u8]) -> io::Result<Vec<u8>> {
        let mut child = self
            .command(dir, args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("no stdin"))?;
        let input = input.to_vec();
        // Writing apart from reading cannot fill both pipes.
        let writer = std::thread::spawn(move || stdin.write_all(&input));
        let output = child.wait_with_output()?;
        writer
            .join()
            .map_err(|_| io::Error::other("the stdin writer panicked"))??;
        checked(args, output)
    }

    /// HEAD names a commit; a new repository has none yet.
    pub fn born(&self, dir: &Path) -> io::Result<bool> {
        if !self.run_or_one(dir, &["rev-parse", "--verify", "-q", "HEAD"])?.is_empty() {
            return Ok(true);
        }
        // Unborn: HEAD names a branch with no ref at all.
        let branch = String::from_utf8(self.run(dir, &["symbolic-ref", "-q", "HEAD"])?)
            .map_err(|_| io::Error::other("HEAD names a ref that is not UTF-8"))?;
        // Only Git's line end goes; a name keeps every byte.
        let branch = branch.strip_suffix('\n').unwrap_or(&branch).to_owned();
        let found = self.command(dir, &["for-each-ref", "--format=%(refname)", &branch]).output()?;
        // The pattern also matches refs below it.
        let held = found.stdout.split(|&byte| byte == b'\n').any(|name| name == branch.as_bytes());
        let unborn = branch.starts_with("refs/heads/") && found.status.success();
        if unborn && !held && found.stderr.is_empty() {
            return Ok(false);
        }
        let mut why = format!("HEAD names {branch}: no commit, and no branch yet to begin");
        let said = String::from_utf8_lossy(&found.stderr);
        if !said.trim().is_empty() {
            why = format!("{why}; Git says {}", said.trim());
        }
        Err(io::Error::other(why))
    }

    /// Exit 1 is an answer: files differ, key unset.
    pub fn run_or_one(&self, dir: &Path, args: &[&str]) -> io::Result<Vec<u8>> {
        let output = self.command(dir, args).output()?;
        match output.status.code() {
            Some(1) => Ok(output.stdout),
            _ => checked(args, output),
        }
    }

    /// `merge-file` exits with its conflict count; past 127 fails.
    pub fn run_counted(&self, dir: &Path, args: &[&str]) -> io::Result<Vec<u8>> {
        let output = self.command(dir, args).output()?;
        match output.status.code() {
            Some(0..=127) => Ok(output.stdout),
            _ => checked(args, output),
        }
    }

    pub(crate) fn command(&self, dir: &Path, args: &[&str]) -> Command {
        ely_platform::git::command(&self.path, dir, args)
    }
}

fn checked(args: &[&str], output: Output) -> io::Result<Vec<u8>> {
    // The verb follows any `-c name=value` pairs.
    let verb = args
        .iter()
        .copied()
        .find(|arg| !arg.starts_with('-') && !arg.contains('='))
        .unwrap_or("");
    match output.status.success() {
        true => {
            log::debug!("tree: git {verb} done");
            Ok(output.stdout)
        }
        false => {
            // Conflicts go to stdout, hooks to stderr.
            let said: Vec<String> = [&output.stdout, &output.stderr]
                .into_iter()
                .map(|bytes| String::from_utf8_lossy(bytes).trim().to_string())
                .filter(|said| !said.is_empty())
                .collect();
            log::warn!("tree: git {verb} failed, {}", output.status);
            Err(io::Error::other(format!(
                "git {verb} ({}):\n{}",
                output.status,
                said.join("\n")
            )))
        }
    }
}

/// Ely's own Git, whatever version it was pinned at.
fn bundled_version(path: &Path) -> io::Result<String> {
    ely_platform::git::version_of(path).map(|(version, _)| version)
}
