use ely_platform::AppId;
use gpui::{AppContext as _, Context, IntoElement, Render, TestAppContext, Window};

/// Unshipped: its saved window place is no one's.
const APP: AppId = AppId::Agent;

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
    }
}

fn more(_: &mut gpui::App, _: bool) -> std::io::Result<super::Build> {
    Ok(Box::new(|window, cx| {
        let blank = cx.new(|_| Blank);
        crate::tool_root(APP, blank.into(), Vec::new(), window, cx)
    }))
}

fn installed(cx: &mut TestAppContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        crate::settings::install_defaults(cx);
        crate::theme::install(cx);
        crate::i18n::install_as("en-US", cx);
    });
}

#[gpui::test]
fn one_window_opens_no_other(cx: &mut TestAppContext) {
    installed(cx);
    assert!(!cx.update(|cx| super::allowed(cx)));
    assert!(cx.update(|cx| super::open(true, cx)).is_err(), "an app not declaring windows refuses");
}

#[gpui::test]
fn new_window_from_a_window_opens_one(cx: &mut TestAppContext) {
    installed(cx);
    cx.update(|cx| super::allow(APP, more, cx));
    let first = cx.update(|cx| crate::run::main_window(APP, crate::run::Smoke::Off, |_, window, cx| more(cx, true).expect("built")(window, cx), cx)).expect("the first");
    cx.update(|cx| crate::tray::set_main(APP, first, cx));
    first.update(cx, |_, window, cx| window.dispatch_action(Box::new(crate::commands::NewWindow), cx)).expect("dispatched");
    cx.run_until_parked();
    assert_eq!(cx.update(|cx| cx.windows().len()), 2, "the menu's window opened");
    assert_ne!(cx.update(|cx| crate::tray::main(cx)), Some(first));
}

#[gpui::test]
fn a_new_window_sits_off_the_front_one_and_quits_last(cx: &mut TestAppContext) {
    installed(cx);
    cx.update(|cx| super::allow(APP, more, cx));
    let first = cx.update(|cx| crate::run::main_window(APP, crate::run::Smoke::Off, |_, window, cx| more(cx, true).expect("built")(window, cx), cx)).expect("the first");
    cx.update(|cx| crate::tray::set_main(APP, first, cx));
    let second = cx.update(|cx| super::open(true, cx)).expect("a second window");
    cx.run_until_parked();
    let at = |window: gpui::AnyWindowHandle, cx: &mut TestAppContext| window.update(cx, |_, window, _| window.window_bounds().get_bounds()).expect("open");
    let (one, two) = (at(first, cx), at(second, cx));
    assert_eq!(two.origin.x - one.origin.x, gpui::px(super::STEP));
    assert_eq!(two.size, one.size, "the new window takes the front one's size");
    assert_eq!(cx.update(|cx| crate::tray::main(cx)), Some(second), "the new window is in front");
    cx.update(|cx| crate::tray::raised(first, cx));
    assert_eq!(cx.update(|cx| crate::tray::main(cx)), Some(first), "the window with the keys is in front");
    assert!(!cx.update(|cx| crate::tray::closed(first.window_id(), cx)), "one window left, no quit");
    assert_eq!(cx.update(|cx| crate::tray::main(cx)), Some(second));
    assert!(cx.update(|cx| crate::tray::closed(second.window_id(), cx)), "the last window quits");
}
