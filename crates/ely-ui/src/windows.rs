//! Later windows of one app (W.1b).

use ely_platform::AppId;
use gpui::{AnyWindowHandle, App, Entity, Global, Render, Window, WindowBounds, point, px, size};

use crate::Root;

/// A later window's root, once its records are found.
pub type Build = Box<dyn FnOnce(&mut Window, &mut App) -> Entity<Root>>;

/// Finds the app's records; `fresh` asks for a blank document.
pub type More = fn(&mut App, bool) -> std::io::Result<Build>;

/// The app opens more windows, built so.
struct Windows(AppId, More);

impl Global for Windows {}

/// How far a new window sits from the front one.
const STEP: f32 = 32.0;

/// The app opens more windows; New Window opens one.
pub(crate) fn allow(app: AppId, more: More, cx: &mut App) {
    cx.set_global(Windows(app, more));
    cx.on_action(|_: &crate::commands::NewWindow, cx| {
        // The asking window is busy until its action ends.
        cx.defer(|cx| {
            if let Err(error) = open(true, cx) {
                log::error!("ui: a new window did not open: {error:#}");
            }
        });
    });
}

/// Whether this app opens more than one window.
pub fn allowed(cx: &App) -> bool {
    cx.has_global::<Windows>()
}

/// Opens another window, offset from the front one.
pub fn open(fresh: bool, cx: &mut App) -> anyhow::Result<AnyWindowHandle> {
    let &Windows(app, more) = cx.try_global::<Windows>().ok_or_else(|| anyhow::anyhow!("this app opens one window"))?;
    let mut options = crate::run::window_options(app, size(px(1100.0), px(720.0)), cx);
    if let Some(front) = crate::tray::main(cx) {
        let mut bounds = front.update(cx, |_, window, _| window.window_bounds())?.get_bounds();
        bounds.origin += point(px(STEP), px(STEP));
        options.window_bounds = Some(WindowBounds::Windowed(bounds));
    }
    let build = more(cx, fresh)?;
    let opened = cx.open_window(options, move |window, cx| {
        crate::theme::follow_system(window);
        let root = build(window, cx);
        follow(&root, window, cx);
        root
    })?;
    let opened: AnyWindowHandle = opened.into();
    crate::tray::set_main(app, opened, cx);
    log::info!("{}: another window opened", app.slug());
    Ok(opened)
}

/// The window that takes the keys becomes the front one.
pub(crate) fn follow<V: Render>(root: &Entity<V>, window: &mut Window, cx: &mut App) {
    root.update(cx, |_, cx| {
        cx.observe_window_activation(window, |_, window, cx| {
            if window.is_window_active() {
                crate::tray::raised(window.window_handle(), cx);
            }
        })
        .detach();
    });
}

/// Lets apps' tests open more windows without `run`.
#[cfg(any(test, feature = "test-support"))]
pub fn allow_for_tests(app: AppId, more: More, cx: &mut App) {
    allow(app, more, cx);
}

#[cfg(test)]
mod tests;
