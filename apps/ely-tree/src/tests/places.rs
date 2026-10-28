//! Stashes, remotes, worktrees, submodules, LFS (C13.5).

use std::path::{Path, PathBuf};

use super::{committed, git, sh};
use crate::{
    detail, remotes, stash,
    structure::{self, Checkout},
    work::{self, Write},
};

/// Identity in the repository's own config, for writes.
pub(crate) fn me(root: &Path) {
    for (key, value) in [
        ("user.name", "Ely"),
        ("user.email", "ely@example.com"),
        ("commit.gpgsign", "false"),
    ] {
        sh(root, &["config", key, value]);
    }
}

fn run(root: &Path, write: Write) -> std::io::Result<()> {
    work::run(&git(), root, &write, &write.args(true))
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|word| word.to_string()).collect()
}

#[test]
fn stashes_list_newest_first_with_untracked_files_apart() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    me(&root);
    std::fs::write(root.join("a.txt"), "one\nchanged\n").expect("writes");
    std::fs::write(root.join("new.txt"), "fresh\n").expect("writes");
    let push = |more: &[&str]| {
        let mut args = words(&["stash", "push"]);
        args.extend(words(more));
        run(&root, Write::Git(args)).expect("stashes");
    };
    push(&["--include-untracked", "-m", "first: words"]);
    std::fs::write(root.join("a.txt"), "one\nagain\n").expect("writes");
    push(&[]);
    let list = stash::list(&git(), &root).expect("reads");
    let branch = sh(&root, &["branch", "--show-current"]).trim().to_string();
    assert_eq!(list.len(), 2);
    assert_eq!(
        (list[1].branch.as_str(), list[1].message.as_str()),
        (branch.as_str(), "first: words")
    );
    assert_eq!(list[0].branch, branch);
    assert!(list[0].message.ends_with("first"), "{}", list[0].message);
    let held = stash::detail(&git(), &root, &list[1].oid).expect("reads");
    let files: Vec<(&str, bool)> = held
        .files
        .iter()
        .map(|file| (file.path.as_str(), file.commit.is_some()))
        .collect();
    assert_eq!(files, [("a.txt", false), ("new.txt", true)]);
    let untracked = &held.files[1];
    let holder = untracked.commit.clone().expect("its own commit");
    let found = detail::file(&git(), &root, &holder, &[], untracked).expect("reads");
    let crate::shown::View::Text { seen, .. } = crate::shown::view(found).expect("a view") else {
        panic!("text");
    };
    assert!(String::from_utf8_lossy(&seen).contains("+fresh"));
}

#[test]
fn a_pop_that_conflicts_keeps_its_stash() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    me(&root);
    std::fs::write(root.join("a.txt"), "one\nstashed\n").expect("writes");
    sh(&root, &["stash", "push", "-q"]);
    std::fs::write(root.join("a.txt"), "one\ncommitted\n").expect("writes");
    sh(&root, &["commit", "-qam", "meanwhile"]);
    let oid = stash::list(&git(), &root).expect("reads")[0].oid.clone();
    let popped = run(
        &root,
        Write::Stash {
            verb: "pop",
            at: 0,
            oid: oid.clone(),
        },
    );
    assert!(
        popped
            .expect_err("conflicts")
            .to_string()
            .contains("CONFLICT")
    );
    let left = stash::list(&git(), &root).expect("reads");
    assert_eq!(
        left.iter().map(|s| s.oid.as_str()).collect::<Vec<_>>(),
        [oid.as_str()]
    );
}

#[test]
fn a_stash_command_refuses_a_moved_list() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    me(&root);
    std::fs::write(root.join("a.txt"), "one\nfirst\n").expect("writes");
    sh(&root, &["stash", "push", "-q"]);
    let first = stash::list(&git(), &root).expect("reads")[0].oid.clone();
    std::fs::write(root.join("a.txt"), "one\nsecond\n").expect("writes");
    sh(&root, &["stash", "push", "-q"]);
    let dropped = run(
        &root,
        Write::Stash {
            verb: "drop",
            at: 0,
            oid: first,
        },
    );
    assert!(
        dropped
            .expect_err("refused")
            .to_string()
            .contains("changed")
    );
    assert_eq!(stash::list(&git(), &root).expect("reads").len(), 2);
}

#[test]
fn remotes_list_urls_and_push_urls() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    for args in [
        words(&["remote", "add", "--", "origin", "/srv/one"]),
        words(&["remote", "add", "--", "a.b", "/srv/two"]),
        words(&["remote", "set-url", "--push", "--", "a.b", "/srv/push"]),
    ] {
        run(&root, Write::Git(args)).expect("runs");
    }
    let list = remotes::list(&git(), &root).expect("reads");
    let shown: Vec<(&str, Vec<&str>, Vec<&str>)> = list
        .iter()
        .map(|r| {
            (
                r.name.as_str(),
                r.urls.iter().map(String::as_str).collect(),
                r.push.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    assert_eq!(
        shown,
        [
            ("a.b", vec!["/srv/two"], vec!["/srv/push"]),
            ("origin", vec!["/srv/one"], vec![]),
        ]
    );
}

/// A bare remote and two clones of it.
fn shared(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let source = committed(&dir.join("source"));
    let bare = dir.join("remote.git");
    sh(
        dir,
        &[
            "clone",
            "-q",
            "--bare",
            source.to_str().expect("utf-8"),
            bare.to_str().expect("utf-8"),
        ],
    );
    let clone = |name: &str| {
        let into = dir.join(name);
        git()
            .clone_repo(bare.to_str().expect("utf-8"), &into)
            .expect("starts")
            .follow(|_| {})
            .expect("clones");
        me(&into);
        into
    };
    let (a, b) = (clone("a"), clone("b"));
    (bare, a, b)
}

#[test]
fn a_forced_push_names_what_it_overwrites_and_holds_its_lease() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (bare, a, b) = shared(dir.path());
    std::fs::write(a.join("a.txt"), "theirs\n").expect("writes");
    sh(&a, &["commit", "-qam", "theirs"]);
    run(&a, Write::Remote(words(&["push", "--progress"]))).expect("pushes");
    std::fs::write(b.join("a.txt"), "mine\n").expect("writes");
    sh(&b, &["commit", "-qam", "mine"]);
    run(
        &b,
        Write::Remote(words(&["fetch", "--progress", "--", "origin"])),
    )
    .expect("fetches");
    let plan = remotes::force(&git(), &b).expect("a plan");
    let branch = sh(&b, &["branch", "--show-current"]).trim().to_string();
    assert_eq!(
        (
            plan.branch.as_str(),
            plan.remote.as_str(),
            plan.target.as_str()
        ),
        (
            branch.as_str(),
            "origin",
            format!("refs/heads/{branch}").as_str()
        )
    );
    assert_eq!(plan.lost.len(), 1);
    assert!(plan.lost[0].ends_with(" theirs"), "{:?}", plan.lost);
    // The remote moves after the plan; the lease refuses.
    std::fs::write(a.join("a.txt"), "theirs again\n").expect("writes");
    sh(&a, &["commit", "-qam", "theirs again"]);
    run(&a, Write::Remote(words(&["push", "--progress"]))).expect("pushes");
    let refused = run(&b, Write::Remote(plan.args())).expect_err("stale lease");
    assert!(refused.to_string().contains("stale info"), "{refused}");
    run(&b, Write::Remote(words(&["fetch", "--progress", "--all"]))).expect("fetches");
    let plan = remotes::force(&git(), &b).expect("a plan");
    assert_eq!(plan.lost.len(), 2);
    run(&b, Write::Remote(plan.args())).expect("forces");
    let there = sh(&bare, &["rev-parse", &format!("refs/heads/{branch}")]);
    assert_eq!(there, sh(&b, &["rev-parse", "HEAD"]));
}

#[test]
fn a_detached_head_or_no_upstream_has_no_forced_push() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    let none = remotes::force(&git(), &root).expect_err("no upstream");
    assert!(none.to_string().contains("no upstream"), "{none}");
    sh(&root, &["checkout", "-q", "--detach"]);
    assert!(remotes::force(&git(), &root).is_err());
}

#[test]
fn worktrees_list_branches_and_prunable_folders() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    let other = dir.path().join("other tree");
    let path = other.to_str().expect("utf-8");
    run(
        &root,
        Write::Git(words(&["worktree", "add", "-b", "side", "--", path])),
    )
    .expect("adds");
    let list = structure::worktrees(&git(), &root).expect("reads");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].path, dunce::canonicalize(&root).expect("real"));
    assert_eq!(
        (list[1].path.clone(), list[1].branch.as_deref()),
        (dunce::canonicalize(&other).expect("real"), Some("side"))
    );
    std::fs::remove_dir_all(&other).expect("removes");
    let list = structure::worktrees(&git(), &root).expect("reads");
    assert!(list[1].prunable.is_some(), "{list:?}");
    run(&root, Write::Git(words(&["worktree", "prune"]))).expect("prunes");
    assert_eq!(structure::worktrees(&git(), &root).expect("reads").len(), 1);
}

#[test]
fn submodules_say_whether_their_commit_is_checked_out() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let sub = committed(&dir.path().join("s"));
    let main = committed(&dir.path().join("m"));
    let file = ["-c", "protocol.file.allow=always"];
    let add = [
        &file[..],
        &[
            "submodule",
            "add",
            "-q",
            sub.to_str().expect("utf-8"),
            "lib",
        ],
    ]
    .concat();
    sh(&main, &add);
    sh(&main, &["commit", "-qm", "with lib"]);
    let recorded = sh(&sub, &["rev-parse", "HEAD"]).trim().to_string();
    let list = structure::submodules(&git(), &main).expect("reads");
    assert_eq!(list.len(), 1);
    assert_eq!(
        (list[0].path.as_str(), &list[0].checkout, list[0].conflicted),
        ("lib", &Checkout::At(recorded.clone()), false)
    );
    let copy = dir.path().join("copy");
    git()
        .clone_repo(main.to_str().expect("utf-8"), &copy)
        .expect("starts")
        .follow(|_| {})
        .expect("clones");
    let list = structure::submodules(&git(), &copy).expect("reads");
    assert_eq!(
        (list[0].recorded.as_str(), &list[0].checkout),
        (recorded.as_str(), &Checkout::Missing)
    );
    let update = [
        &file[..],
        &["submodule", "update", "--init", "--progress", "--", "lib"],
    ]
    .concat();
    sh(&copy, &update);
    let list = structure::submodules(&git(), &copy).expect("reads");
    assert_eq!(list[0].checkout, Checkout::At(recorded));
}

#[test]
fn lfs_files_say_whether_the_object_is_here() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    assert!(structure::lfs(&git(), &root).expect("reads").is_empty());
    sh(&root, &["lfs", "install", "--local"]);
    sh(&root, &["lfs", "track", "*.bin"]);
    std::fs::write(root.join("big.bin"), vec![7u8; 3000]).expect("writes");
    sh(&root, &["add", "."]);
    sh(&root, &["commit", "-qm", "lfs"]);
    let here = structure::lfs(&git(), &root).expect("reads");
    assert_eq!(here.len(), 1);
    assert_eq!(
        (
            here[0].path.as_str(),
            here[0].size,
            here[0].checked_out,
            here[0].downloaded
        ),
        ("big.bin", 3000, true, true)
    );
    let copy = dir.path().join("copy");
    let cloned = std::process::Command::new(&git().path)
        .args([
            "clone",
            "-q",
            root.to_str().expect("utf-8"),
            copy.to_str().expect("utf-8"),
        ])
        .env("GIT_LFS_SKIP_SMUDGE", "1")
        .status()
        .expect("runs");
    assert!(cloned.success());
    let there = structure::lfs(&git(), &copy).expect("reads");
    assert_eq!((there[0].checked_out, there[0].downloaded), (false, false));
}

#[test]
fn a_forced_push_sends_the_commit_it_showed() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (bare, b, _) = shared(dir.path());
    let base = sh(&b, &["rev-parse", "HEAD"]).trim().to_string();
    std::fs::write(b.join("a.txt"), "keep me\n").expect("writes");
    sh(&b, &["commit", "-qam", "keep-me"]);
    run(&b, Write::Remote(words(&["push", "--progress"]))).expect("pushes");
    let plan = remotes::force(&git(), &b).expect("a plan");
    assert!(plan.lost.is_empty());
    // The branch moves back before the yes.
    sh(&b, &["reset", "-q", "--hard", &base]);
    run(&b, Write::Remote(plan.args())).expect("pushes");
    let branch = &plan.branch;
    assert_eq!(
        sh(&bare, &["rev-parse", &format!("refs/heads/{branch}")]).trim(),
        plan.source,
        "keep-me stays"
    );
}

#[cfg(unix)]
#[test]
fn every_line_a_hook_says_is_kept() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().expect("a temp dir");
    let (bare, b, _) = shared(dir.path());
    let hook = bare.join("hooks/pre-receive");
    std::fs::write(
        &hook,
        "#!/bin/sh\necho POLICY_A required >&2\necho POLICY_B required >&2\nexit 1\n",
    )
    .expect("writes");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    std::fs::write(b.join("a.txt"), "refused\n").expect("writes");
    sh(&b, &["commit", "-qam", "refused"]);
    let said = run(&b, Write::Remote(words(&["push", "--progress"]))).expect_err("refused");
    let said = said.to_string();
    assert!(
        said.contains("POLICY_A required") && said.contains("POLICY_B required"),
        "{said}"
    );
}

#[test]
fn a_remote_named_with_a_dash_is_a_name() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (bare, b, _) = shared(dir.path());
    let path = bare.to_str().expect("utf-8");
    run(
        &b,
        Write::Git(words(&["remote", "add", "--", "-dash", path])),
    )
    .expect("adds");
    sh(&b, &["switch", "-q", "-c", "topic"]);
    let push = words(&[
        "push",
        "--progress",
        "--set-upstream",
        "--",
        "-dash",
        "refs/heads/topic:refs/heads/topic",
    ]);
    run(&b, Write::Remote(push)).expect("pushes");
    assert_eq!(
        sh(&b, &["config", "--get", "branch.topic.remote"]).trim(),
        "-dash"
    );
}

#[test]
fn a_stash_with_an_empty_message_still_lists() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    me(&root);
    std::fs::write(root.join("a.txt"), "one\nfirst\n").expect("writes");
    sh(&root, &["stash", "push", "-q", "-m", "named"]);
    std::fs::write(root.join("a.txt"), "one\nbare\n").expect("writes");
    let oid = sh(&root, &["stash", "create"]).trim().to_string();
    sh(&root, &["stash", "store", "-m", "", &oid]);
    let list = stash::list(&git(), &root).expect("reads");
    let shown: Vec<&str> = list.iter().map(|s| s.message.as_str()).collect();
    assert_eq!(shown, ["", "named"]);
    assert_eq!(list[0].oid, oid);
}

#[test]
fn a_worktree_registered_through_a_link_lists_its_real_path() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    let other = dir.path().join("tree");
    let path = other.to_str().expect("utf-8");
    run(
        &root,
        Write::Git(words(&["worktree", "add", "-b", "side", "--", path])),
    )
    .expect("adds");
    // Other tools may write the path through a link.
    #[cfg(unix)]
    std::os::unix::fs::symlink(dir.path(), dir.path().join("link")).expect("links");
    #[cfg(windows)]
    {
        let link = dir.path().join("link");
        let made = std::process::Command::new("cmd").arg("/C").arg("mklink").arg("/J").arg(&link).arg(dir.path()).output().expect("cmd");
        assert!(made.status.success(), "a junction: {}", String::from_utf8_lossy(&made.stderr));
    }
    let gitdir = root.join(".git/worktrees/tree/gitdir");
    std::fs::write(
        &gitdir,
        format!("{}\n", dir.path().join("link/tree/.git").display()),
    )
    .expect("writes");
    let list = structure::worktrees(&git(), &root).expect("reads");
    assert_eq!(list[1].path, dunce::canonicalize(&other).expect("real"));
}
