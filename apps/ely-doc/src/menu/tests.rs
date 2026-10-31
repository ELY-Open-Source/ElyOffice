use ely_gpui_component::theme::Theme;
use ely_lok::{LokView, menu::Target};
use gpui::{Entity, Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext};

use super::Pick;
use crate::docs::Docs;
use crate::tests::find::{finder, total};
use crate::tests::{blank, docs, settle, settled, text_of, type_into, with_window, written};
use ely_office::{FindReplace, prelude::*};

/// The view with the menu's host over it.
struct Host(Entity<Docs>, Entity<LokView>);

impl gpui::Render for Host {
    fn render(&mut self, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};
        let menu = self.0.update(cx, |docs, cx| docs.context_menu(cx));
        gpui::div().relative().size_full().child(self.1.clone()).children(menu)
    }
}

fn frames(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

/// The caret's middle, in window pixels.
fn caret(cx: &mut VisualTestContext, view: &Entity<LokView>) -> Point<Pixels> {
    frames(cx);
    cx.update(|_, app| view.read(app).cursor_bounds().expect("a caret").center())
}

/// Right-clicks through the view and waits for the target's menu.
fn right_click(cx: &mut VisualTestContext, docs: &Entity<Docs>, at: Point<Pixels>, target: Target) {
    cx.update(|_, app| docs.update(app, |docs, _| docs.offer = None));
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::none());
    let shown = |app: &gpui::App| {
        docs.read(app)
            .offer
            .as_ref()
            .is_some_and(|offer| offer.what.target == target)
    };
    for _ in 0..500 {
        cx.run_until_parked();
        if cx.update(|_, app| shown(app)) {
            return frames(cx);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("no {target:?} menu");
}

#[test]
fn a_right_click_opens_word_rows_where_it_lands() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Text");
    let view = cx.update(|app| docs.read(app).active_tab().expect("a tab").view.clone());
    let window = cx.add_window(|_, _| Host(docs.clone(), view.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    let text = caret(&mut shown, &view) - gpui::point(gpui::px(8.0), gpui::px(0.0));
    right_click(&mut shown, &docs, text, Target::Text);
    let cut = shown.debug_bounds("menu-button-0-0").expect("the clipboard row");
    let off = cut.origin - text;
    assert!(
        off.x >= gpui::px(0.0) && off.x < gpui::px(40.0) && off.y < gpui::px(40.0),
        "the menu opens at the click, not the caret: {off:?}"
    );
    let on = |shown: &mut VisualTestContext, pick: Pick| {
        shown.update(|_, app| pick.on(&docs.read(app).offer.as_ref().expect("an offer").what))
    };
    assert!(on(&mut shown, Pick::Comment));
    assert!(!on(&mut shown, Pick::Link), "the kernel links a selection");
    assert!(!on(&mut shown, Pick::Cut), "nothing is selected");
    with_window(&mut cx, &docs, |docs, _, cx| docs.insert_table(2.0, 2.0, cx));
    settled(&cx, &docs);
    let cell = caret(&mut shown, &view);
    right_click(&mut shown, &docs, cell, Target::Table);
    assert!(on(&mut shown, Pick::RowBelow) && on(&mut shown, Pick::SplitRows));
    assert!(!on(&mut shown, Pick::Merge), "one cell does not merge");
    for pick in [Pick::RowBelow, Pick::ColumnRight] {
        with_window(&mut cx, &docs, |docs, window, cx| docs.choose(pick, window, cx));
        settled(&cx, &docs);
    }
    let path = dir.path().join("grown.odt");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, path.clone(), None, window, cx)
    });
    written(&cx, &path, None);
    let flat = text_of(&mut cx, &docs, &path);
    assert_eq!(flat.matches("<table:table-row").count(), 3, "a row below");
    assert_eq!(flat.matches("<table:table-cell").count(), 9, "and a column right");
}

#[test]
fn link_follows_the_kernel_flag() {
    let menu = |enabled: bool| {
        let json = format!(
            r#"{{"menu":[{{"text":"x","type":"command","command":".uno:InsertHyperlink","enabled":"{enabled}"}}]}}"#
        );
        ely_lok::menu::read(&json).expect("the menu parses")
    };
    assert!(Pick::Link.on(&menu(true)));
    assert!(!Pick::Link.on(&menu(false)), "protected text gets no link");
}

#[test]
fn a_misspelled_word_offers_suggestions() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Teh end");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_find(&FindReplace, window, cx)
    });
    let query = finder(&cx, &docs, |finder| finder.query.clone());
    cx.update(|app| query.update(app, |query, cx| query.set_text("Teh", cx)));
    settle(&cx, |app| total(app, &docs) == Some(1));
    let (x, y, _, h) = finder(&cx, &docs, |finder| finder.all[0]);
    let view = cx.update(|app| docs.read(app).active_tab().expect("a tab").view.clone());
    let window = cx.add_window(|_, _| Host(docs.clone(), view.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    frames(&mut shown);
    let at = shown.update(|_, app| view.read(app).window_point((x + 60, y + h / 2)));
    // A left click drops the found selection.
    shown.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    shown.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settled(&cx, &docs);
    // Writer's idle spell check marks the word some time later.
    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        right_click(&mut shown, &docs, at, Target::Text);
        let marked = cx.update(|app| docs.read(app).offer.as_ref().is_some_and(|offer| offer.what.spelling.is_some()));
        if marked || std::time::Instant::now() > until {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let rule = cx.update(|app| {
        let offer = docs.read(app).offer.as_ref().expect("an offer");
        let spelling = offer.what.spelling.as_ref().expect("a spelling menu");
        let (_, rule) = spelling
            .suggestions
            .iter()
            .find(|(label, _)| label == "The")
            .expect("The is offered");
        rule.clone()
    });
    cx.update(|app| docs.update(app, |docs, cx| docs.run_active("spell.apply", &[&rule], cx)));
    settled(&cx, &docs);
    let path = dir.path().join("spelled.odt");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, path.clone(), None, window, cx)
    });
    written(&cx, &path, None);
    assert!(
        text_of(&mut cx, &docs, &path).contains("The end"),
        "the suggestion lands"
    );
}
