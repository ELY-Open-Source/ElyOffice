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
    for _ in 0..200 {
        cx.run_until_parked();
        if let Ok(answer) = answered.try_recv() {
            return answer;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("no answer to {words:?}");
}

#[test]
fn the_words_carry_the_file_made_absolute() {
    let sent = super::request(&["replace", "a.md", "cat", "dog"]).expect("a request");
    assert_eq!((sent[0].as_str(), sent[2].as_str(), sent[3].as_str()), ("replace", "cat", "dog"));
    assert!(std::path::Path::new(&sent[1]).is_absolute(), "{sent:?}");
    assert!(super::request(&["replace", "a.md", "cat"]).is_err(), "too few words");
    assert!(super::request(&["set", "a.md"]).is_err(), "no such verb");
}

#[gpui::test]
fn text_reads_typing_replaces_in_one_step_and_saves(cx: &mut TestAppContext) {
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let path = files.path().join("a.md");
    std::fs::write(&path, "# Cat\n\ncat Cat 世界 😀 cat\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let file = path.display().to_string();
    let listed = ask(&desk, &["docs"], cx).expect("docs");
    assert_eq!(dunce::canonicalize(&listed).ok(), dunce::canonicalize(&path).ok(), "{listed}");
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
    let saved = ask(&desk, &["save", &file], cx).expect("saved");
    assert_eq!(dunce::canonicalize(&saved).ok(), dunce::canonicalize(&path).ok());
    assert_eq!(std::fs::read_to_string(&path).expect("on disk"), after, "on disk at the answer");
    desk.update_in(cx, |desk, window, cx| desk.switch(Mode::Preview, window, cx));
    cx.run_until_parked();
    let refused = ask(&desk, &["replace", &file, "dog", "cat"], cx);
    assert!(refused.as_ref().is_err_and(|why| why.contains("preview")), "{refused:?}");
    assert_eq!(ask(&desk, &["get", &file], cx).as_deref(), Ok(after.as_str()), "preview reads");
}
