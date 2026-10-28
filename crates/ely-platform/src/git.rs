//! Ely's Git: where it is and how it runs (§13.1).

use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
};

use crate::AppId;

/// ElyTree's setting naming a system Git; empty is Ely's own.
pub const PROGRAM: &str = "git";

/// The oldest system Git counted as verified (user decision).
pub const LEAST: (u32, u32) = (2, 40);

/// Git inside a Git install: MinGit's on Windows.
#[cfg(windows)]
const GIT: &str = "cmd/git.exe";
#[cfg(not(windows))]
const GIT: &str = "bin/git";

/// `ELY_GIT`, ElyTree's setting, else Ely's own Git.
pub fn locate() -> io::Result<(PathBuf, bool)> {
    if let Some(path) = std::env::var_os("ELY_GIT") {
        return Ok((PathBuf::from(path), false));
    }
    let set = crate::settings::Store::app(AppId::Tree)?.read()?;
    match set.get(PROGRAM) {
        None => {}
        Some(serde_json::Value::String(path)) if path.is_empty() => {}
        Some(serde_json::Value::String(path)) => return Ok((PathBuf::from(path), false)),
        Some(other) => return Err(io::Error::new(io::ErrorKind::InvalidData, format!("settings: git cannot be {other}"))),
    }
    Ok((own(&std::env::current_exe()?)?, true))
}

/// `git version 2.56.0…` as (2, 56); None for anything else.
pub fn parse_version(text: &str) -> Option<(u32, u32)> {
    let number = text.trim().strip_prefix("git version ")?;
    let mut parts = number.split(|c: char| !c.is_ascii_digit());
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

/// What `git --version` says, once it succeeds.
pub fn version_of(path: &Path) -> io::Result<(String, (u32, u32))> {
    let output = ely_ipc::windowless(&mut Command::new(path)).arg("--version").output()?;
    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(io::Error::other(format!("{} --version failed ({}): {said}", path.display(), output.status)));
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let number = parse_version(&version).ok_or_else(|| io::Error::other(format!("{} is no Git: {version:?}", path.display())))?;
    Ok((version, number))
}

/// A system Git's version, if 2.40 or later.
pub fn system_version(path: &Path) -> io::Result<String> {
    let (version, number) = version_of(path)?;
    if number < LEAST {
        let message = format!("{version} at {} is older than {}.{}", path.display(), LEAST.0, LEAST.1);
        return Err(io::Error::new(io::ErrorKind::Unsupported, message));
    }
    Ok(version)
}

/// No Git beside the app nor beside ElyTree.
#[derive(Debug)]
pub struct NoGit;

impl std::fmt::Display for NoGit {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("Ely's Git is missing: ElyTree carries it")
    }
}

impl std::error::Error for NoGit {}

/// The app at `exe`'s own Git, else ElyTree's, else built.
fn own(exe: &Path) -> io::Result<PathBuf> {
    // ElyTree carries Git; the other apps use its copy.
    let trees = crate::apps::beside(exe, AppId::Tree).into_iter().chain(crate::apps::registered(AppId::Tree));
    let built = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../engines/git").join(GIT);
    find(exe, trees, built)
}

/// The first Git beside `exe`, beside a tree, or built.
fn find(exe: &Path, trees: impl IntoIterator<Item = PathBuf>, built: PathBuf) -> io::Result<PathBuf> {
    exe.parent()
        .map(Path::to_path_buf)
        .into_iter()
        .chain(trees.into_iter().filter_map(|tree| tree.parent().map(Path::to_path_buf)))
        .flat_map(|dir| [dir.join("git").join(GIT), dir.join("../Resources/git").join(GIT)])
        .chain([built])
        .find(|path| path.is_file())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, NoGit))
}

/// Git at `git` in `dir`, as Ely runs it.
pub fn command(git: &Path, dir: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(git);
    ely_ipc::windowless(&mut command)
        .args(args)
        .current_dir(dir)
        // Reads never take the index lock from an outside Git.
        .env("GIT_OPTIONAL_LOCKS", "0")
        // A path is a path, never a pattern.
        .env("GIT_LITERAL_PATHSPECS", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        // No terminal for an editor; Git's own message stands.
        .env("GIT_EDITOR", "true")
        .env("GIT_SEQUENCE_EDITOR", "true");
    command
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    #[test]
    fn a_failing_version_query_is_no_version() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("git");
        std::fs::write(&fake, "#!/bin/sh\necho 'git version 2.56.0'\necho 'fatal: version query failed' >&2\nexit 7\n").unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        let error = super::system_version(&fake).expect_err("a failed run");
        assert!(error.to_string().contains("version query failed"), "{error}");
        assert!(super::version_of(&fake).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_system_git_before_the_floor_is_refused() {
        let fake = |name: &str| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/git").join(name);
        let old = super::system_version(&fake("git-2.39")).expect_err("too old");
        assert_eq!(old.kind(), std::io::ErrorKind::Unsupported);
        assert!(old.to_string().contains("older than 2.40"), "{old}");
        assert_eq!(super::system_version(&fake("git-2.40")).expect("the floor"), "git version 2.40.0");
    }

    #[test]
    fn versions_read_as_major_and_minor() {
        assert_eq!(super::parse_version("git version 2.56.0\n"), Some((2, 56)));
        assert_eq!(super::parse_version("git version 2.54.0 (Apple Git-157)"), Some((2, 54)));
        assert_eq!(super::parse_version("hello"), None);
    }

    use super::{GIT, NoGit, find};
    use crate::{AppId, apps::beside};
    use std::path::Path;

    /// `find` with only temp folders: no installed or built Git.
    fn found(exe: &Path, root: &Path) -> std::io::Result<std::path::PathBuf> {
        find(exe, beside(exe, AppId::Tree), root.join("built").join(GIT))
    }

    fn same(found: std::path::PathBuf, git: &Path) {
        assert_eq!(std::fs::canonicalize(found).expect("found"), std::fs::canonicalize(git).expect("git"));
    }

    #[test]
    fn apps_beside_elytree_use_its_git() {
        let apps = tempfile::tempdir().expect("a temp dir");
        let mark = apps.path().join("ElyMark.app/Contents/MacOS");
        let git = apps.path().join("ElyTree.app/Contents/Resources/git").join(GIT);
        std::fs::create_dir_all(&mark).expect("ElyMark");
        std::fs::create_dir_all(apps.path().join("ElyTree.app/Contents/MacOS")).expect("ElyTree");
        let exe = mark.join("ely-mark");
        let missing = found(&exe, apps.path()).expect_err("no Git yet");
        assert!(missing.get_ref().is_some_and(|error| error.is::<NoGit>()), "{missing}");
        std::fs::create_dir_all(git.parent().expect("a folder")).expect("ElyTree's Git");
        std::fs::write(&git, "").expect("git");
        same(found(&exe, apps.path()).expect("ElyTree's Git"), &git);
    }

    #[test]
    fn a_windows_install_uses_elytrees_git() {
        let root = tempfile::tempdir().expect("a temp dir");
        let exe = root.path().join("Ely Office/ElyMark/ely-mark");
        let git = root.path().join("Ely Office/ElyTree/git").join(GIT);
        std::fs::create_dir_all(exe.parent().expect("a folder")).expect("ElyMark");
        assert!(found(&exe, root.path()).is_err(), "no Git yet");
        std::fs::create_dir_all(git.parent().expect("a folder")).expect("ElyTree's Git");
        std::fs::write(&git, "").expect("git");
        same(found(&exe, root.path()).expect("ElyTree's Git"), &git);
    }

    #[test]
    fn the_built_git_comes_last() {
        let root = tempfile::tempdir().expect("a temp dir");
        let exe = root.path().join("app/ely-mark");
        let built = root.path().join("built").join(GIT);
        std::fs::create_dir_all(built.parent().expect("a folder")).expect("built");
        std::fs::write(&built, "").expect("git");
        same(found(&exe, root.path()).expect("the built Git"), &built);
    }
}
