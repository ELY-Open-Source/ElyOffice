use gpui::{
    AnyView, AppContext as _, Context, FocusHandle, InteractiveElement as _, IntoElement, ParentElement as _, Render, Styled as _,
    TestAppContext, VisualTestContext, Window, div,
};

use super::Shell;

/// A panel that names itself for `debug_bounds`.
struct Probe(&'static str);

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let name = self.0;
        div().debug_selector(move || name.into()).size_full()
    }
}

/// A window with both panels, as the apps build it.
struct Framed(FocusHandle, AnyView, AnyView);

impl Render for Framed {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().key_context(crate::commands::CONTEXT).child(
            Shell::new("framed", &self.0).secondary().left(Some(self.1.clone())).right(Some(self.2.clone())),
        )
    }
}

/// Waits out the slide until `name` is drawn or not.
fn drawn(cx: &mut VisualTestContext, name: &'static str, want: bool) -> bool {
    for _ in 0..50 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        if cx.debug_bounds(name).is_some() == want {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    false
}

#[gpui::test]
fn either_panel_folds_from_its_key(cx: &mut TestAppContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        crate::settings::install_defaults(cx);
        crate::i18n::install(cx);
        crate::commands::bind_keys(cx);
    });
    let window = cx.add_window(|_, cx| {
        let (left, right) = (cx.new(|_| Probe("left-probe")), cx.new(|_| Probe("right-probe")));
        Framed(cx.focus_handle(), left.into(), right.into())
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, cx| {
        let focus = window.root::<Framed>().flatten().expect("the root").read(cx).0.clone();
        window.focus(&focus, cx);
    });
    cx.simulate_resize(gpui::size(gpui::px(1200.0), gpui::px(800.0)));
    assert!(drawn(&mut cx, "left-probe", true) && drawn(&mut cx, "right-probe", true), "both show");
    cx.simulate_keystrokes("secondary-alt-s");
    assert!(drawn(&mut cx, "left-probe", false), "the left folds");
    assert!(drawn(&mut cx, "right-probe", true), "the right stays");
    cx.simulate_keystrokes("secondary-alt-b");
    assert!(drawn(&mut cx, "right-probe", false), "the right folds");
    cx.simulate_keystrokes("secondary-alt-s");
    assert!(drawn(&mut cx, "left-probe", true), "the left is back");
}

/// One frame, no waiting out a slide.
fn frame(cx: &mut VisualTestContext) {
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

#[gpui::test]
fn a_panel_without_room_goes_at_once(cx: &mut TestAppContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        crate::settings::install_defaults(cx);
        crate::i18n::install(cx);
    });
    let window = cx.add_window(|_, cx| {
        let (left, right) = (cx.new(|_| Probe("left-probe")), cx.new(|_| Probe("right-probe")));
        Framed(cx.focus_handle(), left.into(), right.into())
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(gpui::size(gpui::px(1100.0), gpui::px(680.0)));
    assert!(drawn(&mut cx, "right-probe", true), "both show");
    cx.simulate_resize(gpui::size(gpui::px(800.0), gpui::px(680.0)));
    frame(&mut cx);
    cx.simulate_resize(gpui::size(gpui::px(799.0), gpui::px(680.0)));
    frame(&mut cx);
    let body = cx.debug_bounds("shell-body").expect("the content");
    assert!(body.size.width >= gpui::px(360.0), "the first frame squeezed the content to {:?}", body.size.width);
    assert!(cx.debug_bounds("right-probe").is_none(), "the right slid instead of going");
    cx.simulate_resize(gpui::size(gpui::px(800.0), gpui::px(680.0)));
    assert!(drawn(&mut cx, "right-probe", true), "room again: it returns");
}

/// Each frame of a fold keeps the content's least width.
fn folds_within(cx: &mut VisualTestContext, keys: &str, name: &'static str) {
    cx.simulate_keystrokes(keys);
    for _ in 0..50 {
        frame(cx);
        let body = cx.debug_bounds("shell-body").expect("the content");
        assert!(body.size.width >= gpui::px(360.0), "{name} folding squeezed the content to {:?}", body.size.width);
        if cx.debug_bounds(name).is_none() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("{name} never folded");
}

#[gpui::test]
fn a_fold_at_a_narrow_width_never_squeezes_the_content(cx: &mut TestAppContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        crate::settings::install_defaults(cx);
        crate::i18n::install(cx);
        crate::commands::bind_keys(cx);
    });
    let window = cx.add_window(|_, cx| {
        let (left, right) = (cx.new(|_| Probe("left-probe")), cx.new(|_| Probe("right-probe")));
        Framed(cx.focus_handle(), left.into(), right.into())
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, cx| {
        let focus = window.root::<Framed>().flatten().expect("the root").read(cx).0.clone();
        window.focus(&focus, cx);
    });
    cx.simulate_resize(gpui::size(gpui::px(800.0), gpui::px(680.0)));
    assert!(drawn(&mut cx, "left-probe", true) && drawn(&mut cx, "right-probe", true), "both show");
    folds_within(&mut cx, "secondary-alt-b", "right-probe");
    cx.simulate_keystrokes("secondary-alt-b");
    assert!(drawn(&mut cx, "right-probe", true), "the right is back");
    folds_within(&mut cx, "secondary-alt-s", "left-probe");
}

/// A focused window with both panels shown at `width`.
fn both_at(cx: &mut TestAppContext, width: f32) -> VisualTestContext {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        crate::settings::install_defaults(cx);
        crate::i18n::install(cx);
        crate::commands::bind_keys(cx);
    });
    let window = cx.add_window(|_, cx| {
        let (left, right) = (cx.new(|_| Probe("left-probe")), cx.new(|_| Probe("right-probe")));
        Framed(cx.focus_handle(), left.into(), right.into())
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, cx| {
        let focus = window.root::<Framed>().flatten().expect("the root").read(cx).0.clone();
        window.focus(&focus, cx);
    });
    cx.simulate_resize(gpui::size(gpui::px(width), gpui::px(680.0)));
    assert!(drawn(&mut cx, "left-probe", true) && drawn(&mut cx, "right-probe", true), "both show");
    cx
}

/// Frames until `gone` is drawn no more, the content kept.
fn keeps_least(cx: &mut VisualTestContext, gone: &'static str) {
    for _ in 0..60 {
        frame(cx);
        let body = cx.debug_bounds("shell-body").expect("the content");
        assert!(body.size.width >= gpui::px(360.0), "the content shrank to {:?}", body.size.width);
        if cx.debug_bounds(gone).is_none() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("{gone} never went");
}

/// Folds two 30 ms apart; the second slides from its width.
fn fold_both(cx: &mut VisualTestContext, first: (&str, &'static str), second: (&str, &'static str)) {
    cx.simulate_keystrokes(first.0);
    frame(cx);
    std::thread::sleep(std::time::Duration::from_millis(30));
    frame(cx);
    let before = cx.debug_bounds(second.1).expect("still open").size.width;
    cx.simulate_keystrokes(second.0);
    frame(cx);
    let after = cx.debug_bounds(second.1).expect("folding").size.width;
    assert!(after <= before, "{} jumped from {before:?} to {after:?}", second.1);
    assert!(cx.debug_bounds(first.1).is_some(), "{} was cut short", first.1);
    keeps_least(cx, first.1);
    keeps_least(cx, second.1);
}

#[gpui::test]
fn overlapping_folds_keep_the_content_either_way(cx: &mut TestAppContext) {
    let cx = &mut both_at(cx, 800.0);
    fold_both(cx, ("secondary-alt-b", "right-probe"), ("secondary-alt-s", "left-probe"));
    cx.simulate_keystrokes("secondary-alt-b");
    cx.simulate_keystrokes("secondary-alt-s");
    assert!(drawn(cx, "left-probe", true) && drawn(cx, "right-probe", true), "both are back");
    fold_both(cx, ("secondary-alt-s", "left-probe"), ("secondary-alt-b", "right-probe"));
}

#[gpui::test]
fn narrowing_during_a_fold_keeps_the_content(cx: &mut TestAppContext) {
    let cx = &mut both_at(cx, 1100.0);
    cx.simulate_keystrokes("secondary-alt-b");
    frame(cx);
    cx.simulate_resize(gpui::size(gpui::px(640.0), gpui::px(680.0)));
    keeps_least(cx, "right-probe");
    assert!(cx.debug_bounds("left-probe").is_some(), "the left stays");
}
