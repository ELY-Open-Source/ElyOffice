//! Disk checks that land after the tab moved on (R15.50).

use std::path::{Path, PathBuf};

use ely_files::{Fingerprint, Sessions};
use ely_office::prelude::*;
use gpui::{Entity, TestAppContext, VisualTestContext};

use super::saving::{edit, settle, until_saving};
use super::{GAP, Outside, look, opened, shown};
use crate::desk::Desk;
use crate::outside::disk::Gone;

/// What a check begun now carries.
type Began = (Option<Fingerprint>, u64);

fn began(desk: &Entity<Desk>, id: u64, cx: &mut VisualTestContext) -> Began {
    desk.read_with(cx, |desk, _| {
        let tab = desk.tab(id).expect("open");
        (tab.session.fingerprint(), tab.extra.stamp)
    })
}

fn at(desk: &Entity<Desk>, id: u64, cx: &mut VisualTestContext) -> Option<PathBuf> {
    desk.read_with(cx, |desk, _| desk.tab(id).expect("open").session.path().map(PathBuf::from))
}

fn deleted(desk: &Entity<Desk>, id: u64, cx: &mut VisualTestContext) -> bool {
    desk.read_with(cx, |desk, _| matches!(desk.tab(id).expect("open").extra.outside, Outside::Deleted))
}

/// Whether `folder` matches names regardless of case.
fn folds(folder: &Path) -> bool {
    std::fs::write(folder.join("probe.md"), "").expect("a probe");
    folder.join("PROBE.md").exists()
}

/// B's file open behind A, nothing watched.
fn beside(desk: &Entity<Desk>, path: &Path, cx: &mut VisualTestContext) -> (u64, PathBuf) {
    desk.update(cx, |desk, _| desk.watcher = None);
    let a = desk.read_with(cx, |desk, _| desk.core.active().expect("a's tab"));
    let other = path.with_file_name("b.md");
    std::fs::write(&other, "two\n").expect("a file");
    desk.update(cx, |desk, cx| desk.open_file(other.clone(), cx));
    settle(cx);
    let b = desk.update(cx, |desk, _| {
        let b = desk.core.active().expect("b's tab");
        desk.core.set_active(Some(a));
        b
    });
    (b, other)
}

/// A's file moves onto B's; what a check saw before.
fn lost(desk: &Entity<Desk>, a: u64, path: &Path, cx: &mut VisualTestContext) -> (Began, Fingerprint) {
    let (_, other) = beside(desk, path, cx);
    let (began, before) = (began(desk, a, cx), Fingerprint::of(path).expect("seen"));
    std::fs::rename(path, &other).expect("moved over b");
    look(desk, a, cx);
    assert!(deleted(desk, a, cx), "the move was followed");
    (began, before)
}

#[gpui::test]
fn a_search_begun_before_a_replacement_lapses(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("old\n", cx);
    desk.update(cx, |desk, _| desk.watcher = None);
    std::fs::rename(&path, path.with_file_name("old.md")).expect("moved away");
    // The search waits out its gap.
    desk.update(cx, |desk, cx| desk.look(id, cx));
    cx.run_until_parked();
    std::fs::write(&path, "new, longer\n").expect("a replacement");
    desk.update(cx, |desk, cx| desk.look(id, cx));
    cx.run_until_parked();
    assert_eq!(shown(&desk, cx), "new, longer\n", "the replacement was not taken");
    std::fs::remove_file(&path).expect("the replacement goes");
    cx.executor().advance_clock(GAP * 2);
    cx.run_until_parked();
    assert_eq!(at(&desk, id, cx), Some(path.clone()), "the old search moved the tab");
    assert_eq!(shown(&desk, cx), "new, longer\n");
    look(&desk, id, cx);
    assert!(deleted(&desk, id, cx), "the replacement's loss went unsaid");
}

#[gpui::test]
fn an_old_look_leaves_a_refused_rename_deleted(cx: &mut TestAppContext) {
    let (desk, cx, a, path, _dirs) = opened("one\n", cx);
    let (began, before) = lost(&desk, a, &path, cx);
    desk.update(cx, |desk, cx| desk.compared(a, path.clone(), began, Ok(before), cx));
    settle(cx);
    assert!(deleted(&desk, a, cx), "an old look cleared the loss");
}

#[gpui::test]
fn an_old_read_leaves_a_refused_rename_deleted(cx: &mut TestAppContext) {
    let (desk, cx, a, path, _dirs) = opened("one\n", cx);
    let ((_, stamp), before) = lost(&desk, a, &path, cx);
    std::fs::write(&path, "back\n").expect("a new a.md");
    let read = (before, crate::text::Source::decode(b"one\n".to_vec()).expect("decodes"));
    desk.update(cx, |desk, cx| {
        let reads = desk.tab(a).expect("open").extra.reads;
        desk.disk_read(a, path.clone(), (reads, stamp), Ok(read), cx);
    });
    settle(cx);
    assert!(deleted(&desk, a, cx), "an old read cleared the loss");
    assert_eq!(shown(&desk, cx), "one\n", "an unrelated file was taken");
}

#[gpui::test]
fn an_old_search_leaves_a_refused_rename_deleted(cx: &mut TestAppContext) {
    let (desk, cx, a, path, _dirs) = opened("one\n", cx);
    let (began, _) = lost(&desk, a, &path, cx);
    std::fs::write(&path, "back\n").expect("a new a.md");
    desk.update(cx, |desk, cx| desk.still_missing(a, path.clone(), began, Ok(Gone::Back), cx));
    settle(cx);
    assert!(deleted(&desk, a, cx), "an old search cleared the loss");
    assert_eq!(shown(&desk, cx), "one\n", "an unrelated file was taken");
}

#[gpui::test]
fn a_search_across_a_write_looks_again(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one\n", cx);
    desk.update(cx, |desk, _| desk.watcher = None);
    let old = path.with_file_name("old.md");
    let began = began(&desk, id, cx);
    std::fs::rename(&path, &old).expect("moved away");
    let key = Sessions::key(&old).expect("the key");
    // Keep Mine writes the file anew meanwhile.
    desk.update_in(cx, |desk, window, cx| desk.keep_mine(id, None, window, cx));
    settle(cx);
    assert!(path.exists(), "Keep Mine wrote nothing");
    let found = Ok(Gone::Renamed(old, key));
    desk.update(cx, |desk, cx| desk.still_missing(id, path.clone(), began, found, cx));
    settle(cx);
    let real = at(&desk, id, cx).map(|at| dunce::canonicalize(at).expect("real"));
    assert_eq!(real, Some(dunce::canonicalize(&path).expect("real")), "an old search moved the tab off its write");
}

#[gpui::test]
fn a_look_that_changes_nothing_keeps_a_newer_one(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one\n", cx);
    desk.update(cx, |desk, _| desk.watcher = None);
    let (began, before) = (began(&desk, id, cx), Fingerprint::of(&path).expect("seen"));
    std::fs::write(&path, "two, longer\n").expect("an outside write");
    let after = Fingerprint::of(&path).expect("seen");
    desk.update(cx, |desk, cx| {
        desk.compared(id, path.clone(), began, Ok(before), cx);
        desk.compared(id, path.clone(), began, Ok(after), cx);
    });
    settle(cx);
    assert_eq!(shown(&desk, cx), "two, longer\n", "the newer look was dropped");
}

#[gpui::test]
fn a_save_as_onto_a_case_variant_mid_replace_is_refused(cx: &mut TestAppContext) {
    let (desk, cx, a, path, _dirs) = opened("one\n", cx);
    let (_, other) = beside(&desk, &path, cx);
    let folds = folds(path.parent().expect("a folder"));
    edit(&desk, "mine\n", cx);
    // B's file is mid-replace when A saves as `B.md`.
    std::fs::remove_file(&other).expect("the gap");
    let variant = path.with_file_name("B.md");
    let to = variant.clone();
    desk.update_in(cx, |desk, window, cx| desk.write_as(a, to, None, window, cx));
    settle(cx);
    match folds {
        true => {
            assert_eq!(at(&desk, a, cx), Some(path), "two tabs on one file");
            assert!(!variant.exists(), "B's file was written over");
        }
        false => {
            let real = dunce::canonicalize(&variant).expect("written");
            assert_eq!(at(&desk, a, cx), Some(real), "another file was refused");
        }
    }
}

#[gpui::test]
fn a_rename_onto_a_case_variant_of_a_save_as_is_not_followed(cx: &mut TestAppContext) {
    let (desk, cx, a, path, _dirs) = opened("one\n", cx);
    let (b, other) = beside(&desk, &path, cx);
    let (target, renamed) = (path.with_file_name("C.md"), path.with_file_name("c.md"));
    edit(&desk, "mine\n", cx);
    let to = target.clone();
    desk.update_in(cx, |desk, window, cx| desk.write_as(a, to, None, window, cx));
    assert!(until_saving(&desk, a, cx), "the write runs apart from the window");
    // B moves onto A's target, spelled otherwise.
    std::fs::rename(&other, &renamed).expect("moved over c");
    let key = Sessions::key(&renamed).expect("the key");
    desk.update(cx, |desk, cx| desk.follow(b, renamed, key, cx));
    settle(cx);
    let (at_a, at_b) = (at(&desk, a, cx), at(&desk, b, cx));
    assert_ne!(at_a, at_b, "two tabs on one file");
    let target = dunce::canonicalize(path.parent().expect("a folder")).expect("real").join("C.md");
    let saved = at_a.is_some_and(|at| ely_files::same_name(&at, &target).expect("compares"));
    assert!(saved, "the Save As lost its target");
}
