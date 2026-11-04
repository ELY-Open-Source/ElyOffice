use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use ely_files::Action;
use ely_office::prelude::*;
use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{Extra, GAP, Outside};
use crate::desk::{
    Desk,
    tests::{open, start},
};

#[test]
fn a_restarted_autosave_saves_the_next_edit() {
    let mut extra = Extra::default();
    extra.policy.set_has_path();
    extra.policy.edit(Instant::now());
    // A write was in flight when the disk changed.
    extra.writing = Some(1);
    extra.restart_autosave();
    // As autosave settles a heard write, then edits.
    if let Some(revision) = extra.writing.take() {
        extra.policy.save_finished(revision, true);
    }
    let now = Instant::now();
    extra.policy.edit(now);
    assert!(matches!(
        extra.policy.tick(now + Duration::from_secs(5)),
        Action::SaveDue(_)
    ));
}

/// A saved file open: its desk, tab and path.
pub(crate) fn opened(
    text: &str,
    cx: &mut TestAppContext,
) -> (
    Entity<Desk>,
    &'static mut VisualTestContext,
    u64,
    PathBuf,
    [tempfile::TempDir; 2],
) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, text).expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let id = desk.read_with(cx, |desk, _| desk.core.active().expect("a tab"));
    (desk, cx, id, path, [data, files])
}

/// The desk looks at the file, as the watcher would.
pub(crate) fn look(desk: &Entity<Desk>, id: u64, cx: &mut VisualTestContext) {
    desk.update(cx, |desk, cx| desk.look(id, cx));
    cx.run_until_parked();
    cx.executor().advance_clock(GAP * 2);
    cx.run_until_parked();
}

pub(crate) fn shown(desk: &Entity<Desk>, cx: &mut VisualTestContext) -> String {
    desk.read_with(cx, |desk, cx| {
        let view = desk.active_view().expect("a tab");
        view.read(cx).document().read(cx).text(cx)
    })
}

#[gpui::test]
fn a_clean_tab_takes_the_disk_and_keeps_its_place(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("one\n\ntwo\n\nthree\n", cx);
    let view = desk.read_with(cx, |desk, _| desk.active_view().expect("a tab"));
    let document = view.read_with(cx, |view, _| view.document().clone());
    document.update(cx, |document, cx| document.set_caret(4, 2, cx));
    std::fs::write(&path, "uno\n\ndos\n\ntres, longer\n").expect("an outside write");
    look(&desk, id, cx);
    assert_eq!(shown(&desk, cx), "uno\n\ndos\n\ntres, longer\n");
    assert!(!view.read_with(cx, |view, _| view.modified()));
    assert_eq!(
        document.read_with(cx, |document, cx| document.caret(cx)),
        (4, 2)
    );
    let focus = view.read_with(cx, |view, _| view.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(
        shown(&desk, cx),
        "uno\n\ndos\n\ntres, longer\n",
        "undo stops at the disk"
    );
}

#[gpui::test]
fn a_deleted_file_keeps_its_text_and_says_so(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("kept\n", cx);
    std::fs::remove_file(&path).expect("deleted");
    look(&desk, id, cx);
    let deleted = desk.read_with(cx, |desk, _| {
        matches!(desk.tab(id).expect("open").extra.outside, Outside::Deleted)
    });
    assert!(deleted);
    assert_eq!(shown(&desk, cx), "kept\n");
}

#[gpui::test]
fn a_renamed_file_is_followed(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("moved\n", cx);
    let to = path.with_file_name("b.md");
    std::fs::rename(&path, &to).expect("renamed");
    look(&desk, id, cx);
    let now = desk.read_with(cx, |desk, _| {
        desk.tab(id)
            .and_then(|tab| tab.session.path().map(PathBuf::from))
    });
    assert_eq!(
        now.map(|path| path.file_name().map(ToOwned::to_owned)),
        Some(Some("b.md".into()))
    );
}

#[gpui::test]
fn a_path_that_cannot_be_read_says_why(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("text\n", cx);
    std::fs::remove_file(&path).expect("removed");
    std::fs::create_dir(&path).expect("a folder in its place");
    look(&desk, id, cx);
    let unread = desk.read_with(cx, |desk, _| {
        matches!(
            desk.tab(id).expect("open").extra.outside,
            Outside::Unread(_)
        )
    });
    assert!(unread);
}

#[gpui::test]
fn a_replace_that_keeps_the_time_still_reloads(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("old text\n", cx);
    let modified = std::fs::metadata(&path)
        .expect("meta")
        .modified()
        .expect("a time");
    let next = path.with_extension("new");
    std::fs::write(&next, "new text\n").expect("a new file");
    let file = std::fs::File::options()
        .write(true)
        .open(&next)
        .expect("open");
    file.set_modified(modified).expect("the old time");
    drop(file);
    std::fs::rename(&next, &path).expect("a replace");
    look(&desk, id, cx);
    assert_eq!(shown(&desk, cx), "new text\n");
}

#[gpui::test]
fn a_stale_read_never_lands(cx: &mut TestAppContext) {
    let (desk, cx, id, path, _dirs) = opened("first\n", cx);
    std::fs::write(&path, "older\n").expect("an older write");
    let older = (
        ely_files::Fingerprint::of(&path).expect("its fingerprint"),
        crate::text::Source::decode(b"older\n".to_vec()).expect("reads"),
    );
    std::fs::write(&path, "newest disk\n").expect("a newer write");
    look(&desk, id, cx);
    assert_eq!(shown(&desk, cx), "newest disk\n");
    let (reads, stamp) = desk.read_with(cx, |desk, _| {
        let extra = &desk.tab(id).expect("open").extra;
        (extra.reads, extra.stamp)
    });
    for number in [reads - 1, reads] {
        let (path, older) = (path.clone(), (older.0, older.1.clone()));
        desk.update(cx, |desk, cx| {
            desk.disk_read(id, path, (number, stamp), Ok(older), cx)
        });
        cx.run_until_parked();
        assert_eq!(shown(&desk, cx), "newest disk\n", "read {number}");
    }
}

mod late;
mod saving;
