//! Saves and looks touch the disk off the window (R15.41).

use std::path::PathBuf;

use ely_files::SaveState;
use ely_office::prelude::*;
use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{GAP, Outside, opened, shown};
use crate::desk::Desk;

fn edit(desk: &Entity<Desk>, text: &str, cx: &mut VisualTestContext) {
    let view = desk.read_with(cx, |desk, _| desk.active_view().expect("a tab"));
    view.update(cx, |view, cx| view.adopt(text.to_owned(), cx));
}

fn save(desk: &Entity<Desk>, id: u64, cx: &mut VisualTestContext) {
    desk.update_in(cx, |desk, window, cx| desk.save(id, None, window, cx));
}

fn saving(desk: &Entity<Desk>, id: u64, cx: &mut VisualTestContext) -> bool {
    desk.read_with(cx, |desk, _| desk.tab(id).expect("open").session.state() == SaveState::Saving)
}

/// Runs tasks singly until tab `id` is seen saving.
fn until_saving(desk: &Entity<Desk>, id: u64, cx: &mut VisualTestContext) -> bool {
    for _ in 0..10_000 {
        if saving(desk, id, cx) {
            return true;
        }
        if !cx.executor().tick() {
            return false;
        }
    }
    false
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.executor().advance_clock(GAP * 2);
    cx.run_until_parked();
}

#[gpui::test]
fn a_save_writes_between_the_windows_turns(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one\n", cx);
    edit(&desk, "two\n", cx);
    save(&desk, id, cx);
    // A write inside the window's turn is never seen running.
    assert!(until_saving(&desk, id, cx), "the write runs apart from the window");
    settle(cx);
    assert_eq!(std::fs::read_to_string(&path).expect("reads"), "two\n");
    let state = desk.read_with(cx, |desk, _| desk.tab(id).expect("open").session.state());
    assert_eq!(state, SaveState::Saved);
}

#[gpui::test]
fn an_outside_write_during_the_save_wins_and_asks(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one\n", cx);
    edit(&desk, "mine\n", cx);
    save(&desk, id, cx);
    assert!(until_saving(&desk, id, cx), "the write runs apart from the window");
    std::fs::write(&path, "theirs, longer\n").expect("an outside write");
    settle(cx);
    assert_eq!(std::fs::read_to_string(&path).expect("reads"), "theirs, longer\n");
    let changed = desk.read_with(cx, |desk, _| {
        matches!(desk.tab(id).expect("open").extra.outside, Outside::Changed(..))
    });
    assert!(changed, "the change is shown, not overwritten");
    assert_eq!(shown(&desk, cx), "mine\n");
}

#[gpui::test]
fn a_save_asked_during_a_write_lands_after_it(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one\n", cx);
    edit(&desk, "two\n", cx);
    save(&desk, id, cx);
    assert!(until_saving(&desk, id, cx), "the write runs apart from the window");
    edit(&desk, "three\n", cx);
    save(&desk, id, cx);
    settle(cx);
    assert_eq!(std::fs::read_to_string(&path).expect("reads"), "three\n");
    let state = desk.read_with(cx, |desk, _| desk.tab(id).expect("open").session.state());
    assert_eq!(state, SaveState::Saved);
}

#[gpui::test]
fn an_edit_asks_the_disk_nothing_about_its_path(cx: &mut TestAppContext) {
    let (desk, cx, _, _, _dirs) = opened("one\n", cx);
    // Only a disk check would put the file back.
    let recent = desk.update(cx, |desk, _| {
        desk.remembered.clear();
        desk.recent_file.clone()
    });
    std::fs::remove_file(&recent).expect("the recent list was written");
    edit(&desk, "two\n", cx);
    settle(cx);
    assert!(!recent.exists(), "the edit checked its path on the disk");
}

#[gpui::test]
fn a_look_reads_the_disk_between_the_windows_turns(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one\n", cx);
    std::fs::write(&path, "two, longer\n").expect("an outside write");
    let reads = |desk: &Desk| desk.tab(id).expect("open").extra.reads;
    let (before, after) = desk.update(cx, |desk, cx| {
        let before = reads(desk);
        desk.look(id, cx);
        (before, reads(desk))
    });
    assert_eq!(before, after, "the look read the disk on the window's turn");
    settle(cx);
    assert_eq!(shown(&desk, cx), "two, longer\n");
}

fn state(desk: &Entity<Desk>, id: u64, cx: &mut VisualTestContext) -> SaveState {
    desk.read_with(cx, |desk, _| desk.tab(id).expect("open").session.state())
}

#[gpui::test]
fn an_edit_after_the_copy_is_taken_stays_unsaved(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one\n", cx);
    edit(&desk, "two\n", cx);
    let copy = desk.read_with(cx, |desk, _| desk.tab(id).expect("open").work.join("save.md"));
    assert!(!copy.exists(), "no copy before the save");
    save(&desk, id, cx);
    let dispatcher = cx.executor().dispatcher().as_test().expect("a test dispatcher").clone();
    // The copy is taken; the window has not heard yet.
    while !copy.exists() {
        while dispatcher.tick(true) {}
        assert!(copy.exists() || dispatcher.tick(false), "the copy was never taken");
    }
    edit(&desk, "three\n", cx);
    cx.run_until_parked();
    assert_eq!(std::fs::read_to_string(&path).expect("reads"), "two\n");
    assert_eq!(state(&desk, id, cx), SaveState::Dirty, "the later edit shows saved");
}

#[gpui::test]
fn opening_a_file_being_saved_as_focuses_the_saving_tab(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one\n", cx);
    let target = path.with_file_name("b.md");
    std::fs::write(&target, "old\n").expect("the target");
    edit(&desk, "mine\n", cx);
    let to = target.clone();
    desk.update_in(cx, |desk, window, cx| desk.write_as(id, to, None, window, cx));
    assert!(until_saving(&desk, id, cx), "the write runs apart from the window");
    desk.update(cx, |desk, cx| desk.open_file(target.clone(), cx));
    settle(cx);
    let (tabs, active) = desk.read_with(cx, |desk, _| (desk.core.tabs.len(), desk.core.active));
    assert_eq!((tabs, active), (1, Some(id)), "one editable tab per file");
    assert_eq!(std::fs::read_to_string(&target).expect("reads"), "mine\n");
}

#[gpui::test]
fn a_read_waits_for_the_disk_seen_again(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one\n", cx);
    std::fs::write(&path, "two, longer\n").expect("an outside write");
    let seen = ely_files::Fingerprint::of(&path).expect("the fingerprint");
    let disk = crate::text::Source::decode(std::fs::read(&path).expect("reads")).expect("decodes");
    desk.update(cx, |desk, cx| {
        let number = desk.tab(id).expect("open").extra.reads;
        desk.disk_read(id, path.clone(), number, Ok((seen, disk)), cx);
    });
    assert_eq!(shown(&desk, cx), "one\n", "the window saw the disk on its turn");
    settle(cx);
    assert_eq!(shown(&desk, cx), "two, longer\n");
}

#[gpui::test]
fn keep_mine_sees_the_disk_between_the_windows_turns(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one\n", cx);
    edit(&desk, "mine\n", cx);
    save(&desk, id, cx);
    assert!(until_saving(&desk, id, cx), "the write runs apart from the window");
    std::fs::write(&path, "theirs, longer\n").expect("an outside write");
    settle(cx);
    assert_eq!(state(&desk, id, cx), SaveState::ExternalModified);
    let exporting = desk.update_in(cx, |desk, window, cx| {
        desk.keep_mine(id, None, window, cx);
        desk.tab(id).expect("open").exporting
    });
    assert!(!exporting, "Keep Mine read the disk on the window's turn");
    settle(cx);
    assert_eq!(std::fs::read_to_string(&path).expect("reads"), "mine\n");
    assert_eq!(state(&desk, id, cx), SaveState::Saved);
}

#[gpui::test]
fn the_recent_list_retries_a_path_it_missed(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one\n", cx);
    let blocker = path.with_file_name("blocker");
    std::fs::write(&blocker, "").expect("a file in the way");
    let good = desk.update(cx, |desk, _| {
        desk.remembered.clear();
        desk.recent.clear();
        desk.tab_mut(id).expect("open").extra.tracked = None;
        std::mem::replace(&mut desk.recent_file, blocker.join("recent"))
    });
    std::fs::remove_file(&good).expect("the list starts empty");
    let track = |desk: &Entity<Desk>, cx: &mut VisualTestContext| {
        desk.update(cx, |desk, cx| desk.track_recent(id, &path, cx));
    };
    // Gone mid-replace: the list waits, the window does not.
    std::fs::rename(&path, blocker.with_extension("away")).expect("moved away");
    track(&desk, cx);
    cx.run_until_parked();
    std::fs::rename(blocker.with_extension("away"), &path).expect("moved back");
    desk.update(cx, |desk, _| desk.recent_file = good.clone());
    // Each notification meanwhile asks the disk nothing.
    track(&desk, cx);
    cx.run_until_parked();
    assert!(!good.exists(), "a failure is retried per notification");
    cx.executor().advance_clock(crate::places::RETRY);
    cx.run_until_parked();
    track(&desk, cx);
    assert!(!good.exists(), "the retry wrote on the window's turn");
    cx.run_until_parked();
    let real = dunce::canonicalize(&path).expect("the real path");
    assert_eq!(desk.read_with(cx, |desk, _| desk.recent.first().cloned()), Some(real));
}

#[gpui::test]
fn a_save_as_onto_a_file_still_loading_is_refused(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one
", cx);
    let target = path.with_file_name("b.md");
    std::fs::write(&target, "old
").expect("the target");
    edit(&desk, "mine
", cx);
    let to = target.clone();
    let held = desk.update_in(cx, |desk, window, cx| {
        desk.write_as(id, to, None, window, cx);
        // Opened mid-export; its tab is not there yet.
        desk.open_file(target.clone(), cx);
        desk.holder(&target).expect("the holder")
    });
    assert!(held.is_some_and(|other| other != id), "a loading file has no holder");
    settle(cx);
    assert_eq!(std::fs::read_to_string(&target).expect("reads"), "old
");
    let paths = desk.read_with(cx, |desk, _| {
        desk.core.tabs.iter().map(|tab| tab.session.path().map(PathBuf::from)).collect::<Vec<_>>()
    });
    assert_eq!(paths, [Some(path), Some(target)], "each file in one tab");
}
