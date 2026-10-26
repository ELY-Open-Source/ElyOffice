//! The main window, and the tray icon that outlives it.

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
    window: Option<AnyWindowHandle>,
    kept: Option<Kept>,
}

impl Global for Main {}

pub(crate) fn set_main(app: AppId, window: AnyWindowHandle, cx: &mut App) {
    match cx.try_global::<Main>().is_some() {
        true => cx.global_mut::<Main>().window = Some(window),
        false => cx.set_global(Main { app, window: Some(window), kept: None }),
    }
}

/// Main window closed: true quits; a tray keeps the app.
pub(crate) fn closed(window: WindowId, cx: &mut App) -> bool {
    let Some(main) = cx.try_global::<Main>() else {
        return false;
    };
    if main.window.is_none_or(|main| main.window_id() != window) {
        return false;
    }
    if main.kept.is_none() {
        return true;
    }
    cx.global_mut::<Main>().window = None;
    log::info!("tray: the main window closed; the app stays in the tray");
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
        false => cx.set_global(Main { app, window: None, kept }),
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

/// Whether the main window is open and in front.
pub fn main_in_front(cx: &App) -> bool {
    let main = cx.try_global::<Main>().and_then(|main| main.window);
    main.is_some_and(|main| cx.active_window() == Some(main))
}

/// Whether closing the main window leaves the app running.
pub fn kept(cx: &App) -> bool {
    cx.try_global::<Main>().is_some_and(|main| main.kept.is_some())
}

/// Brings the main window forward, reopening one the tray kept.
pub fn show_main(cx: &mut App) -> Option<AnyWindowHandle> {
    let main = cx.try_global::<Main>()?;
    let (app, window) = (main.app, main.window);
    let window = match window {
        Some(window) => window,
        None => {
            let reopen = main.kept.as_ref()?.reopen.clone();
            let opened = crate::run::main_window(app, crate::run::Smoke::Off, move |_, window, cx| reopen(window, cx), cx);
            match opened {
                Ok(window) => {
                    log::info!("tray: the main window opened again");
                    cx.global_mut::<Main>().window = Some(window);
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

#[cfg(test)]
mod tests;
