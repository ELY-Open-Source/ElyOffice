use std::sync::Arc;

use ely_office::prelude::*;

use gpui::{TestAppContext, TestDispatcher};

use super::{Counts, count};
use crate::desk::tests::{open, start};
use crate::look::font_tests::Families;
use crate::view::Mode;

#[test]
fn ideographs_count_one_each_and_characters_as_seen() {
    assert_eq!(count("你好世界"), Counts { words: 4, characters: 4 });
    assert_eq!(count("Hello, world!"), Counts { words: 2, characters: 13 });
    assert_eq!(count("# 第 1 行\n"), Counts { words: 3, characters: 8 });
    assert_eq!(count("e\u{301}"), Counts { words: 1, characters: 1 });
}

#[test]
fn the_status_bar_counts_typing_and_any_selection() {
    let lines = Arc::new(Families::default());
    let mut cx = TestAppContext::build_with_text_system(TestDispatcher::new(0), None, lines.clone());
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let path = files.path().join("a.md");
    std::fs::write(&path, "你好 world!\n").expect("a file");
    let (desk, cx) = start(&mut cx, data.path());
    // What the status bar drew, read from text layout.
    let drew = |said: &str, cx: &mut gpui::VisualTestContext| {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        let laid = std::mem::take(&mut *lines.laid.lock().expect("the lines"));
        laid.iter().any(|line| line == said)
    };
    open(&desk, &path, cx);
    assert!(drew("3 words, 10 characters", cx));
    let view = desk.read_with(cx, |desk, _| desk.active_view().expect("a tab"));
    view.update_in(cx, |view, window, cx| view.switch(Mode::Source, window, cx));
    cx.simulate_input("天 ");
    assert!(drew("4 words, 12 characters", cx));
    cx.simulate_keystrokes("end shift-left");
    assert!(drew("0 of 4 words", cx), "a mark alone is a selection");
    // A frame drawn before the executor runs counts nothing.
    let all = gpui::Keystroke::parse("secondary-a").expect("a key");
    cx.update(|window, cx| window.dispatch_keystroke(all, cx));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let laid = std::mem::take(&mut *lines.laid.lock().expect("the lines"));
    assert!(!laid.iter().any(|line| line == "4 of 4 words"), "drawing counted");
    assert!(drew("4 of 4 words", cx), "the count lands later");
    // An older selection's count arriving last is dropped.
    let id = desk.read_with(cx, |desk, _| desk.core.active.expect("a tab"));
    let old: Arc<str> = "!".into();
    desk.update(cx, |desk, cx| desk.land_selection(id, &old, 0, cx));
    assert!(!drew("0 of 4 words", cx), "a stale count landed");
    let kept = desk.read_with(cx, |desk, _| desk.tab(id).and_then(|tab| tab.extra.selection.clone()));
    assert_eq!(kept.and_then(|(_, words)| words), Some(4));
}
