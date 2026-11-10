use ely_gpui_component::{
    git::{DiffLayout, DiffViewer},
    typography::SelectableText,
};
use gpui::{
    Context, InteractiveElement, IntoElement, Modifiers, MouseButton, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestAppContext,
    VisualTestContext, Window, div, point, px,
};

struct Page {
    layout: Option<DiffLayout>,
    picks: Vec<((usize, usize), bool)>,
}

impl Render for Page {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.layout {
            Some(layout) => {
                let this = cx.entity();
                DiffViewer::new("select-diff", "a.txt", "old\n", "你好🙂\nsecond\n")
                    .layout(layout)
                    .headless()
                    .on_line(move |place, shift, _, cx| {
                        this.update(cx, |s, _| s.picks.push((place, shift)))
                    })
                    .into_any_element()
            }
            None => div().id("label-row")
                .on_click(cx.listener(|page, _, _, _| page.picks.push(((0, 0), false))))
                .child(SelectableText::new("message", "你好🙂\nsecond"))
                .into_any_element(),
        };
        div().w(px(640.)).h(px(400.)).child(content)
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, _| w.refresh());
    cx.run_until_parked();
}

fn copy(cx: &mut VisualTestContext) -> String {
    cx.simulate_keystrokes(ely_gpui_component::secondary!("c"));
    cx.update(|_, cx| {
        cx.read_from_clipboard()
            .and_then(|item| item.text())
            .unwrap_or_default()
    })
}

#[test]
fn text_drag_copies_unicode_across_lines_without_staging() {
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let (page, cx) = cx.add_window_view(|_, _| Page {
        layout: Some(DiffLayout::Unified),
        picks: vec![],
    });
    cx.update(|w, _| w.activate_window());
    settle(cx);
    let first = cx
        .debug_bounds("selection-text-2")
        .expect("first added line");
    let last = cx
        .debug_bounds("selection-text-3")
        .expect("second added line");
    let start = point(first.left() + px(1.), first.center().y);
    let end = point(last.right() + px(20.), last.center().y);
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    settle(cx);
    cx.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    settle(cx);
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    settle(cx);
    assert_eq!(copy(cx), "你好🙂\nsecond");
    assert!(page.read_with(cx, |p, _| p.picks.is_empty()));
    cx.simulate_mouse_down(
        point(first.left() + px(20.), first.center().y),
        MouseButton::Left,
        Modifiers::default(),
    );
    settle(cx);
    let gutter = point(first.left() - px(5.), first.center().y);
    cx.simulate_mouse_move(gutter, MouseButton::Left, Modifiers::default());
    settle(cx);
    cx.simulate_mouse_up(gutter, MouseButton::Left, Modifiers::default());
    settle(cx);
    assert!(
        page.read_with(cx, |p, _| p.picks.is_empty()),
        "a drag to the gutter is not a row click"
    );

    cx.simulate_click(start, Modifiers::default());
    settle(cx);
    cx.simulate_click(
        point(last.left() + px(1.), last.center().y),
        Modifiers {
            shift: true,
            ..Default::default()
        },
    );
    settle(cx);
    assert_eq!(
        page.read_with(cx, |p, _| p.picks.clone()),
        vec![((0, 1), false), ((0, 2), true)]
    );
}

#[test]
fn quick_drag_before_repaint_still_selects_without_staging() {
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let (page, cx) = cx.add_window_view(|_, _| Page { layout: Some(DiffLayout::Unified), picks: vec![] });
    cx.update(|w, _| w.activate_window());
    settle(cx);
    let first = cx.debug_bounds("selection-text-2").expect("first added line");
    let last = cx.debug_bounds("selection-text-3").expect("last added line");
    let start = point(first.left() + px(1.), first.center().y);
    let end = point(last.right() + px(20.), last.center().y);
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    settle(cx);
    assert_eq!(copy(cx), "你好🙂\nsecond");
    assert!(page.read_with(cx, |p, _| p.picks.is_empty()));
}

#[test]
fn split_copy_stays_on_the_chosen_side() {
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let (_, cx) = cx.add_window_view(|_, _| Page {
        layout: Some(DiffLayout::Split),
        picks: vec![],
    });
    cx.update(|w, _| w.activate_window());
    settle(cx);
    let first = cx
        .debug_bounds("selection-text-1")
        .expect("right first line");
    cx.simulate_click(
        point(first.left() + px(1.), first.center().y),
        Modifiers::default(),
    );
    settle(cx);
    cx.simulate_keystrokes(ely_gpui_component::secondary!("a"));
    settle(cx);
    assert_eq!(copy(cx), "你好🙂\nsecond");
}

#[test]
fn plain_text_copies_and_does_not_edit() {
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let (page, cx) = cx.add_window_view(|_, _| Page {
        layout: None,
        picks: vec![],
    });
    cx.update(|w, _| w.activate_window());
    settle(cx);
    let bounds = cx.debug_bounds("selection-text-0").expect("text");
    let start = point(bounds.left(), bounds.top() + px(2.));
    let end = point(bounds.right(), bounds.bottom() - px(2.));
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    assert_eq!(copy(cx), "你好🙂\nsecond");
    assert!(page.read_with(cx, |page, _| page.picks.is_empty()));
    cx.simulate_click(bounds.center(), Modifiers::default());
    assert_eq!(page.read_with(cx, |page, _| page.picks.len()), 1);
    cx.simulate_keystrokes(ely_gpui_component::secondary!("a"));
    cx.simulate_keystrokes("backspace");
    assert_eq!(copy(cx), "你好🙂\nsecond");
    cx.simulate_click(
        point(bounds.left() + px(2.), bounds.top() + px(2.)),
        Modifiers::default(),
    );
    settle(cx);
    cx.simulate_click(
        point(bounds.right() - px(2.), bounds.bottom() - px(2.)),
        Modifiers {
            shift: true,
            ..Default::default()
        },
    );
    settle(cx);
    assert_eq!(
        copy(cx),
        "你好🙂\nsecond",
        "Shift extends the plain text selection"
    );
}

struct LongPage;

struct Labels;

const LONG_LABEL: &str = "很长的提交标题🙂 abcdefghijklmnopqrstuvwxyz";

impl Render for Labels {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        use gpui::InteractiveElement;
        div().w(px(400.)).h(px(400.)).child(
            div()
                .w(px(100.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .debug_selector(|| "label-first".into())
                        .child(SelectableText::new("first", LONG_LABEL).ellipsis()),
                )
                .child(
                    div()
                        .debug_selector(|| "label-second".into())
                        .child(SelectableText::new("second", "second label")),
                ),
        )
    }
}

#[test]
fn ellipsis_keeps_full_copy_and_outside_click_clears_selection() {
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let (_, cx) = cx.add_window_view(|_, _| Labels);
    cx.update(|w, _| w.activate_window());
    settle(cx);
    let elided = cx
        .debug_bounds("selection-ellipsis")
        .expect("ellipsis at rest");
    assert!(elided.size.width <= px(100.));
    let first = cx.debug_bounds("label-first").expect("first label");
    cx.simulate_click(
        point(first.left() + px(10.), first.center().y),
        Modifiers::default(),
    );
    cx.simulate_keystrokes(ely_gpui_component::secondary!("a"));
    settle(cx);
    assert_eq!(copy(cx), LONG_LABEL);
    assert!(
        cx.debug_bounds("selection-ellipsis").is_none(),
        "selected text replaces the elided display"
    );

    cx.simulate_click(point(px(300.), first.center().y), Modifiers::default());
    settle(cx);
    assert!(
        cx.debug_bounds("selection-ellipsis").is_some(),
        "clipped text beyond the viewport is outside"
    );
    cx.simulate_click(first.center(), Modifiers::default());
    cx.simulate_keystrokes(ely_gpui_component::secondary!("a"));
    settle(cx);
    cx.simulate_click(point(px(300.), px(300.)), Modifiers::default());
    settle(cx);
    assert!(
        cx.debug_bounds("selection-ellipsis").is_some(),
        "blank click clears highlighting"
    );
    cx.update(|_, app| app.write_to_clipboard(gpui::ClipboardItem::new_string("sentinel".into())));
    assert_eq!(
        copy(cx),
        "sentinel",
        "cleared selection cannot copy stale text"
    );

    cx.simulate_click(first.center(), Modifiers::default());
    cx.simulate_keystrokes(ely_gpui_component::secondary!("a"));
    settle(cx);
    let second = cx.debug_bounds("label-second").expect("second label");
    cx.simulate_click(second.center(), Modifiers::default());
    settle(cx);
    assert!(
        cx.debug_bounds("selection-ellipsis").is_some(),
        "another text field clears the first"
    );
}

impl Render for LongPage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let text = (0..100)
            .map(|n| format!("line {n:03}\n"))
            .collect::<String>();
        div()
            .w(px(640.))
            .h(px(200.))
            .child(DiffViewer::new("long", "a.txt", "", &text).headless())
    }
}

#[test]
fn held_drag_scrolls_beyond_visible_lines() {
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let (_, cx) = cx.add_window_view(|_, _| LongPage);
    cx.update(|w, _| w.activate_window());
    settle(cx);
    let first = cx.debug_bounds("selection-text-1").expect("first line");
    cx.simulate_mouse_down(
        point(first.left() + px(1.), first.center().y),
        MouseButton::Left,
        Modifiers::default(),
    );
    settle(cx);
    let end = point(first.left() + px(80.), px(210.));
    cx.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    for _ in 0..20 {
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(50));
        settle(cx);
    }
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    settle(cx);
    let copied = copy(cx);
    assert!(copied.starts_with("line 000\n"), "{copied:?}");
    assert!(
        copied.lines().count() > 15,
        "drag should scroll: {copied:?}"
    );
}

struct Blamed;

struct SparsePage;

impl Render for SparsePage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let middle = (0..40).map(|n| format!("new {n}\n")).collect::<String>();
        div().w(px(640.)).h(px(200.)).child(
            DiffViewer::new(
                "sparse",
                "a.txt",
                "first\nlast\n",
                &format!("first\n{middle}last\n"),
            )
            .headless()
            .layout(DiffLayout::Split),
        )
    }
}

#[test]
fn split_drag_scrolls_past_empty_cells() {
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let (_, cx) = cx.add_window_view(|_, _| SparsePage);
    cx.update(|w, _| w.activate_window());
    settle(cx);
    let first = cx.debug_bounds("diff-text-left-1").expect("first row");
    let start = point(first.left() + px(1.), first.center().y);
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    settle(cx);
    let end = point(start.x + px(60.), px(210.));
    cx.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    for _ in 0..60 {
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(50));
        settle(cx);
    }
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    settle(cx);
    assert_eq!(copy(cx), "first\nlast");
}

impl Render for Blamed {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        use ely_gpui_component::git::BlameView;
        div().w(px(640.)).h(px(200.)).child(BlameView::new(
            "blamed",
            "let x = 1;\n你好🙂",
            [0, 0],
            [],
        ))
    }
}

#[test]
fn blame_selection_keeps_syntax_and_copies_code() {
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let (_, cx) = cx.add_window_view(|_, _| Blamed);
    cx.update(|w, _| w.activate_window());
    settle(cx);
    let first = cx.debug_bounds("selection-text-0").expect("code");
    cx.simulate_click(first.center(), Modifiers::default());
    cx.simulate_keystrokes(ely_gpui_component::secondary!("a"));
    settle(cx);
    assert_eq!(copy(cx), "let x = 1;\n你好🙂");
}
