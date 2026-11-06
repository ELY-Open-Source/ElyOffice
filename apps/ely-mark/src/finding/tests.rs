use ely_gpui_component::forms::TextInput;
use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};

use crate::desk::{
    Desk,
    tests::{open, start},
};

fn opened(
    text: &str,
    cx: &mut TestAppContext,
) -> (
    Entity<Desk>,
    &'static mut VisualTestContext,
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
    (desk, cx, [data, files])
}

fn query(desk: &Entity<Desk>, words: &str, cx: &mut VisualTestContext) {
    desk.update_in(cx, |desk, window, cx| {
        desk.on_find(&ely_office::FindReplace, window, cx)
    });
    cx.simulate_input(words);
    cx.run_until_parked();
}

fn current_text(desk: &Entity<Desk>, cx: &mut VisualTestContext) -> String {
    desk.read_with(cx, |desk, cx| {
        let view = desk.active_view().expect("a tab");
        view.read(cx).document().read(cx).text(cx)
    })
}

#[gpui::test]
fn every_match_is_counted_and_replaced_in_one_step(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("a b a\n", cx);
    query(&desk, "a", cx);
    assert_eq!(
        desk.read_with(cx, |desk, _| desk
            .finding
            .as_ref()
            .map(|find| find.hits.len())),
        Some(2)
    );
    desk.update_in(cx, |desk, window, cx| {
        let field = cx.new(|cx| TextInput::new(window, cx));
        field.update(cx, |field, cx| field.set_text("x", cx));
        desk.finding.as_mut().expect("open").replace = Some(field);
        desk.replace(true, window, cx);
    });
    cx.run_until_parked();
    assert_eq!(current_text(&desk, cx), "x b x\n");
    let focus = desk.read_with(cx, |desk, cx| {
        desk.active_view().expect("a tab").read(cx).focus.clone()
    });
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(
        current_text(&desk, cx),
        "a b a\n",
        "one undo takes all back"
    );
}

#[gpui::test]
fn a_bad_pattern_says_why(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("a\n", cx);
    desk.update_in(cx, |desk, window, cx| {
        desk.on_find(&ely_office::FindReplace, window, cx)
    });
    desk.update(cx, |desk, cx| {
        desk.finding.as_mut().expect("open").options.regex = true;
        desk.count(cx);
    });
    cx.simulate_input("(");
    cx.run_until_parked();
    let error = desk.read_with(cx, |desk, _| {
        desk.finding.as_ref().and_then(|find| find.error.clone())
    });
    assert!(error.is_some());
}

#[gpui::test]
fn a_step_shows_the_next_match(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("one\n\ntwo one\n", cx);
    query(&desk, "one", cx);
    desk.update_in(cx, |desk, window, cx| {
        desk.step(true, window, cx);
        desk.step(true, window, cx);
    });
    let current = desk.read_with(cx, |desk, _| {
        desk.finding.as_ref().and_then(|find| find.current)
    });
    assert_eq!(current, Some(1));
}

/// Enter in find steps on, editing nothing (#60).
#[gpui::test]
fn enter_steps_through_matches_and_escape_lands_in_one(cx: &mut TestAppContext) {
    let source = "one\n\ntwo one\n\nthree one\n";
    let (desk, cx, _files) = opened(source, cx);
    query(&desk, "one", cx);
    let at = |desk: &Entity<Desk>, cx: &mut VisualTestContext| {
        desk.read_with(cx, |desk, _| desk.finding.as_ref().and_then(|find| find.current))
    };
    for want in [0, 1, 2, 0] {
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        assert_eq!(at(&desk, cx), Some(want), "Enter steps on");
    }
    assert_eq!(current_text(&desk, cx), source, "Enter edits nothing");
    cx.simulate_keystrokes("enter escape");
    cx.run_until_parked();
    assert!(desk.read_with(cx, |desk, _| desk.finding.is_none()), "Escape closes");
    cx.simulate_input("1");
    cx.run_until_parked();
    assert_eq!(current_text(&desk, cx), "one\n\ntwo 1\n\nthree one\n", "typing takes the match");
}

#[gpui::test]
fn the_table_menu_inserts_and_edits_a_table(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("Intro\n", cx);
    let focus = desk.read_with(cx, |desk, _| desk.core.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.dispatch_action(crate::tools::InsertTable);
    assert!(current_text(&desk, cx).contains("| ---- | ---- | ---- |"));
    cx.dispatch_action(crate::tools::DeleteColumn);
    cx.run_until_parked();
    assert!(
        current_text(&desk, cx).contains("| ---- | ---- |\n"),
        "{}",
        current_text(&desk, cx)
    );
}

fn replace_one(desk: &Entity<Desk>, with: &str, cx: &mut VisualTestContext) {
    desk.update_in(cx, |desk, window, cx| {
        let field = cx.new(|cx| TextInput::new(window, cx));
        field.update(cx, |field, cx| field.set_text(with, cx));
        desk.finding.as_mut().expect("open").replace = Some(field);
        desk.replace(false, window, cx);
    });
    cx.run_until_parked();
}

fn current_hit(desk: &Entity<Desk>, cx: &mut VisualTestContext) -> Option<std::ops::Range<usize>> {
    desk.read_with(cx, |desk, _| {
        let find = desk.finding.as_ref()?;
        find.current.map(|at| find.hits[at].clone())
    })
}

#[gpui::test]
fn a_replacement_that_still_matches_moves_on(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("a a\n", cx);
    query(&desk, "a", cx);
    desk.update_in(cx, |desk, window, cx| desk.step(true, window, cx));
    replace_one(&desk, "aa", cx);
    assert_eq!(current_text(&desk, cx), "aa a\n");
    assert_eq!(
        current_hit(&desk, cx),
        Some(3..4),
        "the next original match"
    );
}

#[gpui::test]
fn replacing_the_last_match_wraps_to_the_first(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("a b a\n", cx);
    query(&desk, "a", cx);
    desk.update_in(cx, |desk, window, cx| desk.step(false, window, cx));
    replace_one(&desk, "x", cx);
    assert_eq!(current_text(&desk, cx), "a b x\n");
    assert_eq!(current_hit(&desk, cx), Some(0..1));
}

#[gpui::test]
fn preview_keeps_its_text_from_tables_and_replacement(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("a\n", cx);
    let view = desk.read_with(cx, |desk, _| desk.active_view().expect("a tab"));
    view.update_in(cx, |view, window, cx| {
        view.switch(crate::view::Mode::Preview, window, cx)
    });
    let focus = desk.read_with(cx, |desk, _| desk.core.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.dispatch_action(crate::tools::InsertTable);
    query(&desk, "a", cx);
    desk.update_in(cx, |desk, window, cx| desk.step(true, window, cx));
    replace_one(&desk, "x", cx);
    assert_eq!(current_text(&desk, cx), "a\n");
}

#[gpui::test]
fn clipboard_keys_in_find_reach_the_field(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("alpha beta\n", cx);
    query(&desk, "beta", cx);
    let field = desk.read_with(cx, |desk, _| {
        desk.finding.as_ref().expect("open").query.clone()
    });
    field.update(cx, |field, cx| field.select(0..4, cx));
    cx.simulate_keystrokes("secondary-shift-c");
    cx.run_until_parked();
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert_eq!(copied.as_deref(), Some("beta"));
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("alpha".into()));
    cx.simulate_keystrokes("secondary-shift-v");
    cx.run_until_parked();
    assert_eq!(
        field.read_with(cx, |field, _| field.text().to_string()),
        "alpha"
    );
    assert_eq!(current_text(&desk, cx), "alpha beta\n");
}
