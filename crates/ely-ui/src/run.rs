use std::time::Duration;

use ely_gpui_component::Assets;
use ely_gpui_component::theme::ActiveTheme;
use ely_platform::{AppId, env, graphics, instance::Start, logging, startup};
use gpui::{
    AnyWindowHandle, App, Bounds, Entity, Render, TitlebarOptions, Window, WindowBounds, WindowDecorations,
    WindowOptions, px, size,
};

use crate::theme;

/// Starts `app`; `ELY_SMOKE=1` draws a frame, `hold` stays up.
pub fn run<V: Render>(
    app: AppId,
    root: impl FnOnce(AppId, &mut Window, &mut App) -> Entity<V> + 'static,
) {
    // As the installer after an update, nothing else runs.
    if let Some(code) = crate::updates::installer(app) {
        std::process::exit(code);
    }
    let files = crate::opening::launch_args().unwrap_or_else(|error| startup::fail(app, &format!("a launch argument has no place: {error}")));
    // Before any lock is taken in the old folder.
    let (moved, admitted) = ely_platform::folder::at_start().unwrap_or_else(|error| startup::fail(app, &format!("the data folder is not ready: {error}")));
    // Before the log: a second copy must not trim it.
    let door = match ely_platform::instance::hold(app, &files) {
        Ok(Start::First(door)) => door,
        Ok(Start::Handed) => std::process::exit(0),
        Err(error) => startup::fail(app, &format!("{error}")),
    };
    // Later starts wait until this lock is held.
    drop(admitted);
    if let Err(error) = logging::init(app) {
        startup::fail(app, &format!("logging did not start: {error:#}"));
    }
    moved.log();
    if let Err(error) = graphics::check() {
        startup::fail(app, &format!("{error:#}"));
    }
    startup::guard(app);
    startup::no_window_restore();
    let smoke = Smoke::from_env();
    let application = gpui_platform::application().with_assets(Assets);
    let handed = crate::opening::listen(&application, files, door).unwrap_or_else(|error| startup::fail(app, &format!("the door for other copies did not open: {error}")));
    // The Dock icon brings back a window the tray kept.
    application.on_reopen(|cx| {
        crate::tray::show_main(cx);
    });
    application.run(move |cx: &mut App| {
        let hands = crate::opening::keep(handed, cx);
        open(app, smoke, root, hands, cx)
    });
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Smoke {
    Off,
    Once,
    Hold,
}

impl Smoke {
    fn from_env() -> Self {
        Self::from_value(env::value("ELY_SMOKE").as_deref())
    }

    fn from_value(value: Option<&str>) -> Self {
        match value {
            None => Smoke::Off,
            Some("1") => Smoke::Once,
            Some("hold") => Smoke::Hold,
            Some(other) => {
                log::warn!("run: ELY_SMOKE={other} is not 1 or hold, ignored");
                Smoke::Off
            }
        }
    }
}

fn open<V: Render>(
    app: AppId,
    smoke: Smoke,
    root: impl FnOnce(AppId, &mut Window, &mut App) -> Entity<V> + 'static,
    hands: crate::opening::Hands,
    cx: &mut App,
) {
    // Names the app's system notifications.
    cx.set_app_identity(&format!("ElyOffice.{}", app.name()), app.name());
    if let Err(error) = ely_gpui_component::init(cx) {
        startup::fail(app, &format!("the components did not start: {error:#}"));
    }
    crate::settings::install(app, cx);
    // Clears asked for last time; nothing opens what stays.
    if let Err(error) = ely_platform::storage::clear_marked(app) {
        startup::fail(app, &format!("the data you asked to clear could not be deleted, so it is not opened; the next start tries again: {error}"));
    }
    theme::install(cx);
    crate::i18n::install(cx);
    crate::updates::install(app, cx);
    cx.on_action(|_: &crate::commands::CheckForUpdates, cx| crate::updates::check_now(cx));
    crate::commands::bind_keys(cx);
    crate::commands::bind_quit(cx);
    cx.on_action(move |_: &crate::commands::OpenSettings, cx| crate::settings::open(app, cx));
    let main = match main_window(app, smoke, root, cx) {
        Ok(window) => window,
        Err(error) => startup::fail(app, &format!("the window did not open: {error:#}")),
    };
    crate::tray::set_main(app, main, cx);
    crate::opening::home(app, hands, cx);
    // Settings alone would hold the lock yet open nothing.
    cx.on_window_closed(move |cx, closed| {
        if crate::tray::closed(closed, cx) {
            cx.quit();
        }
    })
    .detach();
    if smoke == Smoke::Off {
        cx.activate(true);
    }
    // Benchmarks only: idle apps do nothing (PERF-02).
    if ely_platform::env::flag("ELY_STALLS") == Some(true) {
        watch_stalls(cx);
    }
    if ely_platform::env::flag("ELY_FRAMES") == Some(true) {
        crate::frames::watch(cx);
    }
    log::info!("{} window open", app.name());
}

/// Opens the main window where it last was.
pub(crate) fn main_window<V: Render>(
    app: AppId,
    smoke: Smoke,
    root: impl FnOnce(AppId, &mut Window, &mut App) -> Entity<V> + 'static,
    cx: &mut App,
) -> anyhow::Result<AnyWindowHandle> {
    let mut options = WindowOptions {
        focus: smoke == Smoke::Off,
        inactive_frame_interval: (smoke != Smoke::Hold).then_some(Duration::from_millis(250)),
        ..window_options(app, size(px(1100.0), px(720.0)), cx)
    };
    if let Some(bounds) = crate::settings::restored_frame(app, cx) {
        options.window_bounds = Some(bounds);
    }
    let opened = cx.open_window(options, |window, cx| {
        window.on_next_frame(move |_, cx| {
            startup::started();
            if smoke == Smoke::Once {
                println!("ready {}", app.name());
                cx.quit();
            }
        });
        theme::follow_system(window);
        crate::settings::follow_frame(app, window, cx);
        root(app, window, cx)
    })?;
    Ok(opened.into())
}

/// Logs each main-thread stretch busy past 100 ms.
fn watch_stalls(cx: &mut App) {
    const BEAT: Duration = Duration::from_millis(20);
    cx.spawn(async move |cx: &mut gpui::AsyncApp| {
        loop {
            let asked = std::time::Instant::now();
            cx.background_executor().timer(BEAT).await;
            let late = asked.elapsed().saturating_sub(BEAT);
            if late > Duration::from_millis(100) {
                log::warn!("ui: the main thread was busy {} ms", late.as_millis());
            }
        }
    })
    .detach();
}

/// A window of `app`, `size` large, with Ely's title bar.
pub fn window_options(app: AppId, size: gpui::Size<gpui::Pixels>, cx: &App) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size, cx))),
        titlebar: Some(TitlebarOptions {
            title: Some(app.name().into()),
            appears_transparent: true,
            traffic_light_position: Some(cx.theme().traffic_light_origin()),
        }),
        window_decorations: Some(WindowDecorations::Client),
        window_min_size: Some(gpui::size(px(640.0), px(420.0))),
        app_id: Some(app.slug().into()),
        inactive_frame_interval: Some(Duration::from_millis(250)),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::Smoke;

    #[test]
    fn only_one_and_hold_start_a_smoke_run() {
        assert!(Smoke::from_value(Some("1")) == Smoke::Once);
        assert!(Smoke::from_value(Some("hold")) == Smoke::Hold);
        for other in [None, Some("0"), Some(""), Some("true")] {
            assert!(Smoke::from_value(other) == Smoke::Off, "{other:?}");
        }
    }
}
