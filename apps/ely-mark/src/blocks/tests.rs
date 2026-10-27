use ely_gpui_component::documents::MarkdownDocument;
#[cfg(target_os = "windows")]
use gpui::Action;
use gpui::{
    ClipboardItem, Entity, Focusable as _, Menu, MenuItem, TestAppContext, VisualTestContext,
};

use crate::{
    desk::{
        Desk,
        tests::{open, start},
    },
    view::Mode,
};

pub(super) fn opened(
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
    let focus = desk.read_with(cx, |desk, _| desk.core.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    (desk, cx, [data, files])
}

pub(super) fn document(
    desk: &Entity<Desk>,
    cx: &mut VisualTestContext,
) -> Entity<MarkdownDocument> {
    desk.read_with(cx, |desk, cx| {
        let view = desk.active_view().expect("a tab");
        view.read(cx).document().clone()
    })
}

pub(super) fn current(desk: &Entity<Desk>, cx: &mut VisualTestContext) -> String {
    let document = document(desk, cx);
    document.read_with(cx, |document, cx| document.text(cx))
}

fn mode(desk: &Entity<Desk>, cx: &mut VisualTestContext) -> Mode {
    desk.read_with(cx, |desk, cx| {
        desk.active_view().expect("a tab").read(cx).mode
    })
}

/// `keys`, then the background reads.
pub(super) fn press(keys: &str, cx: &mut VisualTestContext) {
    cx.simulate_keystrokes(keys);
    cx.run_until_parked();
}

/// The window's toasts that hold `words`.
pub(super) fn toasted(desk: &Entity<Desk>, words: &str, cx: &mut VisualTestContext) -> usize {
    let root = desk.read_with(cx, |desk, _| desk.core.root.upgrade().expect("the window"));
    let toasts = root.read_with(cx, |root, cx| root.toasts(cx));
    toasts
        .iter()
        .filter(|(said, _)| said.contains(words))
        .count()
}

#[gpui::test]
fn paragraph_keys_retype_the_caret_block(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("Intro\n", cx);
    press("secondary-1", cx);
    assert_eq!(current(&desk, cx), "# Intro\n");
    press("secondary--", cx);
    assert_eq!(current(&desk, cx), "## Intro\n");
    press("secondary-=", cx);
    assert_eq!(current(&desk, cx), "# Intro\n");
    press("secondary-0", cx);
    assert_eq!(current(&desk, cx), "Intro\n");
    press("secondary-{", cx);
    assert_eq!(current(&desk, cx), "1. Intro\n");
    press("secondary-}", cx);
    assert_eq!(current(&desk, cx), "- Intro\n");
    press("secondary-shift-q", cx);
    assert_eq!(
        current(&desk, cx),
        "- > Intro\n",
        "the item's text is quoted"
    );
}

#[gpui::test]
fn format_keys_wrap_the_caret_word(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("word\n", cx);
    press("secondary-b", cx);
    assert_eq!(current(&desk, cx), "**word**\n");
    press("secondary-\\", cx);
    assert_eq!(current(&desk, cx), "word\n");
    press("alt-%", cx);
    assert_eq!(current(&desk, cx), "~~word~~\n");
}

#[gpui::test]
fn ctrl_slash_toggles_source_inside_the_code_editor(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("a\n", cx);
    press("secondary-/", cx);
    assert_eq!(mode(&desk, cx), Mode::Source);
    let editor = document(&desk, cx).read_with(cx, |document, _| document.editor().clone());
    let focus = editor.read_with(cx, |editor, cx| editor.focus_handle(cx));
    cx.update(|window, cx| window.focus(&focus, cx));
    press("secondary-/", cx);
    assert_eq!(mode(&desk, cx), Mode::Visual);
    assert_eq!(current(&desk, cx), "a\n", "no comment marks");
}

#[gpui::test]
fn a_refused_command_says_why(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("```\ncode\n```\n", cx);
    press("secondary-1", cx);
    assert_eq!(current(&desk, cx), "```\ncode\n```\n");
    assert_eq!(toasted(&desk, "inside code", cx), 1);
}

#[gpui::test]
fn alt_arrows_move_a_table_row(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("| A | B |\n| - | - |\n| 1 | 2 |\n", cx);
    press("secondary-/", cx);
    document(&desk, cx).update(cx, |document, cx| document.set_caret(2, 2, cx));
    press("alt-up", cx);
    assert_eq!(
        current(&desk, cx),
        "| 1    | 2    |\n| ---- | ---- |\n| A    | B    |\n"
    );
    press("alt-up", cx);
    assert_eq!(toasted(&desk, "Nothing lies past", cx), 1);
}

#[gpui::test]
fn view_keys_follow_typora(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("a\n", cx);
    press("f8 f9 secondary-+", cx);
    let look = desk.read_with(cx, |desk, _| desk.look.clone());
    assert!(look.focus && look.typewriter && look.size > 100, "{look:?}");
    press("secondary-)", cx);
    assert_eq!(desk.read_with(cx, |desk, _| desk.look.size), 100);
}

#[gpui::test]
fn copy_as_markdown_and_paste_as_plain_text(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("a **b**\n", cx);
    press("secondary-shift-c", cx);
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert_eq!(copied.as_deref(), Some("a **b**\n"));
    cx.write_to_clipboard(ClipboardItem::new_string("*x*".into()));
    press("secondary-/", cx);
    press("secondary-shift-v", cx);
    assert_eq!(current(&desk, cx), "*x*a **b**\n");
}

/// Row names; `-` a line, `▸` a submenu.
fn rows(menu: &Menu) -> Vec<String> {
    let mut out = Vec::new();
    for item in &menu.items {
        match item {
            MenuItem::Separator => out.push("-".into()),
            MenuItem::Submenu(sub) => {
                out.push(format!("▸ {}", sub.name));
                out.extend(rows(sub).into_iter().map(|row| format!("  {row}")));
            }
            MenuItem::Action { name, .. } => out.push(name.to_string()),
            MenuItem::SystemMenu(os) => out.push(os.name.to_string()),
        }
    }
    out
}

#[gpui::test]
fn the_paragraph_and_format_menus_follow_typora(cx: &mut TestAppContext) {
    let (desk, cx, _files) = opened("a\n", cx);
    let menus = desk.read_with(cx, |desk, cx| crate::keys::menus(&desk.look, cx));
    let named: Vec<String> = menus.iter().map(|menu| menu.name.to_string()).collect();
    assert_eq!(named, ["Edit", "Paragraph", "Format", "View"]);
    let table = [
        "Insert Table",
        "-",
        "Add Row Above",
        "Add Row Below",
        "Add Column Before",
        "Add Column After",
        "-",
        "Move Row Up",
        "Move Row Down",
        "Move Column Left",
        "Move Column Right",
        "-",
        "Delete Row",
        "Delete Column",
        "-",
        "Prettify Source Code",
        "-",
        "Delete Table",
    ];
    let mut paragraph: Vec<String> = [
        "Heading 1",
        "Heading 2",
        "Heading 3",
        "Heading 4",
        "Heading 5",
        "Heading 6",
        "-",
        "Paragraph",
        "-",
        "Increase Heading Level",
        "Decrease Heading Level",
        "-",
        "▸ Table",
    ]
    .map(String::from)
    .into();
    paragraph.extend(table.map(|row| format!("  {row}")));
    paragraph.extend(
        [
            "Math Block",
            "Code Fences",
            "-",
            "Quote",
            "-",
            "Ordered List",
            "Unordered List",
            "Task List",
            "-",
            "Insert Paragraph Before",
            "Insert Paragraph After",
            "-",
            "Link Reference",
            "Footnotes",
            "-",
            "Horizontal Line",
            "Table of Contents",
            "YAML Front Matter",
        ]
        .map(String::from),
    );
    assert_eq!(rows(&menus[1]), paragraph);
    assert_eq!(
        rows(&menus[2]).join("|"),
        "Strong|Emphasis|Underline|Code|-|Strike|-|Hyperlink|Insert Image|-|Clear Format"
    );
}

/// The keys a menu row shows for `action`.
#[cfg(target_os = "windows")]
fn keys(action: &dyn Action, cx: &mut VisualTestContext) -> String {
    cx.update(|window, cx| {
        let binding = window
            .highest_precedence_binding_for_action(action)
            .expect("a binding");
        let strokes: Vec<String> = binding
            .keystrokes()
            .iter()
            .map(|key| key.unparse(cx.platform_info().os))
            .collect();
        strokes.join(" ")
    })
}

#[cfg(target_os = "windows")]
#[gpui::test]
fn every_command_takes_typoras_windows_key(cx: &mut TestAppContext) {
    use crate::{blocks::*, clip::*, keys::ToggleSource, look, tools::*};
    let (_desk, cx, _files) = opened("a\n", cx);
    let typora: [(&dyn Action, &str); 34] = [
        (&Heading1, "ctrl-1"),
        (&Heading6, "ctrl-6"),
        (&Paragraph, "ctrl-0"),
        (&Promote, "ctrl-="),
        (&Demote, "ctrl--"),
        (&MathBlock, "ctrl-shift-m"),
        (&CodeFences, "ctrl-shift-k"),
        (&Quote, "ctrl-shift-q"),
        (&OrderedList, "ctrl-{"),
        (&BulletList, "ctrl-}"),
        (&TaskList, "ctrl-shift-x"),
        (&Strong, "ctrl-b"),
        (&Emphasis, "ctrl-i"),
        (&Underline, "ctrl-u"),
        (&InlineCode, "ctrl-~"),
        (&Strike, "alt-%"),
        (&Hyperlink, "ctrl-k"),
        (&InsertImage, "ctrl-shift-i"),
        (&ClearFormat, "ctrl-\\"),
        (&InsertTable, "ctrl-t"),
        (&RowBelow, "ctrl-enter"),
        (&DeleteRow, "ctrl-shift-backspace"),
        (&MoveRowUp, "alt-up"),
        (&MoveRowDown, "alt-down"),
        (&MoveColumnLeft, "alt-left"),
        (&MoveColumnRight, "alt-right"),
        (&ToggleSource, "ctrl-/"),
        (&look::FocusMode, "f8"),
        (&look::Typewriter, "f9"),
        (&look::Larger, "ctrl-+"),
        (&look::Smaller, "ctrl-_"),
        (&look::ActualSize, "ctrl-("),
        (&CopyMarkdown, "ctrl-shift-c"),
        (&PastePlain, "ctrl-shift-v"),
    ];
    for (action, want) in typora {
        assert_eq!(keys(action, cx), want, "{}", action.name());
    }
}
