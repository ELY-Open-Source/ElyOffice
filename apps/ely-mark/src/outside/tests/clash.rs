//! A save that meets an outside change asks (SF.14).

use ely_office::prelude::*;
use gpui::TestAppContext;

use super::saving::{edit, settle};
use super::{Outside, opened};

#[gpui::test]
fn a_save_over_an_outside_change_asks_then_overwrites(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one\n", cx);
    edit(&desk, "mine\n", cx);
    std::fs::write(&path, "theirs\n").expect("an outside write");
    // The watcher runs on real time; look as it would.
    super::look(&desk, id, cx);
    let (changed, asked) = desk.read_with(cx, |desk, _| {
        (matches!(desk.tab(id).expect("open").extra.outside, Outside::Changed(..)), desk.clash)
    });
    assert!(changed, "the change shows in the status bar");
    assert_eq!(asked, None, "no save, no question");
    desk.update_in(cx, |desk, window, cx| desk.save(id, None, window, cx));
    settle(cx);
    assert_eq!(desk.read_with(cx, |desk, _| desk.clash), Some(id), "the save asks");
    assert_eq!(std::fs::read_to_string(&path).expect("reads"), "theirs\n", "nothing written yet");
    let fingerprint = desk.read_with(cx, |desk, _| match &desk.tab(id).expect("open").extra.outside {
        Outside::Changed(_, fingerprint) => *fingerprint,
        _ => panic!("still changed"),
    });
    desk.update_in(cx, |desk, window, cx| {
        desk.clash = None;
        desk.overwrite(id, fingerprint, window, cx)
    });
    settle(cx);
    assert_eq!(std::fs::read_to_string(&path).expect("reads"), "mine\n");
    assert!(desk.read_with(cx, |desk, _| matches!(desk.tab(id).expect("open").extra.outside, Outside::Clear)));
}

#[gpui::test]
fn a_deleted_file_shows_in_the_status_bar(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one\n", cx);
    edit(&desk, "mine\n", cx);
    std::fs::remove_file(&path).expect("an outside delete");
    super::look(&desk, id, cx);
    let item = desk.update(cx, |desk, cx| desk.outside_item(cx).is_some());
    assert!(item, "a deleted file shows in the status bar");
    assert!(desk.read_with(cx, |desk, _| matches!(desk.tab(id).expect("open").extra.outside, Outside::Deleted)));
}

fn said(desk: &gpui::Entity<crate::desk::Desk>, cx: &mut gpui::VisualTestContext) -> usize {
    let root = desk.read_with(cx, |desk, _| desk.core.root.upgrade().expect("the window"));
    let toasts = root.read_with(cx, |root, cx| root.toasts(cx));
    toasts.iter().filter(|(said, _)| said.contains("a.md")).count()
}

fn unreadable(path: &std::path::Path) {
    std::fs::remove_file(path).expect("removed");
    std::fs::create_dir(path).expect("a folder in its place");
}

#[gpui::test]
fn an_asked_check_says_an_unread_file_again(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("text\n", cx);
    unreadable(&path);
    super::look(&desk, id, cx);
    super::look(&desk, id, cx);
    assert_eq!(said(&desk, cx), 1, "a repeated background look stays quiet");
    desk.update(cx, |desk, cx| desk.check_disk(cx));
    cx.run_until_parked();
    assert_eq!(said(&desk, cx), 2, "the asked check says it again");
    assert!(desk.read_with(cx, |desk, _| desk.tab(id).expect("open").extra.asked.is_none()));
}

#[gpui::test]
fn an_asked_first_failure_leaves_the_next_look_quiet(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("text\n", cx);
    unreadable(&path);
    desk.update(cx, |desk, cx| desk.check_disk(cx));
    cx.run_until_parked();
    assert_eq!(said(&desk, cx), 1, "the asked check says it");
    super::look(&desk, id, cx);
    assert_eq!(said(&desk, cx), 1, "the background look after it stays quiet");
}

#[gpui::test]
fn an_ask_for_a_path_the_tab_left_says_nothing(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("text\n", cx);
    unreadable(&path);
    super::look(&desk, id, cx);
    let gone = path.with_file_name("gone.md");
    desk.update(cx, |desk, _| desk.tab_mut(id).expect("open").extra.asked = Some(gone));
    super::look(&desk, id, cx);
    assert_eq!(said(&desk, cx), 1, "an old path's ask is not this one");
    assert!(desk.read_with(cx, |desk, _| desk.tab(id).expect("open").extra.asked.is_none()));
}
