//! Another program reads, replaces and saves the Markdown.

use gpui::{Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};

use crate::desk::{
    Desk,
    tests::{open, start},
};
use crate::view::Mode;

/// The answer to `words`, once it comes.
fn ask(desk: &Entity<Desk>, words: &[&str], cx: &mut VisualTestContext) -> Result<String, String> {
    let (ask, answered) = ely_ui::Ask::for_tests(&serde_json::json!(words).to_string());
    desk.update(cx, |desk, cx| desk.answer_request(ask, cx));
    for _ in 0..6000 {
        cx.run_until_parked();
        if let Ok(answer) = answered.try_recv() {
            return answer;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("no answer to {words:?}");
}

/// The files `docs` lists, parsed as JSON.
fn docs(desk: &Entity<Desk>, cx: &mut VisualTestContext) -> Vec<String> {
    serde_json::from_str(&ask(desk, &["docs"], cx).expect("docs")).expect("a JSON list")
}

/// A path answered as JSON, parsed.
fn path_of(answer: Result<String, String>) -> String {
    serde_json::from_str(&answer.expect("a path")).expect("a JSON path")
}

#[test]
fn the_words_carry_the_file_made_absolute() {
    let sent = super::request(&["replace", "a.md", "cat", "dog"]).expect("a request");
    assert_eq!((sent[0].as_str(), sent[2].as_str(), sent[3].as_str()), ("replace", "cat", "dog"));
    assert!(std::path::Path::new(&sent[1]).is_absolute(), "{sent:?}");
    assert!(super::request(&["replace", "a.md", "cat"]).is_err(), "too few words");
    assert!(super::request(&["set", "a.md"]).is_err(), "too few words");
    assert!(super::request(&["cut", "a.md"]).is_err(), "no such verb");
    assert_eq!(super::request(&["append", "a.md", ""]), Err(super::APPEND_EMPTY.into()));
    assert_eq!(super::request(&["set", "a.md", ""]).expect("empties")[2], "");
}

#[gpui::test]
fn text_reads_typing_replaces_in_one_step_and_saves(cx: &mut TestAppContext) {
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let path = files.path().join("a.md");
    std::fs::write(&path, "# Cat\n\ncat Cat 世界 😀 cat\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let file = path.display().to_string();
    let listed = docs(&desk, cx);
    let canonical: Vec<_> = listed.iter().map(|file| dunce::canonicalize(file).ok()).collect();
    assert_eq!(canonical, [dunce::canonicalize(&path).ok()], "{listed:?}");
    assert_eq!(ask(&desk, &["get", &file], cx).as_deref(), Ok("# Cat\n\ncat Cat 世界 😀 cat\n"));
    // Typing in an open block, not yet written back.
    let block = cx.debug_bounds("markdown-block-1").expect("the paragraph is drawn");
    let at = point(block.left() + px(4.0), block.top() + px(4.0));
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    cx.simulate_input("!");
    let before = ask(&desk, &["get", &file], cx).expect("read");
    assert_eq!(before.matches('!').count(), 1, "unsaved typing shows: {before}");
    assert_eq!(ask(&desk, &["replace", &file, "cat", "dog"], cx), Ok(String::new()));
    let after = ask(&desk, &["get", &file], cx).expect("read");
    assert_eq!(after, before.replace("cat", "dog"), "every exact match, case kept");
    let missing = ask(&desk, &["replace", &file, "zebra", "x"], cx);
    assert!(missing.as_ref().is_err_and(|why| why.starts_with("nothing replaced")), "{missing:?}");
    assert!(ask(&desk, &["replace", &file, "", "x"], cx).is_err(), "an empty FIND");
    let other = files.path().join("other.md");
    std::fs::write(&other, "x").expect("a stray file");
    assert!(ask(&desk, &["get", &other.display().to_string()], cx).is_err(), "a file not open");
    let document = desk.read_with(cx, |desk, cx| desk.active_view().expect("a tab").read(cx).document().clone());
    document.update(cx, |document, cx| document.editor().update(cx, |editor, cx| editor.undo(cx)));
    cx.run_until_parked();
    assert_eq!(ask(&desk, &["get", &file], cx).as_deref(), Ok(before.as_str()), "one undo step");
    document.update(cx, |document, cx| document.editor().update(cx, |editor, cx| editor.redo(cx)));
    cx.run_until_parked();
    let saved = path_of(ask(&desk, &["save", &file], cx));
    assert_eq!(dunce::canonicalize(&saved).ok(), dunce::canonicalize(&path).ok());
    assert_eq!(std::fs::read_to_string(&path).expect("on disk"), after, "on disk at the answer");
    desk.update_in(cx, |desk, window, cx| desk.switch(Mode::Preview, window, cx));
    cx.run_until_parked();
    let refused = ask(&desk, &["replace", &file, "dog", "cat"], cx);
    assert!(refused.as_ref().is_err_and(|why| why.contains("preview")), "{refused:?}");
    assert_eq!(ask(&desk, &["get", &file], cx).as_deref(), Ok(after.as_str()), "preview reads");
}

#[gpui::test]
fn set_and_append_are_one_step_each_and_keep_the_line_endings(cx: &mut TestAppContext) {
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let path = files.path().join("a.md");
    std::fs::write(&path, "# A\r\n\r\nbare").expect("a CRLF file, no final break");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let file = path.display().to_string();
    let get = |cx: &mut VisualTestContext| ask(&desk, &["get", &file], cx).expect("read");
    let step = |back: bool, cx: &mut VisualTestContext| {
        let document = desk.read_with(cx, |desk, cx| desk.active_view().expect("a tab").read(cx).document().clone());
        document.update(cx, |document, cx| document.editor().update(cx, |editor, cx| if back { editor.undo(cx) } else { editor.redo(cx) }));
        cx.run_until_parked();
    };
    assert_eq!(ask(&desk, &["append", &file, "- one\r\n- two"], cx), Ok(String::new()));
    assert_eq!(get(cx), "# A\n\nbare\n- one\n- two", "on a new line; CRLF read as LF");
    assert_eq!(ask(&desk, &["append", &file, "\nTail\n"], cx), Ok(String::new()));
    assert_eq!(get(cx), "# A\n\nbare\n- one\n- two\n\nTail\n", "a blank line first starts a paragraph");
    assert_eq!(ask(&desk, &["append", &file, "end"], cx), Ok(String::new()));
    assert_eq!(get(cx), "# A\n\nbare\n- one\n- two\n\nTail\nend", "no doubled break");
    step(true, cx);
    assert_eq!(get(cx), "# A\n\nbare\n- one\n- two\n\nTail\n", "one undo step");
    assert_eq!(ask(&desk, &["set", &file, "# New\r\rBody\n"], cx), Ok(String::new()));
    assert_eq!(get(cx), "# New\n\nBody\n", "lone CR read as LF");
    step(true, cx);
    assert_eq!(get(cx), "# A\n\nbare\n- one\n- two\n\nTail\n", "one undo step");
    step(false, cx);
    assert!(ask(&desk, &["save", &file], cx).is_ok(), "saved");
    assert_eq!(std::fs::read(&path).expect("on disk"), b"# New\r\n\r\nBody\r\n", "the file's CRLF kept");
    assert_eq!(ask(&desk, &["set", &file, ""], cx), Ok(String::new()));
    assert_eq!(get(cx), "", "empty clears it");
    assert_eq!(ask(&desk, &["append", &file, "x"], cx), Ok(String::new()));
    assert_eq!(get(cx), "x", "nothing to break from");
    desk.update_in(cx, |desk, window, cx| desk.switch(Mode::Preview, window, cx));
    cx.run_until_parked();
    for words in [&["set", &file, "y"][..], &["append", &file, "y"]] {
        let refused = ask(&desk, words, cx);
        assert!(refused.as_ref().is_err_and(|why| why.contains("preview")), "{refused:?}");
    }
    assert_eq!(get(cx), "x", "preview takes no edits");
}

#[gpui::test]
fn typing_goes_on_where_it_was_after_another_program_replaces(cx: &mut TestAppContext) {
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let path = files.path().join("a.md");
    std::fs::write(&path, "# Typing\n\nA paragraph.\n\ntoken0\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let file = path.display().to_string();
    let block = cx.debug_bounds("markdown-block-1").expect("the paragraph is drawn");
    let at = point(block.left() + px(4.0), block.top() + px(4.0));
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    cx.simulate_input("x");
    // A match after the caret, then one before it.
    for (round, (find, with)) in [("token0", "token1"), ("Typing", "Typed twice")].into_iter().enumerate() {
        assert_eq!(ask(&desk, &["replace", &file, find, with], cx), Ok(String::new()));
        cx.simulate_input("y");
        cx.run_until_parked();
        let text = ask(&desk, &["get", &file], cx).expect("read");
        let typed = format!("x{}", "y".repeat(round + 1));
        assert!(text.contains(&typed) && text.contains(with), "after {find}: {text:?}");
    }
}

#[gpui::test]
fn a_deleted_files_tab_still_reads_and_replaces(cx: &mut TestAppContext) {
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let path = files.path().join("gone.md");
    std::fs::write(&path, "cat\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let file = path.display().to_string();
    assert_eq!(ask(&desk, &["replace", &file, "cat", "dog"], cx), Ok(String::new()));
    std::fs::remove_file(&path).expect("deleted");
    // Time for the watcher to see the delete.
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(20));
        cx.run_until_parked();
    }
    assert_eq!(ask(&desk, &["get", &file], cx).as_deref(), Ok("dog\n"), "the tab keeps its text");
    assert_eq!(ask(&desk, &["replace", &file, "dog", "fox"], cx), Ok(String::new()));
    assert_eq!(ask(&desk, &["get", &file], cx).as_deref(), Ok("fox\n"));
    let refused = ask(&desk, &["save", &file], cx);
    assert!(refused.as_ref().is_err_and(|why| why.contains("moved or deleted")), "{refused:?}");
    assert!(!path.exists(), "nothing written back");
}

#[gpui::test]
fn open_loads_once_and_export_writes_a_copy(cx: &mut TestAppContext) {
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let path = files.path().join("a.md");
    std::fs::write(&path, "# Cat\n\nA *cat* sat.\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    let file = path.display().to_string();
    assert_eq!(path_of(ask(&desk, &["open", &file], cx)), file, "opened here");
    assert_eq!(path_of(ask(&desk, &["open", &file], cx)), file, "already open");
    let listed = docs(&desk, cx);
    assert_eq!(listed.len(), 1, "one tab: {listed:?}");
    let page = files.path().join("a.html");
    let named = page.display().to_string();
    assert_eq!(path_of(ask(&desk, &["export", &file, &named], cx)), named);
    let html = std::fs::read_to_string(&page).expect("the page");
    assert!(html.contains("<em>cat</em>"), "{html}");
    let plain = files.path().join("a.txt");
    let named = plain.display().to_string();
    assert_eq!(path_of(ask(&desk, &["export", &file, &named], cx)), named);
    let text = std::fs::read_to_string(&plain).expect("the text");
    assert!(text.contains("A cat sat.") && !text.contains('*'), "{text}");
    let over = ask(&desk, &["export", &file, &file], cx);
    assert!(over.as_ref().is_err_and(|why| why.contains("open file")), "{over:?}");
    assert_eq!(std::fs::read_to_string(&path).expect("kept"), "# Cat\n\nA *cat* sat.\n", "source untouched");
    let stray = files.path().join("b.md").display().to_string();
    assert!(ask(&desk, &["export", &stray, &named], cx).is_err(), "a file not open");
}
