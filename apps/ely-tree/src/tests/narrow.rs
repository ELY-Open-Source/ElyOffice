//! A narrow window keeps each split's diff in view (C05.7).

use gpui::{TestAppContext, VisualTestContext, px, size};

use super::{
    committed, sh,
    window::{branch, settle, window},
};
use crate::{look::Look, tree::Side};

/// The list ends inside the window; the diff keeps more.
fn fits(cx: &mut VisualTestContext, list: &'static str, shown: &'static str) {
    let list_at = cx.debug_bounds(list).expect("the list");
    let shown_at = cx.debug_bounds(shown).expect("the diff");
    assert!(list_at.right() <= px(640.0), "{list} ends at {:?}", list_at.right());
    assert!(shown_at.size.width >= list_at.size.width, "{shown} has {:?} beside {:?}", shown_at.size.width, list_at.size.width);
}

#[test]
fn file_history_and_a_commits_files_leave_room_for_the_diff() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = committed(dir.path());
    let mut app = TestAppContext::single();
    let (tree, cx) = window(&mut app, dir.path());
    cx.simulate_resize(size(px(640.0), px(680.0)));
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.file_log("a.txt", cx)));
    settle(cx, |app| matches!(tree.read(app).look, Some(Look::FileLog { touches: Some(Ok(_)), .. })));
    cx.update(|_, app| tree.update(app, |tree, cx| tree.pick_touch(0, cx)));
    settle(cx, |app| matches!(tree.read(app).look, Some(Look::FileLog { view: Some(Ok(_)), .. })));
    fits(cx, "tree-file-log-list", "tree-file-log-shown");

    let head = sh(&root, &["rev-parse", "HEAD"]).trim().to_string();
    cx.update(|_, app| tree.update(app, |tree, cx| tree.set_side(Side::History, cx)));
    cx.update(|_, app| tree.update(app, |tree, cx| tree.pick_commit(&head, cx)));
    settle(cx, |app| tree.read(app).active().and_then(|o| o.hist.pick.detail.as_ref()).is_some_and(|d| d.is_ok()));
    cx.update(|_, app| tree.update(app, |tree, cx| tree.pick_touched(crate::hist::Whose::History, 0, cx)));
    settle(cx, |app| matches!(tree.read(app).active().and_then(|o| o.hist.pick.file.as_ref()), Some((0, Some(Ok(_))))));
    fits(cx, "tree-touched", "tree-touched-shown");
}
