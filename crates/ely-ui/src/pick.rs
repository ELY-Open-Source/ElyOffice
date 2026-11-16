//! The unified AppImage asks which app to start.

use std::{ffi::OsString, path::PathBuf, process::Command};

use ely_gpui_component::{buttons::Button, primitives::Severity};
use ely_platform::{AppId, env, startup};
use gpui::{
    AppContext as _, Context, FocusHandle, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, px, size,
};

use crate::{i18n::text, shell::Shell};

/// Runs the choice in `host`'s binary, then exits.
pub(crate) fn run(host: AppId) {
    let image = env::value("APPIMAGE").map(PathBuf::from).unwrap_or_else(|| startup::fail(host, "ELY_PICK is for the AppImage, and APPIMAGE is unset"));
    let logged = ely_platform::root().and_then(|root| ely_platform::logging::init_at("ely-pick", &root.join("ely-pick.log")));
    if let Err(error) = logged {
        startup::fail(host, &format!("logging did not start: {error:#}"));
    }
    // The files go to the chosen app.
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    log::info!("pick: {} has no app in its name; {} files wait", image.display(), args.len());
    gpui_platform::application().with_assets(ely_gpui_component::Assets).run(move |cx| {
        if let Err(error) = ely_gpui_component::init(cx) {
            startup::fail(host, &format!("the components did not start: {error:#}"));
        }
        crate::settings::install(host, cx);
        crate::theme::install(cx);
        crate::i18n::install(cx);
        let mut options = crate::run::window_options(host, size(px(320.0), px(420.0)), cx);
        if let Some(bar) = options.titlebar.as_mut() {
            bar.title = Some("Ely Office".into());
        }
        options.app_id = Some("ely-office".into());
        let opened = cx.open_window(options, move |window, cx| {
            crate::theme::follow_system(window);
            cx.new(|cx| Pick::new(image, args, window, cx))
        });
        if let Err(error) = opened {
            startup::fail(host, &format!("the window did not open: {error:#}"));
        }
        cx.on_window_closed(|cx, _| cx.quit()).detach();
        cx.activate(true);
    });
}

struct Pick {
    image: PathBuf,
    args: Vec<OsString>,
    focus: FocusHandle,
    failed: Option<SharedString>,
}

impl Pick {
    fn new(image: PathBuf, args: Vec<OsString>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self { image, args, focus, failed: None }
    }

    /// Starts `app` from the image; this window goes.
    fn start(&mut self, app: AppId, cx: &mut Context<Self>) {
        let mut command = Command::new(&self.image);
        command.arg(app.slug()).args(&self.args).env_remove("ELY_PICK");
        match command.spawn() {
            Ok(child) => {
                log::info!("pick: started {} as {}", app.name(), child.id());
                cx.quit();
            }
            Err(error) => {
                log::error!("pick: {} did not start: {error}", app.name());
                self.failed = Some(text(cx, "menu.app.switch_failed", &[("app", app.name()), ("error", &error.to_string())]));
                cx.notify();
            }
        }
    }
}

impl Render for Pick {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        // The image's own: ElyAgent comes later.
        let apps = AppId::ALL.into_iter().filter(|app| ely_platform::apps::installed(*app)).map(|app| {
            let this = this.clone();
            Button::new(SharedString::from(format!("pick-{}", app.slug())), app.name()).full_width().on_click(move |_, _, cx| {
                this.update(cx, |pick, cx| pick.start(app, cx)).ok();
            })
        });
        let failed = self.failed.clone().map(|said| crate::settings::said::line("pick-failed", Severity::Danger, said, cx));
        Shell::new("Ely Office", &self.focus).secondary().child(
            div()
                .size_full()
                .p_6()
                .flex()
                .flex_col()
                .gap_2()
                .child(div().pb_2().child(text(cx, "pick.title", &[])))
                .children(apps)
                .children(failed),
        )
    }
}
