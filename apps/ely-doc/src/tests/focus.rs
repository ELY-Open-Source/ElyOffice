//! The keys follow the front page across tab switches (R15.40).

use ely_gpui_component::theme::Theme;
use gpui::{Entity, FocusHandle, Focusable as _, TestAppContext, VisualTestContext};

use crate::docs::Docs;
use crate::tests::{blank, docs, settle};
use ely_office::{FindReplace, prelude::*};

/// The app's window, beside a control that stays.
struct Front(Entity<Docs>, FocusHandle);

impl gpui::Render for Front {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{InteractiveElement as _, ParentElement as _, Styled as _};
        gpui::div().size_full().child(self.0.clone()).child(gpui::div().track_focus(&self.1))
    }
}

fn frames(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

/// Two blank tabs in Docs' window; the first has keys.
fn two_tabs(cx: &mut TestAppContext, dir: &std::path::Path) -> Option<(Entity<Docs>, VisualTestContext, u64, u64, FocusHandle)> {
    let docs = docs(cx, dir)?;
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
        ely_ui::set_document_menus(ely_platform::AppId::Doc, &ely_ui::commands::FILE, Vec::new(), cx);
    });
    let (a, b) = (blank(cx, &docs), blank(cx, &docs));
    let window = cx.add_window(|_, cx| Front(docs.clone(), cx.focus_handle()));
    let mut shown = VisualTestContext::from_window(window.into(), cx);
    // Focus events need the window in front, as typing does.
    shown.update(|window, _| window.activate_window());
    shown.run_until_parked();
    assert!(shown.update(|window, _| window.is_window_active()), "the window is in front");
    shown.update(|window, app| {
        docs.update(app, |docs, cx| {
            docs.own_window(window, cx);
            docs.core.set_active(Some(a));
            let focus = docs.tab(a).expect("a").view.read(cx).focus_handle().clone();
            window.focus(&focus, cx);
            cx.notify();
        })
    });
    frames(&mut shown);
    assert!(holds_keys(&mut shown, &docs, a), "the first tab starts with the keys");
    let kept = shown.update(|window, app| window.root::<Front>().flatten().expect("the front").read(app).1.clone());
    Some((docs, shown, a, b, kept))
}

fn holds_keys(cx: &mut VisualTestContext, docs: &Entity<Docs>, id: u64) -> bool {
    cx.update(|window, app| docs.read(app).tab(id).expect("open").view.read(app).focus_handle().is_focused(window))
}

/// Each tab's id and kernel edit count.
fn edits(cx: &mut VisualTestContext, docs: &Entity<Docs>) -> Vec<(u64, u64)> {
    settle(cx, |app| docs.read(app).core.tabs.iter().all(|tab| tab.view.read(app).settled()));
    cx.update(|_, app| docs.read(app).core.tabs.iter().map(|tab| (tab.id, tab.view.read(app).edited())).collect())
}

/// One typed letter must reach tab `id` alone.
#[track_caller]
fn first_key_lands(cx: &mut VisualTestContext, docs: &Entity<Docs>, id: u64) {
    let before = edits(cx, docs);
    cx.simulate_input("Q");
    let after = edits(cx, docs);
    let grew: Vec<u64> = (before.iter().zip(&after))
        .filter(|(old, new)| new.1 > old.1)
        .map(|(_, new)| new.0)
        .collect();
    assert_eq!(grew, vec![id], "the first key lands in tab {id}");
}

#[test]
fn a_picked_tab_takes_the_keys() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, mut shown, a, b, _)) = two_tabs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    for id in [b, a, b] {
        // What a click on the tab strip runs.
        shown.update(|window, app| docs.update(app, |docs, cx| docs.show_tab(id, window, cx)));
        frames(&mut shown);
        assert!(holds_keys(&mut shown, &docs, id), "tab {id} has the keys");
        first_key_lands(&mut shown, &docs, id);
    }
}

#[test]
fn a_page_that_leaves_hands_the_keys_to_the_front() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, mut shown, a, b, _)) = two_tabs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    // A switch that leaves focus alone, as Go Back does.
    shown.update(|_, app| docs.update(app, |docs, cx| {
        docs.core.set_active(Some(b));
        cx.notify();
    }));
    frames(&mut shown);
    assert!(holds_keys(&mut shown, &docs, b), "the shown tab has the keys");
    first_key_lands(&mut shown, &docs, b);
    shown.update(|_, app| docs.update(app, |docs, cx| docs.close_tab(b, cx)));
    frames(&mut shown);
    assert!(holds_keys(&mut shown, &docs, a), "closing the front tab hands over");
    first_key_lands(&mut shown, &docs, a);
    shown.update(|_, app| docs.update(app, |docs, cx| docs.close_tab(a, cx)));
    frames(&mut shown);
    let root = shown.update(|window, app| docs.read(app).core.focus.is_focused(window));
    assert!(root, "with no tab the window keeps the keys for New and Open");
}

#[test]
fn a_find_box_that_leaves_with_its_tab_hands_the_keys_on() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((docs, mut shown, a, b, kept)) = two_tabs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    shown.update(|window, app| docs.update(app, |docs, cx| docs.on_find(&FindReplace, window, cx)));
    frames(&mut shown);
    let query = shown.update(|_, app| docs.read(app).core.finder.as_ref().expect("open").query.clone());
    assert!(shown.update(|window, app| query.focus_handle(app).is_focused(window)), "find has the keys");
    // Go Back from the field: A's box leaves with A.
    shown.update(|_, app| docs.update(app, |docs, cx| {
        docs.core.set_active(Some(b));
        cx.notify();
    }));
    frames(&mut shown);
    assert!(holds_keys(&mut shown, &docs, b), "the front page has the keys");
    first_key_lands(&mut shown, &docs, b);
    // A control that stays keeps them.
    shown.update(|window, app| window.focus(&kept, app));
    shown.update(|_, app| docs.update(app, |docs, cx| {
        docs.core.set_active(Some(a));
        cx.notify();
    }));
    frames(&mut shown);
    assert!(shown.update(|window, _| kept.is_focused(window)), "the user's control keeps the keys");
}
