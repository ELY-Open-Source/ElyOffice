use std::{cell::Cell, rc::Rc};

use ely_platform::AppId;
use gpui::{AnyWindowHandle, AppContext as _, Context, IntoElement, Render, TestAppContext, Window};

const PNG: &[u8] = include_bytes!("../../../../assets/icons/ely-mail.png");
/// Unshipped: its saved window place is no one's.
const APP: AppId = AppId::Agent;

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
    }
}

fn root(window: &mut Window, cx: &mut gpui::App) -> gpui::Entity<crate::Root> {
    let blank = cx.new(|_| Blank);
    crate::tool_root(APP, blank.into(), Vec::new(), window, cx)
}

fn installed(cx: &mut TestAppContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        crate::settings::install_defaults(cx);
        crate::theme::install(cx);
        crate::i18n::install_as("en-US", cx);
    });
}

fn open(cx: &mut TestAppContext) -> AnyWindowHandle {
    let window = cx
        .update(|cx| crate::run::main_window(APP, crate::run::Smoke::Off, |_, window, cx| root(window, cx), cx))
        .expect("a window");
    cx.update(|cx| super::set_main(APP, window, cx));
    window
}

fn close(window: AnyWindowHandle, cx: &mut TestAppContext) {
    window.update(cx, |_, window, _| window.remove_window()).expect("closed");
    cx.run_until_parked();
}

fn keep(reopened: Rc<Cell<usize>>, cx: &mut TestAppContext) -> anyhow::Result<()> {
    cx.update(|cx| {
        super::keep(APP, PNG, "envelope", move |window, cx| {
            reopened.set(reopened.get() + 1);
            root(window, cx)
        }, cx)
    })
}

/// Without a tray, gpui quits when the last window goes.
#[gpui::test]
fn an_unkept_app_quits_with_its_window(cx: &mut TestAppContext) {
    installed(cx);
    let first = open(cx);
    assert!(cx.update(|cx| super::closed(first.window_id(), cx)), "with no tray, closing quits");
    close(first, cx);
    assert_eq!(cx.quit_requests(), usize::from(cfg!(not(target_os = "macos"))));
}

// AppKit's status item needs the main thread.
#[cfg(not(target_os = "macos"))]
#[gpui::test]
fn a_kept_app_closes_to_the_tray_and_opens_again(cx: &mut TestAppContext) {
    installed(cx);
    let Some(_host) = host::serve() else {
        return eprintln!("SKIP: no dbus-daemon");
    };
    let first = open(cx);
    let reopened = Rc::new(Cell::new(0));
    keep(reopened.clone(), cx).expect("the tray shows");
    close(first, cx);
    assert_eq!(cx.quit_requests(), 0, "the tray keeps gpui from quitting");
    assert!(!cx.update(|cx| super::closed(first.window_id(), cx)), "the tray keeps the app");
    assert!(cx.update(|cx| cx.windows().is_empty()));
    // The tray's first item.
    cx.update(|cx| super::picked(0, cx));
    let again = cx.update(|cx| cx.windows());
    assert_eq!((again.len(), reopened.get()), (1, 1));
    assert_ne!(again[0].window_id(), first.window_id());
    cx.update(|cx| super::picked(0, cx));
    assert_eq!((cx.update(|cx| cx.windows().len()), reopened.get()), (1, 1), "an open window is only raised");
}

/// No tray host: the app closes as any other.
#[cfg(target_os = "linux")]
#[gpui::test]
fn without_a_tray_host_closing_quits(cx: &mut TestAppContext) {
    installed(cx);
    let Some(_bus) = host::bus() else {
        return eprintln!("SKIP: no dbus-daemon");
    };
    let first = open(cx);
    let refused = keep(Rc::new(Cell::new(0)), cx).expect_err("no host, no tray");
    assert!(format!("{refused:#}").contains("no tray host"), "{refused:#}");
    assert!(cx.update(|cx| super::closed(first.window_id(), cx)));
    close(first, cx);
    assert_eq!(cx.quit_requests(), 1);
}

/// A private session bus and a stand-in tray host.
#[cfg(target_os = "linux")]
mod host;

#[cfg(not(target_os = "linux"))]
mod host {
    /// Windows and macOS need no host.
    pub fn serve() -> Option<()> {
        Some(())
    }
}
