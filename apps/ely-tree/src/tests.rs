//! Repositories, status and writes against the bundled Git (C13.1).

use std::path::{Path, PathBuf};

use ely_gpui_component::{git::Changed, lists::GitStatus};

use crate::{
    changes::sections,
    git::Git,
    repo,
    status::{self, State},
    work::Write,
};

mod boxes;
mod branching;
mod diffs;
mod flows;
mod history;
mod place_flows;
mod places;
mod cleaning;
mod narrow;
mod recovery;
mod window;

pub(crate) fn git() -> Git {
    static ALONE: std::sync::Once = std::sync::Once::new();
    // Tests read no machine Git settings.
    ALONE.call_once(|| {
        let none = std::env::temp_dir().join("ely-tree-tests-no-gitconfig");
        // SAFETY: once, before any test runs Git.
        unsafe {
            std::env::set_var("GIT_CONFIG_NOSYSTEM", "1");
            std::env::set_var("GIT_CONFIG_GLOBAL", none);
        }
    });
    Git::find().expect("the bundled Git; run scripts/build-git.sh")
}

/// Runs Git with a test identity and no signing.
pub(crate) fn sh(dir: &Path, args: &[&str]) -> String {
    let mut all = vec![
        "-c",
        "user.name=Ely",
        "-c",
        "user.email=ely@example.com",
        "-c",
        "commit.gpgsign=false",
        "-c",
        "tag.gpgSign=false",
    ];
    all.extend(args);
    String::from_utf8(git().run(dir, &all).expect("git runs")).expect("utf-8")
}

/// A repository with one commit of `a.txt`.
pub(crate) fn committed(dir: &Path) -> PathBuf {
    let root = dir.join("work");
    repo::init(&git(), &root).expect("init");
    sh(&root, &["config", "user.name", "Ely"]);
    sh(&root, &["config", "user.email", "ely@example.com"]);
    std::fs::write(root.join("a.txt"), "one\ntwo\n").expect("writes");
    sh(&root, &["add", "a.txt"]);
    sh(&root, &["commit", "-q", "-m", "first"]);
    root
}

type Listed<'a> = (&'a str, GitStatus, Option<(usize, usize)>);

fn paths(files: &[Changed]) -> Vec<Listed<'_>> {
    files
        .iter()
        .map(|file| (file.path.as_ref(), file.status, file.lines))
        .collect()
}

#[test]
fn a_repository_opens_from_inside_and_a_bare_one_does_not() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::create_dir(root.join("deep")).expect("a folder");
    let opened = repo::open(&git(), &root.join("deep")).expect("opens");
    assert_eq!(
        opened.root,
        dunce::canonicalize(&root).expect("real"),
        "the real top"
    );
    assert!(opened.git_dir.ends_with(".git"));
    let bare = dir.path().join("bare.git");
    sh(
        dir.path(),
        &["init", "-q", "--bare", bare.to_str().expect("utf-8")],
    );
    let refused = repo::open(&git(), &bare).expect_err("bare");
    assert_eq!(refused.kind(), std::io::ErrorKind::Unsupported);
}

#[test]
fn the_work_tree_reads_into_five_sections() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("b.txt"), "b\n").expect("writes");
    std::fs::write(root.join("clash.txt"), "base\n").expect("writes");
    sh(&root, &["add", "."]);
    sh(&root, &["commit", "-q", "-m", "second"]);
    let main = sh(&root, &["branch", "--show-current"]).trim().to_string();
    sh(&root, &["checkout", "-q", "-b", "side"]);
    std::fs::write(root.join("clash.txt"), "side\n").expect("writes");
    sh(&root, &["commit", "-q", "-am", "side"]);
    sh(&root, &["checkout", "-q", &main]);
    std::fs::write(root.join("clash.txt"), "main\n").expect("writes");
    sh(&root, &["commit", "-q", "-am", "main"]);
    assert!(
        git().run(&root, &["merge", "-q", "side"]).is_err(),
        "a conflict"
    );
    std::fs::write(root.join("a.txt"), "one\ntwo\nthree\n").expect("writes");
    sh(&root, &["mv", "b.txt", "c.txt"]);
    std::fs::write(root.join("[x].txt"), "\0binary").expect("writes");
    std::fs::write(root.join(".gitignore"), "build/\n").expect("writes");
    std::fs::create_dir(root.join("build")).expect("a folder");
    std::fs::write(root.join("build/out"), "x").expect("writes");
    let read = status::read(&git(), &root).expect("reads");
    assert_eq!(read.head.branch.as_deref(), Some(main.as_str()));
    let [conflicted, staged, unstaged, untracked, ignored] = sections(&read);
    assert_eq!(
        paths(&conflicted),
        [("clash.txt", GitStatus::Conflicted, None)]
    );
    assert_eq!(
        paths(&staged),
        [("c.txt", GitStatus::Renamed, Some((0, 0)))]
    );
    assert_eq!(
        paths(&unstaged),
        [("a.txt", GitStatus::Modified, Some((1, 0)))]
    );
    assert_eq!(
        paths(&untracked),
        [
            (".gitignore", GitStatus::Untracked, None),
            ("[x].txt", GitStatus::Untracked, None)
        ]
    );
    assert_eq!(
        paths(&ignored),
        [("build/", GitStatus::Untracked, None)],
        "a folder is one entry"
    );
    assert_eq!(
        read.entries
            .iter()
            .find(|e| e.path == "c.txt")
            .and_then(|e| e.from.as_deref()),
        Some("b.txt")
    );
}

/// Runs one write as the app does.
fn write(root: &Path, write: Write) {
    let born = status::read(&git(), root)
        .expect("reads")
        .head
        .oid
        .is_some();
    let args = write.args(born);
    git()
        .run(root, &args.iter().map(String::as_str).collect::<Vec<_>>())
        .expect("the write runs");
}

pub(crate) fn entry(root: &Path, path: &str) -> status::Entry {
    let read = status::read(&git(), root).expect("reads");
    read.entries
        .into_iter()
        .find(|entry| entry.path == path)
        .expect("listed")
}

#[test]
fn writes_stage_unstage_and_discard_one_literal_path() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("[x].txt"), "star\n").expect("writes");
    std::fs::write(root.join("x.txt"), "x\n").expect("writes");
    write(&root, Write::Stage(entry(&root, "[x].txt")));
    let staged = |root: &Path| {
        status::read(&git(), root)
            .expect("reads")
            .staged
            .into_keys()
            .collect::<Vec<_>>()
    };
    assert_eq!(staged(&root), ["[x].txt"], "the brackets are no pattern");
    write(&root, Write::Unstage(entry(&root, "[x].txt")));
    assert!(staged(&root).is_empty());
    std::fs::write(root.join("a.txt"), "changed\n").expect("writes");
    write(&root, Write::Discard(entry(&root, "a.txt")));
    assert_eq!(
        std::fs::read_to_string(root.join("a.txt")).expect("reads"),
        "one\ntwo\n"
    );
    write(&root, Write::Discard(entry(&root, "[x].txt")));
    assert!(
        !root.join("[x].txt").exists() && root.join("x.txt").exists(),
        "only that file goes"
    );
    std::fs::write(root.join("a.txt"), "again\n").expect("writes");
    write(&root, Write::StageAll);
    assert_eq!(staged(&root), ["a.txt"], "tracked changes only");
    write(&root, Write::UnstageAll);
    assert!(staged(&root).is_empty());
}

#[test]
fn before_the_first_commit_unstaging_still_works() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path().join("new");
    repo::init(&git(), &root).expect("init");
    std::fs::write(root.join("a.txt"), "a\n").expect("writes");
    write(&root, Write::Stage(entry(&root, "a.txt")));
    let read = status::read(&git(), &root).expect("reads");
    assert_eq!(read.head.oid, None);
    assert_eq!(read.staged.get("a.txt"), Some(&Some((1, 0))));
    write(&root, Write::Unstage(entry(&root, "a.txt")));
    assert_eq!(entry(&root, "a.txt").state, State::Untracked);
    write(&root, Write::Stage(entry(&root, "a.txt")));
    std::fs::write(root.join("a.txt"), "a\nedited\n").expect("writes");
    write(&root, Write::Unstage(entry(&root, "a.txt")));
    assert_eq!(
        entry(&root, "a.txt").state,
        State::Untracked,
        "staged and edited"
    );
    write(&root, Write::Stage(entry(&root, "a.txt")));
    std::fs::write(root.join("a.txt"), "a\nagain\n").expect("writes");
    write(&root, Write::UnstageAll);
    assert_eq!(entry(&root, "a.txt").state, State::Untracked);
    assert_eq!(
        std::fs::read_to_string(root.join("a.txt")).expect("reads"),
        "a\nagain\n"
    );
}

#[test]
fn a_clone_reports_progress_and_refuses_a_full_folder() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let source = committed(dir.path());
    let into = dir.path().join("copy");
    let mut seen = Vec::new();
    let url = source.to_str().expect("utf-8");
    git()
        .clone_repo(url, &into)
        .expect("starts")
        .follow(|line| seen.push(line.to_string()))
        .expect("clones");
    assert!(into.join("a.txt").is_file());
    assert!(
        seen.iter().any(|line| line.starts_with("Cloning into")),
        "{seen:?}"
    );
    let full = git().clone_repo(url, &into).err().expect("refused");
    assert_eq!(full.kind(), std::io::ErrorKind::AlreadyExists);
}

#[cfg(unix)]
#[test]
fn a_stopped_clone_ends_and_leaves_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    // A server that never answers keeps Git waiting.
    let silent = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
    let url = format!("git://{}/nothing", silent.local_addr().expect("an address"));
    let into = dir.path().join("never");
    let running = git().clone_repo(&url, &into).expect("starts");
    let stop = running.stop();
    let (said, heard) = std::sync::mpsc::channel();
    let followed = std::thread::spawn(move || {
        running.follow(|line| {
            let _ = said.send(line.to_string());
        })
    });
    let _held = silent.accept().expect("Git connects");
    let first = heard
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("a line while waiting");
    assert!(first.starts_with("Cloning into"), "{first}");
    stop.now().expect("stops");
    assert!(
        followed.join().expect("joins").is_err(),
        "a stopped clone fails"
    );
    assert!(!into.exists(), "Git removed what it began");
    stop.now().expect("stopping an ended clone is a no-op");
}

#[test]
fn a_staged_rename_edited_again_stages_by_its_new_path() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    sh(&root, &["mv", "a.txt", "c.txt"]);
    std::fs::write(root.join("c.txt"), "one\ntwo\nmore\n").expect("writes");
    write(&root, Write::Stage(entry(&root, "c.txt")));
    let read = status::read(&git(), &root).expect("reads");
    assert!(read.unstaged.is_empty(), "the edit is staged");
    write(&root, Write::Unstage(entry(&root, "c.txt")));
    let read = status::read(&git(), &root).expect("reads");
    assert!(read.staged.is_empty(), "both paths go back");
}

#[test]
fn a_gone_upstream_has_no_counts() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let source = committed(dir.path());
    let into = dir.path().join("copy");
    let url = source.to_str().expect("utf-8");
    git()
        .clone_repo(url, &into)
        .expect("starts")
        .follow(|_| {})
        .expect("clones");
    let head = status::read(&git(), &into).expect("reads").head;
    assert_eq!(head.counts, Some((0, 0)));
    let upstream = head.upstream.expect("an upstream");
    sh(
        &into,
        &["update-ref", "-d", &format!("refs/remotes/{upstream}")],
    );
    let head = status::read(&git(), &into).expect("reads").head;
    assert_eq!(
        (head.upstream.as_deref(), head.counts),
        (Some(upstream.as_str()), None)
    );
}

#[cfg(unix)]
#[test]
fn a_linked_file_keeps_its_own_name() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dunce::canonicalize(dir.path()).expect("real");
    std::fs::create_dir(root.join("docs")).expect("a folder");
    std::fs::write(root.join("real.md"), "a").expect("a file");
    let outside = tempfile::tempdir().expect("another dir");
    std::fs::write(outside.path().join("far.md"), "b").expect("a far file");
    std::os::unix::fs::symlink("../real.md", root.join("docs/link.md")).expect("a link");
    std::os::unix::fs::symlink(outside.path().join("far.md"), root.join("far.md")).expect("an outside link");
    // Git tracks the link, not what it points at.
    assert_eq!(crate::tree::inside(&root, &root.join("docs/link.md")).expect("inside"), "docs/link.md");
    assert_eq!(crate::tree::inside(&root, &root.join("far.md")).expect("inside"), "far.md");
    assert!(crate::tree::inside(&root, &outside.path().join("far.md")).is_err());
}
