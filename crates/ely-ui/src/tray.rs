//! The document windows, front last, and the tray icon.

use std::rc::Rc;

use ely_gpui_component::desktop::{TrayIcon, TrayItem, TrayLook};
use ely_platform::AppId;
use gpui::{AnyWindowHandle, App, Entity, Global, QuitMode, Window, WindowId};

use crate::{Root, i18n};

type Reopen = Rc<dyn Fn(&mut Window, &mut App) -> Entity<Root>>;

struct Kept {
    reopen: Reopen,
    _icon: TrayIcon,
}

struct Main {
    app: AppId,
    /// Open document windows; the last active is last.
    windows: Vec<AnyWindowHandle>,
    kept: Option<Kept>,
}

impl Global for Main {}

/// The front document window; set only when it changes.
pub struct Front(pub Option<AnyWindowHandle>);

impl Global for Front {}

/// Announces a new front window to `observe_global::<Front>`.
fn announce(cx: &mut App) {
    let front = main(cx);
    if cx.try_global::<Front>().is_none_or(|was| was.0 != front) {
        log::info!("tray: the front document window is {:?}", front.map(|window| window.window_id()));
        cx.set_global(Front(front));
    }
}

/// A document window opened; it is in front.
pub(crate) fn set_main(app: AppId, window: AnyWindowHandle, cx: &mut App) {
    match cx.try_global::<Main>().is_some() {
        true => cx.global_mut::<Main>().windows.push(window),
        false => cx.set_global(Main { app, windows: vec![window], kept: None }),
    }
    announce(cx);
}

/// Document window `window` took the keys.
pub(crate) fn raised(window: AnyWindowHandle, cx: &mut App) {
    if !cx.has_global::<Main>() {
        return;
    }
    let main = cx.global_mut::<Main>();
    let Some(at) = main.windows.iter().position(|open| *open == window) else { return };
    let window = main.windows.remove(at);
    main.windows.push(window);
    announce(cx);
}

/// A window closed: true quits; a tray keeps the app.
pub(crate) fn closed(window: WindowId, cx: &mut App) -> bool {
    if !cx.has_global::<Main>() {
        return false;
    }
    let main = cx.global_mut::<Main>();
    let Some(at) = main.windows.iter().position(|open| open.window_id() == window) else {
        return false;
    };
    main.windows.remove(at);
    log::info!("tray: a document window closed, {} left", main.windows.len());
    let (left, kept) = (!main.windows.is_empty(), main.kept.is_some());
    announce(cx);
    if left {
        return false;
    }
    if !kept {
        return true;
    }
    log::info!("tray: the last window closed; the app stays in the tray");
    false
}

/// Keeps the app in the tray once its window closes.
pub fn keep(
    app: AppId,
    png: &'static [u8],
    symbol: &'static str,
    reopen: impl Fn(&mut Window, &mut App) -> Entity<Root> + 'static,
    cx: &mut App,
) -> anyhow::Result<()> {
    let items = vec![
        TrayItem::Action(i18n::text(cx, "tray.open", &[("app", app.name())])),
        TrayItem::Separator,
        TrayItem::Action(i18n::text(cx, "menu.app.quit", &[("app", app.name())])),
    ];
    let look = TrayLook { symbol, png, name: app.name().into() };
    let icon = TrayIcon::new(look, items, picked, cx)?;
    // gpui otherwise quits with the last window.
    cx.set_quit_mode(QuitMode::Explicit);
    let kept = Some(Kept { reopen: Rc::new(reopen), _icon: icon });
    match cx.try_global::<Main>().is_some() {
        true => cx.global_mut::<Main>().kept = kept,
        // The root builds before its window is known.
        false => cx.set_global(Main { app, windows: Vec::new(), kept }),
    }
    Ok(())
}

fn picked(index: usize, cx: &mut App) {
    match index {
        0 => {
            show_main(cx);
        }
        2 => crate::commands::quit(cx),
        other => log::error!("tray: no item {other}"),
    }
}

/// Whether a document window is open and in front.
pub fn main_in_front(cx: &App) -> bool {
    main(cx).is_some_and(|main| cx.active_window() == Some(main))
}

/// Whether closing the main window leaves the app running.
pub fn kept(cx: &App) -> bool {
    cx.try_global::<Main>().is_some_and(|main| main.kept.is_some())
}

/// The last active document window, while one is open.
pub fn main(cx: &App) -> Option<AnyWindowHandle> {
    cx.try_global::<Main>()?.windows.last().copied()
}

/// Brings the front window forward, or reopens one.
pub fn show_main(cx: &mut App) -> Option<AnyWindowHandle> {
    let main = cx.try_global::<Main>()?;
    let (app, window) = (main.app, main.windows.last().copied());
    let window = match window {
        Some(window) => window,
        None => {
            let reopen = main.kept.as_ref()?.reopen.clone();
            let opened = crate::run::main_window(app, crate::run::Smoke::Off, move |_, window, cx| reopen(window, cx), cx);
            match opened {
                Ok(window) => {
                    log::info!("tray: the main window opened again");
                    cx.global_mut::<Main>().windows.push(window);
                    window
                }
                Err(error) => {
                    log::error!("tray: the main window did not open again: {error:#}");
                    return None;
                }
            }
        }
    };
    cx.activate(true);
    window.update(cx, |_, window, _| window.activate_window()).ok()?;
    Some(window)
}

/// The window list moved by hand, for apps' tests.
#[cfg(any(test, feature = "test-support"))]
pub mod testing {
    use gpui::{AnyWindowHandle, App};

    pub fn opened(app: ely_platform::AppId, window: AnyWindowHandle, cx: &mut App) {
        super::set_main(app, window, cx);
    }

    pub fn raised(window: AnyWindowHandle, cx: &mut App) {
        super::raised(window, cx);
    }

    pub fn closed(window: AnyWindowHandle, cx: &mut App) -> bool {
        super::closed(window.window_id(), cx)
    }
}

#[cfg(test)]
mod tests;
