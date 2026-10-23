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
