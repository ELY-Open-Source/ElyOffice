//! History, a commit's detail, blame and committing (C13.3).

use std::path::Path;

use super::{committed, git, sh};
use crate::{
    blame, commit,
    commits::{self, Find, Query},
    detail,
    shown::Found,
    work::{self, Write},
};

fn commit_all(root: &Path, message: &str) {
    sh(root, &["add", "-A"]);
    sh(root, &["commit", "-q", "-m", message]);
}

fn head(root: &Path) -> String {
    sh(root, &["rev-parse", "HEAD"]).trim().to_string()
}

#[test]
fn history_reads_branches_tags_merges_and_searches() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    let main = sh(&root, &["branch", "--show-current"]).trim().to_string();
    sh(&root, &["checkout", "-q", "-b", "side"]);
    std::fs::write(root.join("side.txt"), "side needle\n").expect("writes");
    commit_all(&root, "Side work");
    sh(&root, &["checkout", "-q", &main]);
    std::fs::write(root.join("main.txt"), "main\n").expect("writes");
    commit_all(&root, "Main work");
    sh(
        &root,
        &[
            "-c",
            "user.name=Bea",
            "merge",
            "-q",
            "--no-ff",
            "side",
            "-m",
            "Merge side",
        ],
    );
    sh(&root, &["tag", "v1"]);
    let all = commits::read(&git(), &root, 0, None).expect("reads");
    assert_eq!(all.len(), 4);
    assert_eq!(all[0].subject, "Merge side");
    assert_eq!(all[0].parents.len(), 2, "a merge has two parents");
    assert!(
        all[0].refs.contains(&main) && all[0].refs.contains(&"v1".to_string()),
        "{:?}",
        all[0].refs
    );
    assert_eq!(
        commits::read(&git(), &root, 3, None).expect("reads").len(),
        1,
        "a later page"
    );
    let find = |find, text: &str| {
        let query = Query {
            find,
            text: text.into(),
        };
        commits::read(&git(), &root, 0, Some(&query))
            .expect("searches")
            .into_iter()
            .map(|c| c.subject)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        find(Find::Message, "side WORK"),
        ["Side work"],
        "message, any case"
    );
    assert_eq!(find(Find::Content, "needle"), ["Side work"], "added text");
    assert_eq!(find(Find::Author, "nobody"), Vec::<String>::new());
    let first = all.last().expect("first").id.clone();
    assert_eq!(find(Find::Hash, &first[..8]), ["first"], "an id prefix");
    assert!(find(Find::Hash, "zzzz").is_empty());
}

#[test]
fn a_files_history_follows_its_renames() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("a.txt"), "one\nTWO\nthree\n").expect("writes");
    commit_all(&root, "two");
    sh(&root, &["mv", "a.txt", "b.txt"]);
    commit_all(&root, "move");
    let touches = commits::file(&git(), &root, "b.txt").expect("reads");
    let shape: Vec<_> = touches
        .iter()
        .map(|t| {
            (
                t.commit.subject.as_str(),
                t.path.as_str(),
                t.from.as_deref(),
                t.lines,
            )
        })
        .collect();
    assert_eq!(
        shape,
        [
            ("move", "b.txt", Some("a.txt"), Some((0, 0))),
            ("two", "a.txt", None, Some((2, 1))),
            ("first", "a.txt", None, Some((2, 0)))
        ]
    );
    let change = |at: usize| {
        let t = &touches[at];
        detail::change(
            &git(),
            &root,
            &t.commit.id,
            &t.commit.parents,
            &t.path,
            t.from.clone(),
        )
        .expect("reads")
    };
    assert!(matches!(change(1), Found::Text { .. }));
    assert!(
        matches!(change(2), Found::Text { old: None, .. }),
        "added in the first commit"
    );
}

#[test]
fn a_commit_details_its_people_files_and_diffs() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    let first = detail::read(&git(), &root, &head(&root)).expect("reads");
    assert!(first.parents.is_empty());
    assert_eq!(
        first
            .files
            .iter()
            .map(|f| (f.status, f.path.as_str()))
            .collect::<Vec<_>>(),
        [('A', "a.txt")]
    );
    assert!(matches!(
        detail::file(&git(), &root, &first.id, &first.parents, &first.files[0]).expect("reads"),
        Found::Text { old: None, .. }
    ));
    std::fs::write(root.join("new.txt"), "new\n").expect("writes");
    sh(&root, &["mv", "a.txt", "renamed.txt"]);
    sh(&root, &["add", "-A"]);
    sh(&root, &["commit", "-q", "-m", "Rename\n\nWith a body."]);
    let read = detail::read(&git(), &root, &head(&root)).expect("reads");
    assert_eq!(read.message, "Rename\n\nWith a body.");
    assert_eq!(
        (read.author.name.as_str(), read.author.email.as_str()),
        ("Ely", "ely@example.com")
    );
    assert_eq!(read.signed, 'N');
    let files: Vec<_> = read
        .files
        .iter()
        .map(|f| (f.status, f.from.as_deref(), f.path.as_str(), f.lines))
        .collect();
    assert_eq!(
        files,
        [
            ('A', None, "new.txt", Some((1, 0))),
            ('R', Some("a.txt"), "renamed.txt", Some((0, 0)))
        ]
    );
}

#[test]
fn blame_names_each_lines_commit_and_what_is_not_committed() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    std::fs::write(root.join("a.txt"), "one\ntwo\nthree\n").expect("writes");
    commit_all(&root, "third line");
    std::fs::write(root.join("a.txt"), "one\ntwo\nthree\nfour\n").expect("writes");
    let read = blame::read(&git(), &root, "a.txt").expect("reads");
    assert_eq!(read.code, "one\ntwo\nthree\nfour\n");
    let owner = |line: usize| &read.owners[read.lines[line]];
    assert_eq!(
        (owner(0).subject.as_str(), owner(2).subject.as_str()),
        ("first", "third line")
    );
    assert_eq!(owner(0).author, "Ely");
    assert!(owner(3).uncommitted() && !owner(0).uncommitted());
    assert_eq!(
        read.lines[0], read.lines[1],
        "one owner for both first lines"
    );
}

fn write(root: &Path, write: Write) -> std::io::Result<()> {
    work::run(&git(), root, &write, &write.args(true))
}

#[test]
fn commits_go_in_with_their_message_and_amend_replaces_the_last() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    sh(&root, &["config", "user.name", "Ely"]);
    sh(&root, &["config", "user.email", "ely@example.com"]);
    sh(&root, &["config", "commit.gpgsign", "false"]);
    std::fs::write(root.join("a.txt"), "changed\n").expect("writes");
    sh(&root, &["add", "a.txt"]);
    let message = commit::message("  Change a  ", "Line one.\n# kept, not a comment\n");
    write(
        &root,
        Write::Commit {
            message,
            amend: false,
        },
    )
    .expect("commits");
    assert_eq!(
        commit::last_message(&git(), &root).expect("reads"),
        "Change a\n\nLine one.\n# kept, not a comment"
    );
    write(
        &root,
        Write::Commit {
            message: "Change a, better".into(),
            amend: true,
        },
    )
    .expect("amends");
    let all = commits::read(&git(), &root, 0, None).expect("reads");
    assert_eq!(
        all.iter().map(|c| c.subject.as_str()).collect::<Vec<_>>(),
        ["Change a, better", "first"]
    );
    assert_eq!(
        commit::split("Subject\n\nBody\nmore"),
        ("Subject".into(), "Body\nmore".into())
    );
}

#[test]
fn git_that_quits_unread_says_why() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    // More than a pipe holds: the write must break.
    let input = vec![b'x'; 1 << 20];
    let failed = git()
        .run_input(&root, &["rev-parse", "--verify", "refs/heads/none"], &input)
        .expect_err("no such branch");
    assert!(failed.to_string().contains("git rev-parse (exit"), "{failed}");
}

#[cfg(unix)]
#[test]
fn a_failing_hook_says_why() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    sh(&root, &["config", "user.name", "Ely"]);
    sh(&root, &["config", "user.email", "ely@example.com"]);
    sh(&root, &["config", "commit.gpgsign", "false"]);
    let hook = root.join(".git/hooks/pre-commit");
    std::fs::write(
        &hook,
        "#!/bin/sh\necho 'lint found 3 problems' >&2\nexit 1\n",
    )
    .expect("writes");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    std::fs::write(root.join("a.txt"), "changed\n").expect("writes");
    sh(&root, &["add", "a.txt"]);
    let failed = write(
        &root,
        Write::Commit {
            message: "x".into(),
            amend: false,
        },
    )
    .expect_err("the hook stops it");
    assert!(
        failed.to_string().contains("lint found 3 problems"),
        "{failed}"
    );
}

#[test]
fn identity_and_signing_come_from_git() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    write(
        &root,
        Write::Config {
            key: "user.name".into(),
            value: "Ann Lee".into(),
            global: false,
        },
    )
    .expect("sets");
    write(
        &root,
        Write::Config {
            key: "user.email".into(),
            value: "ann@example.com".into(),
            global: false,
        },
    )
    .expect("sets");
    sh(&root, &["config", "commit.gpgsign", "true"]);
    sh(&root, &["config", "gpg.format", "ssh"]);
    let me = commit::me(&git(), &root).expect("reads");
    assert_eq!(
        (me.name.as_deref(), me.email.as_deref(), me.signs.as_deref()),
        (Some("Ann Lee"), Some("ann@example.com"), Some("ssh"))
    );
    sh(&root, &["config", "commit.gpgsign", "false"]);
    assert_eq!(commit::me(&git(), &root).expect("reads").signs, None);
}

#[test]
fn hash_lines_stay_whatever_commit_cleanup_says() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    for (key, value) in [
        ("user.name", "Ely"),
        ("user.email", "ely@example.com"),
        ("commit.gpgsign", "false"),
        ("commit.cleanup", "strip"),
    ] {
        sh(&root, &["config", key, value]);
    }
    std::fs::write(root.join("a.txt"), "changed\n").expect("writes");
    sh(&root, &["add", "a.txt"]);
    write(
        &root,
        Write::Commit {
            message: "probe\n\n# required body".into(),
            amend: false,
        },
    )
    .expect("commits");
    assert_eq!(
        commit::last_message(&git(), &root).expect("reads"),
        "probe\n\n# required body"
    );
    write(
        &root,
        Write::Commit {
            message: "probe 2\n\n# still here".into(),
            amend: true,
        },
    )
    .expect("amends");
    assert_eq!(
        commit::last_message(&git(), &root).expect("reads"),
        "probe 2\n\n# still here"
    );
}
