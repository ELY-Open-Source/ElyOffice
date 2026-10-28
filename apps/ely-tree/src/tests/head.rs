//! An unborn HEAD told from a broken one (G-03).

use super::{committed, git, sh};

#[test]
fn a_new_repository_lists_no_moves_and_no_lfs_files() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path().join("new");
    crate::repo::init(&git(), &root).expect("init");
    assert_eq!(
        crate::recover::reflog(&git(), &root).expect("no reflog error"),
        Vec::new()
    );
    assert_eq!(
        crate::structure::lfs(&git(), &root).expect("no LFS error"),
        Vec::new()
    );
    // A new branch whose name ends in a no-break space.
    let named = committed(&dir.path().join("named"));
    sh(&named, &["branch", "topic"]);
    sh(&named, &["checkout", "-q", "--orphan", "topic\u{a0}"]);
    assert_eq!(crate::recover::reflog(&git(), &named).expect("no reflog error"), Vec::new());
}

#[test]
fn a_broken_head_is_an_error_not_an_empty_repository() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    let branch = sh(&root, &["symbolic-ref", "--short", "HEAD"]).trim().to_owned();
    std::fs::write(root.join(".git/refs/heads").join(branch), "not-an-object-id
").expect("breaks");
    assert!(crate::recover::reflog(&git(), &root).is_err(), "the reflog says why");
    assert!(crate::structure::lfs(&git(), &root).is_err(), "LFS says why");
    // HEAD at a missing tag is no new branch either.
    let other = dir.path().join("other");
    crate::repo::init(&git(), &other).expect("init");
    sh(&other, &["symbolic-ref", "HEAD", "refs/tags/missing"]);
    assert!(crate::recover::reflog(&git(), &other).is_err(), "a tag HEAD says why");
}

#[test]
fn an_orphan_named_above_a_branch_is_new() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    sh(&root, &["branch", "topic/sub"]);
    sh(&root, &["checkout", "-q", "--orphan", "topic"]);
    assert!(!git().born(&root).expect("no error"), "topic/sub is not topic");
    assert_eq!(crate::recover::reflog(&git(), &root).expect("no reflog error"), Vec::new());
    assert_eq!(crate::structure::lfs(&git(), &root).expect("no LFS error"), Vec::new());
}
