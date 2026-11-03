//! Saves and looks touch the disk off the window (R15.41).

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
