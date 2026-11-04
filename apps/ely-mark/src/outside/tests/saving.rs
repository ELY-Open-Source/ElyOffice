//! Saves and looks touch the disk off the window (R15.41).

use std::{
    path::PathBuf,
    sync::{Arc, Barrier, Mutex},
};

use ely_files::SaveState;
use ely_office::prelude::*;
use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{GAP, Outside, opened, shown};
use crate::desk::Desk;

pub(super) fn edit(desk: &Entity<Desk>, text: &str, cx: &mut VisualTestContext) {
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
pub(super) fn until_saving(desk: &Entity<Desk>, id: u64, cx: &mut VisualTestContext) -> bool {
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

pub(super) fn settle(cx: &mut VisualTestContext) {
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
    let (tabs, active) = desk.read_with(cx, |desk, _| (desk.core.tabs.len(), desk.core.active()));
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
        let extra = &desk.tab(id).expect("open").extra;
        desk.disk_read(id, path.clone(), (extra.reads, extra.stamp), Ok((seen, disk)), cx);
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

#[gpui::test]
fn a_target_gone_while_loading_keeps_its_holder(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one
", cx);
    let target = path.with_file_name("b.md");
    std::fs::write(&target, "old
").expect("the target");
    let held = desk.update(cx, |desk, cx| {
        desk.open_file(target.clone(), cx);
        // Gone mid-replace while it loads.
        std::fs::remove_file(&target).expect("the delete");
        desk.holder(&target).expect("the holder")
    });
    assert!(held.is_some_and(|other| other != id), "a missing loading file has no holder");
    settle(cx);
}

#[test]
fn tabs_opened_together_all_reach_the_recent_list() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let file = dir.path().join("recent.json");
    let (turn, start) = (Arc::new(Mutex::new(())), Arc::new(Barrier::new(16)));
    let threads: Vec<_> = (0..16)
        .map(|n| {
            let (file, turn, start) = (file.clone(), turn.clone(), start.clone());
            let path = dir.path().join(format!("{n}.md"));
            std::fs::write(&path, "").expect("a file");
            std::thread::spawn(move || {
                start.wait();
                crate::places::in_turn(&turn, || crate::places::record(&file, &path, None))
            })
        })
        .collect();
    for thread in threads {
        thread.join().expect("the writer ends").expect("recorded");
    }
    let list = ely_files::recent::recent(&file).expect("the list");
    assert_eq!(list.len(), 16, "a concurrent write lost entries");
}

#[gpui::test]
fn recent_list_changes_wait_for_their_turn_off_the_window(cx: &mut TestAppContext) {
    let (desk, cx, _, path, _dirs) = opened("one
", cx);
    cx.run_until_parked();
    let file = desk.read_with(cx, |desk, _| desk.recent_file.clone());
    let real = dunce::canonicalize(&path).expect("the real path");
    let listed = || ely_files::recent::recent(&file).expect("the list").contains(&real);
    assert!(listed(), "the open was recorded");
    desk.update(cx, |desk, cx| desk.forget(&real, cx));
    assert!(listed(), "forget wrote on the window's turn");
    cx.run_until_parked();
    assert!(!listed());
    desk.update(cx, |desk, cx| desk.remember(&path, cx));
    assert!(!listed(), "remember wrote on the window's turn");
    cx.run_until_parked();
    assert!(listed());
    desk.update(cx, |desk, cx| desk.clear_recent(cx));
    assert!(listed(), "clearing wrote on the window's turn");
    cx.run_until_parked();
    assert!(!listed());
    assert!(desk.read_with(cx, |desk, _| desk.recent.is_empty()));
}

#[gpui::test]
fn a_tab_is_found_in_memory_while_its_file_is_away(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one
", cx);
    let spelled = path.parent().expect("a folder").join(".").join("a.md");
    std::fs::remove_file(&path).expect("gone mid-replace");
    let (mark, remote) = desk.update(cx, |desk, _| {
        let asked = spelled.to_str().expect("a UTF-8 path");
        (desk.tab_of(&spelled), ely_office::Remote::tab_of(desk, asked))
    });
    assert_eq!(mark, Some(id), "the window looked for it on the disk");
    assert_eq!(remote, Ok(id));
}

#[gpui::test]
fn a_tab_and_its_asked_line_land_with_no_disk_read(cx: &mut TestAppContext) {
    let (desk, cx, _, path, _dirs) = opened("one
", cx);
    let other = path.with_file_name("b.md");
    std::fs::write(&other, "x
y
z
").expect("a file");
    let work = desk.update_in(cx, |desk, window, cx| {
        let work = desk.core.work.join(format!("doc-{}", desk.core.next));
        desk.open_at(None, other.clone(), (2, 0), window, cx);
        work
    });
    // Its work dir comes last off the window.
    while !work.exists() {
        assert!(cx.executor().tick(), "the load never read the file");
    }
    std::fs::remove_file(&other).expect("the file leaves before the tab lands");
    cx.run_until_parked();
    let (tabs, going) = desk.read_with(cx, |desk, _| (desk.core.tabs.len(), desk.going.is_some()));
    assert_eq!(tabs, 2, "the landing read the disk");
    assert!(!going, "the asked line waited on the disk");
}

#[gpui::test]
fn a_file_moved_onto_another_tabs_is_not_shared(cx: &mut TestAppContext) {
    let (desk, cx, a, path, _dirs) = opened("one
", cx);
    let other = path.with_file_name("b.md");
    std::fs::write(&other, "two
").expect("a file");
    desk.update(cx, |desk, cx| desk.open_file(other.clone(), cx));
    settle(cx);
    std::fs::rename(&path, &other).expect("moved over b");
    super::look(&desk, a, cx);
    let (paths, deleted) = desk.read_with(cx, |desk, _| {
        let paths = desk.core.tabs.iter().map(|tab| tab.session.path().map(PathBuf::from)).collect::<Vec<_>>();
        (paths, matches!(desk.tab(a).expect("open").extra.outside, Outside::Deleted))
    });
    assert_eq!(paths, [Some(path), Some(other)], "two tabs on one file");
    assert!(deleted, "the moved tab lost its file");
}

#[gpui::test]
fn a_rename_onto_a_save_as_target_is_not_followed(cx: &mut TestAppContext) {
    let (desk, cx, a, path, _dirs) = opened("one
", cx);
    let (other, target) = (path.with_file_name("b.md"), path.with_file_name("c.md"));
    std::fs::write(&other, "two
").expect("a file");
    desk.update(cx, |desk, cx| desk.open_file(other.clone(), cx));
    settle(cx);
    let b = desk.read_with(cx, |desk, _| desk.core.active().expect("b's tab"));
    desk.update(cx, |desk, _| desk.core.set_active(Some(a)));
    edit(&desk, "mine
", cx);
    let to = target.clone();
    desk.update_in(cx, |desk, window, cx| desk.write_as(a, to, None, window, cx));
    assert!(until_saving(&desk, a, cx), "the write runs apart from the window");
    // B is renamed over A's target before A lands.
    std::fs::rename(&other, &target).expect("moved over c");
    let key = ely_files::Sessions::key(&target).expect("the key");
    desk.update(cx, |desk, cx| desk.follow(b, target.clone(), key, cx));
    settle(cx);
    let (at_a, at_b) = desk.read_with(cx, |desk, _| {
        let at = |id| desk.tab(id).expect("open").session.path().map(PathBuf::from);
        (at(a), at(b))
    });
    assert_ne!(at_a, at_b, "two tabs on one file");
    let real = dunce::canonicalize(target.parent().expect("a folder")).expect("real").join("c.md");
    assert_eq!(at_a, Some(real), "the Save As lost its target");
}

#[gpui::test]
fn a_line_asked_twice_during_a_load_lands_the_last(cx: &mut TestAppContext) {
    let (desk, cx, _, path, _dirs) = opened("one
", cx);
    let other = path.with_file_name("b.md");
    std::fs::write(&other, "x
y
z
w
").expect("a file");
    let spelled = path.parent().expect("a folder").join(".").join("b.md");
    desk.update_in(cx, |desk, window, cx| {
        desk.open_at(None, other.clone(), (1, 0), window, cx);
        desk.open_at(None, spelled, (3, 0), window, cx);
    });
    settle(cx);
    let (tabs, going) = desk.read_with(cx, |desk, _| (desk.core.tabs.len(), desk.going.is_some()));
    assert_eq!((tabs, going), (2, false), "a line was left waiting");
    let line = desk.read_with(cx, |desk, cx| desk.active_view().expect("a tab").read(cx).place(cx).line);
    assert_eq!(line, 3, "the later asked line was dropped");
}

fn save_as_onto_a_taken_file(then: Option<ely_office::After>, cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one
", cx);
    let target = path.with_file_name("c.md");
    std::fs::write(&target, "old
").expect("the target");
    edit(&desk, "mine
", cx);
    let to = target.clone();
    desk.update_in(cx, |desk, window, cx| desk.write_as(id, to, then, window, cx));
    assert!(until_saving(&desk, id, cx), "the write runs apart from the window");
    // Another session takes the target before the landing.
    let taken = desk.update(cx, |desk, _| desk.core.sessions.open(&target, false).expect("registers"));
    assert!(matches!(taken, ely_files::Open::Fresh(_)));
    settle(cx);
    let (open, registry, path, state, dirty) = desk.read_with(cx, |desk, cx| {
        let tab = desk.tab(id).expect("the tab stays");
        (desk.core.tabs.len(), tab.registry, tab.session.path().map(PathBuf::from), tab.session.state(), tab.dirty(cx))
    });
    assert_eq!((open, registry, path), (1, None, None), "the tab still names the taken file");
    assert_ne!(state, SaveState::Saved, "the collision shows as saved");
    assert!(dirty, "the text is unsaved");
}

#[gpui::test]
fn a_save_as_landing_on_a_taken_file_stays_unsaved(cx: &mut TestAppContext) {
    save_as_onto_a_taken_file(None, cx);
}

#[gpui::test]
fn a_close_waiting_on_a_taken_save_as_keeps_the_tab(cx: &mut TestAppContext) {
    save_as_onto_a_taken_file(Some(ely_office::After::Tab), cx);
}

#[gpui::test]
fn a_refused_rename_is_not_looked_at_again(cx: &mut TestAppContext) {
    let (desk, cx, a, path, _dirs) = opened("one
", cx);
    desk.update(cx, |desk, _| desk.watcher = None);
    let other = path.with_file_name("b.md");
    std::fs::write(&other, "two
").expect("a file");
    desk.update(cx, |desk, cx| desk.open_file(other.clone(), cx));
    settle(cx);
    std::fs::rename(&path, &other).expect("moved over b");
    super::look(&desk, a, cx);
    // Only another look would find the file back.
    std::fs::write(&path, "back
").expect("a new a.md");
    for _ in 0..4 {
        settle(cx);
    }
    let deleted = desk.read_with(cx, |desk, _| matches!(desk.tab(a).expect("open").extra.outside, Outside::Deleted));
    assert!(deleted, "the refused rename was looked at again");
}
