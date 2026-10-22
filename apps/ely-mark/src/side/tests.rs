use gpui::{ScrollDelta, ScrollWheelEvent, TestAppContext, point, px};

use super::Pane;
use crate::desk::tests::{open, start};

#[gpui::test]
fn a_long_outline_scrolls_to_its_last_heading(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("long.md");
    let text: String = (0..100).map(|n| format!("# Heading {n}\n\n")).collect();
    std::fs::write(&path, text).expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let side = desk.read_with(cx, |desk, _| desk.side.clone());
    side.update(cx, |side, cx| {
        side.pane = Pane::Outline;
        cx.notify();
    });
    cx.run_until_parked();
    let before = cx
        .debug_bounds("mark-outline")
        .expect("the outline is drawn");
    let window = cx.update(|window, _| window.viewport_size());
    assert!(
        before.bottom() > window.height,
        "the outline is longer than the window"
    );
    let at = point(before.left() + px(20.0), px(300.0));
    for _ in 0..20 {
        cx.simulate_event(ScrollWheelEvent {
            position: at,
            delta: ScrollDelta::Pixels(point(px(0.0), px(-1000.0))),
            ..Default::default()
        });
        cx.run_until_parked();
    }
    let after = cx
        .debug_bounds("mark-outline")
        .expect("the outline is drawn");
    assert!(
        after.bottom() <= window.height,
        "the last heading is in view: {after:?}"
    );
}

/// Searches the folder for `words`; the hits as found.
fn search(
    side: &gpui::Entity<super::Side>,
    words: &str,
    cx: &mut gpui::VisualTestContext,
) -> Vec<(Option<u64>, crate::folder::Hit)> {
    side.update(cx, |side, cx| {
        side.query.update(cx, |query, cx| query.set_text(words, cx));
        side.wide = true;
        side.search(cx);
    });
    cx.run_until_parked();
    side.read_with(cx, |side, _| side.hits.clone())
}

#[gpui::test]
fn a_hit_opens_its_file_with_the_caret_on_the_match(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    std::fs::write(files.path().join("a.md"), "# A\n\nNo match.\n").expect("a");
    std::fs::write(files.path().join("b.md"), "# B\n\nThe Needle here.\n").expect("b");
    let (desk, cx) = start(cx, data.path());
    open(&desk, files.path(), cx);
    let side = desk.read_with(cx, |desk, _| desk.side.clone());
    let hits = search(&side, "needle", cx);
    let found: Vec<_> = hits.iter().map(|(_, hit)| (hit.line, hit.column)).collect();
    assert_eq!(found, [(3, 4)]);
    let (tab, hit) = hits[0].clone();
    assert_eq!(tab, None, "b.md is not open");
    desk.update_in(cx, |desk, window, cx| {
        desk.open_at(tab, hit.file, (hit.line - 1, hit.column), window, cx)
    });
    cx.run_until_parked();
    let place = desk.read_with(cx, |desk, cx| {
        desk.active_view().expect("b.md opened").read(cx).place(cx)
    });
    assert_eq!((place.line, place.column), (2, 4));
}

#[gpui::test]
fn a_hit_in_an_open_file_moves_its_caret(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, "# A\n\nfirst\n\nsecond Word\n").expect("a");
    let (desk, cx) = start(cx, data.path());
    open(&desk, files.path(), cx);
    open(&desk, &path, cx);
    let side = desk.read_with(cx, |desk, _| desk.side.clone());
    let hits = search(&side, "word", cx);
    let (tab, hit) = hits[0].clone();
    assert!(tab.is_some(), "the open tab searches");
    desk.update_in(cx, |desk, window, cx| {
        desk.open_at(tab, hit.file, (hit.line - 1, hit.column), window, cx)
    });
    cx.run_until_parked();
    let place = desk.read_with(cx, |desk, cx| {
        desk.active_view().expect("a").read(cx).place(cx)
    });
    assert_eq!((place.line, place.column), (4, 7));
}

#[gpui::test]
fn the_side_folds_from_its_key_and_stays_folded(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, "# A\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let side = desk.read_with(cx, |desk, _| desk.side.clone());
    side.update(cx, |side, cx| {
        side.pane = Pane::Outline;
        cx.notify();
    });
    cx.run_until_parked();
    assert!(cx.debug_bounds("mark-outline").is_some(), "the side shows");
    let focus = desk.read_with(cx, |desk, _| desk.core.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    let kept = || crate::look::load(&data.path().join("look.json")).expect("the look reads");
    // The panel slides; wait out its motion.
    let drawn = |cx: &mut gpui::VisualTestContext, want: bool| {
        for _ in 0..50 {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
            if cx.debug_bounds("mark-outline").is_some() == want {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        false
    };
    cx.simulate_keystrokes("secondary-shift-l");
    assert!(drawn(cx, false), "the side folds");
    assert!(!kept().side && !kept().focus, "only the side is kept folded");
    cx.simulate_keystrokes("secondary-shift-l");
    assert!(drawn(cx, true), "the side is back");
    assert!(kept().side);
}

#[gpui::test]
fn a_shut_recent_menu_stays_shut_when_files_return(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, "# A\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let list = cx.debug_bounds("mark-recent-list").expect("the recent list");
    let row = point(list.center().x, list.top() + px(12.0));
    cx.simulate_mouse_down(row, gpui::MouseButton::Right, gpui::Modifiers::none());
    cx.simulate_mouse_up(row, gpui::MouseButton::Right, gpui::Modifiers::none());
    cx.run_until_parked();
    let menu = "menu-row-Remove from Recent";
    assert!(cx.debug_bounds(menu).is_some(), "opened");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds(menu).is_none(), "shut");
    let side = desk.read_with(cx, |desk, _| desk.side.clone());
    for pane in [Pane::Outline, Pane::Files] {
        side.update(cx, |side, cx| {
            side.pane = pane;
            cx.notify();
        });
        cx.run_until_parked();
    }
    assert!(cx.debug_bounds("mark-recent-list").is_some(), "files are back");
    assert!(cx.debug_bounds(menu).is_none(), "not replayed");
}
