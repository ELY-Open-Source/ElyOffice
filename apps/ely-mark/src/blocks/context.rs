//! The right-click menu, driven as a user would (R4.10).

use ely_gpui_component::theme::{ActiveTheme as _, TextSize};
use gpui::{
    ClipboardItem, Entity, Modifiers, MouseButton, Point, TestAppContext, VisualTestContext, point,
    px,
};

use super::tests::{current, document, opened, toasted};
use crate::{desk::Desk, view::Mode};

/// A right press on block `ix`'s first line, character `k`.
fn right_on(ix: usize, k: usize, cx: &mut VisualTestContext) {
    let block = cx
        .debug_bounds(Box::leak(format!("markdown-block-{ix}").into_boxed_str()))
        .expect("the block is drawn");
    let size = cx.update(|window, cx| {
        cx.theme()
            .text_size(TextSize::Base)
            .to_pixels(window.rem_size())
    });
    // The test text system sets glyphs 0.6 em apart.
    let x = block.left() + px(13.0) + size * 0.6 * (k as f32 + 0.25);
    right_at(point(x, block.top() + px(10.0)), cx);
}

fn right_at(at: Point<gpui::Pixels>, cx: &mut VisualTestContext) {
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::none());
    cx.run_until_parked();
}

/// Arrow `keys`, then Enter; menus choose on its release.
fn choose(keys: &str, cx: &mut VisualTestContext) {
    if !keys.is_empty() {
        cx.simulate_keystrokes(keys);
    }
    cx.simulate_keystrokes("enter");
    let enter = gpui::Keystroke::parse("enter").expect("a key");
    cx.simulate_event(gpui::KeyUpEvent { keystroke: enter });
    cx.run_until_parked();
}

fn clipboard(cx: &mut VisualTestContext) -> Option<String> {
    cx.read_from_clipboard().and_then(|item| item.text())
}

fn switch(desk: &Entity<Desk>, mode: Mode, cx: &mut VisualTestContext) {
    desk.update_in(cx, |desk, window, cx| desk.switch(mode, window, cx));
    cx.run_until_parked();
}

#[gpui::test]
fn paste_from_the_menu_lands_at_the_pressed_caret(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("Intro\n", cx);
    cx.write_to_clipboard(ClipboardItem::new_string("New ".into()));
    right_on(0, 0, cx);
    choose("down", cx);
    assert_eq!(current(&desk, cx), "New Intro\n");
}

#[gpui::test]
fn a_link_opens_and_copies_its_address(cx: &mut TestAppContext) {
    let (_desk, cx, _files) = opened("[docs](https://x.test)\n", cx);
    right_on(0, 1, cx);
    choose("down", cx);
    assert_eq!(cx.opened_url().as_deref(), Some("https://x.test"));
    right_on(0, 1, cx);
    choose("down down down", cx);
    assert_eq!(clipboard(cx).as_deref(), Some("https://x.test"));
}

#[gpui::test]
fn a_code_block_copies_its_code_and_deletes_by_name(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("```rust\nlet x = 1;\n```\n\nNext\n", cx);
    right_on(0, 0, cx);
    choose("down down down", cx);
    assert_eq!(clipboard(cx).as_deref(), Some("let x = 1;"));
    right_on(0, 0, cx);
    choose("up", cx);
    assert!(
        !current(&desk, cx).contains("let x"),
        "{}",
        current(&desk, cx)
    );
}

#[gpui::test]
fn a_quote_is_deleted_from_the_last_row(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("> quoted\n\nNext\n", cx);
    right_on(0, 1, cx);
    choose("up", cx);
    let text = current(&desk, cx);
    assert!(!text.contains("quoted") && text.contains("Next"), "{text}");
}

#[gpui::test]
fn a_picture_shows_its_file_or_goes(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("Lead.\n\n![pic](missing.png)\n", cx);
    right_on(1, 1, cx);
    choose("down", cx);
    assert_eq!(toasted(&desk, "no file here", cx), 1);
    right_on(1, 1, cx);
    choose("down down", cx);
    assert!(!current(&desk, cx).contains("!["), "{}", current(&desk, cx));
}

#[gpui::test]
fn source_cuts_the_selection_a_right_press_keeps(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("alpha beta\n", cx);
    switch(&desk, Mode::Source, cx);
    let editor = document(&desk, cx).read_with(cx, |document, _| document.editor().clone());
    editor.update(cx, |editor, cx| editor.select(std::iter::once(0..5), cx));
    let line = cx.debug_bounds("editor-line-0").expect("drawn");
    right_at(point(line.left() + px(20.0), line.top() + px(6.0)), cx);
    choose("down", cx);
    assert_eq!(current(&desk, cx), " beta\n");
    assert_eq!(clipboard(cx).as_deref(), Some("alpha"));
}

#[gpui::test]
fn an_open_block_copies_its_selection_and_stays(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("Intro words\n", cx);
    let block = cx.debug_bounds("markdown-block-0").expect("drawn");
    cx.simulate_mouse_down(block.center(), MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(block.center(), MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    cx.simulate_keystrokes("home shift-right shift-right shift-right shift-right shift-right");
    let field = cx.debug_bounds("markdown-open").expect("open");
    right_at(point(field.left() + px(20.0), field.top() + px(14.0)), cx);
    choose("down right", cx);
    assert_eq!(clipboard(cx).as_deref(), Some("Intro"));
    assert!(
        cx.debug_bounds("markdown-open").is_some(),
        "the block stays open"
    );
    assert_eq!(current(&desk, cx), "Intro words\n");
}

#[gpui::test]
fn preview_offers_copies_only(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("alpha **beta**\n", cx);
    switch(&desk, Mode::Preview, cx);
    let block = cx.debug_bounds("markdown-block-0").expect("drawn");
    right_at(block.center(), cx);
    choose("down", cx);
    assert_eq!(clipboard(cx).as_deref(), Some("alpha beta\n"));
    right_at(block.center(), cx);
    choose("down down", cx);
    assert_eq!(clipboard(cx).as_deref(), Some("alpha **beta**\n"));
}

#[gpui::test]
fn html_code_keeps_picture_addresses(cx: &mut TestAppContext) {
    let (_desk, cx, _files) = opened("![pic](missing.png) and *it*\n", cx);
    cx.dispatch_action(crate::export::CopyHtml);
    cx.run_until_parked();
    let html = clipboard(cx).expect("copied");
    assert!(
        html.contains("src=\"missing.png\"") && html.contains("<em>it</em>"),
        "{html}"
    );
}

#[gpui::test]
fn copies_convert_off_the_window_thread(cx: &mut TestAppContext) {
    let (_desk, cx, _files) = opened("alpha *it*\n", cx);
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("before".into()));
    // The context's dispatch parks; this one does not.
    cx.update(|window, cx| window.dispatch_action(Box::new(crate::export::CopyHtml), cx));
    assert_eq!(
        clipboard(cx).as_deref(),
        Some("before"),
        "not on the window thread"
    );
    cx.update(|window, cx| window.dispatch_action(Box::new(crate::export::CopyPlain), cx));
    cx.run_until_parked();
    assert_eq!(clipboard(cx).as_deref(), Some("alpha it\n"));
}

#[gpui::test]
fn a_later_markdown_copy_outlives_a_pending_conversion(cx: &mut TestAppContext) {
    let (_desk, cx, _files) = opened("alpha *it*\n", cx);
    cx.update(|window, cx| window.dispatch_action(Box::new(crate::export::CopyHtml), cx));
    cx.update(|window, cx| window.dispatch_action(Box::new(crate::clip::CopyMarkdown), cx));
    cx.run_until_parked();
    assert_eq!(clipboard(cx).as_deref(), Some("alpha *it*\n"));
}

#[gpui::test]
fn a_later_editor_copy_outlives_a_pending_conversion(cx: &mut TestAppContext) {
    use gpui::Focusable as _;
    let (desk, cx, _files) = opened("alpha *it*\n", cx);
    switch(&desk, Mode::Source, cx);
    let editor = document(&desk, cx).read_with(cx, |document, _| document.editor().clone());
    editor.update(cx, |editor, cx| editor.select(std::iter::once(0..5), cx));
    let focus = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.dispatch_action(Box::new(crate::export::CopyHtml), cx));
    cx.update(|window, cx| {
        let copy = gpui::Keystroke::parse_with_os("secondary-c", cx.platform_info().os).expect("key");
        window.dispatch_keystroke(copy, cx)
    });
    cx.run_until_parked();
    assert_eq!(clipboard(cx).as_deref(), Some("alpha"));
}

#[gpui::test]
fn a_later_rich_copy_with_the_same_text_outlives_a_pending_conversion(cx: &mut TestAppContext) {
    use ely_platform::clipboard::{self, Clip};
    if std::env::var_os("ELY_CLIPBOARD_TEST").is_none() {
        eprintln!("SKIP: set ELY_CLIPBOARD_TEST=1; this replaces the clipboard");
        return;
    }
    let rich = |html: &str| Clip::Html {
        html: html.into(),
        text: "x".into(),
    };
    let (_desk, cx, _files) = opened("alpha *it*\n", cx);
    clipboard::write(&rich("<b>x</b>")).expect("writes");
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("x".into()));
    cx.update(|window, cx| window.dispatch_action(Box::new(crate::export::CopyHtml), cx));
    clipboard::write(&rich("<i>x</i>")).expect("writes");
    cx.run_until_parked();
    let html = clipboard::html().expect("reads");
    assert_eq!(
        clipboard(cx).as_deref(),
        Some("x"),
        "the conversion is dropped"
    );
    assert!(html.is_some_and(|html| html.contains("<i>x</i>")));
}

#[gpui::test]
fn a_shut_menu_stays_shut_when_its_tab_returns(cx: &mut TestAppContext) {
    let (desk, cx, files) = opened("alpha\n", cx);
    right_on(0, 1, cx);
    assert!(cx.debug_bounds("menu-button-0-0").is_some(), "opened");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds("menu-button-0-0").is_none(), "shut");
    let other = files[1].path().join("b.md");
    std::fs::write(&other, "beta\n").expect("b");
    crate::desk::tests::open(&desk, &other, cx);
    let first = desk.read_with(cx, |desk, _| desk.core.tabs[0].id);
    desk.update(cx, |desk, cx| {
        desk.core.active = Some(first);
        cx.notify();
    });
    cx.run_until_parked();
    assert!(cx.debug_bounds("menu-button-0-0").is_none(), "not replayed");
}
