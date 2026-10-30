use ely_gpui_component::{
    feedback::{Toast, ToastViewport, Toaster},
    navigation::{Command as PaletteCommand, CommandPalette},
    primitives::{Severity, raise},
    theme::ActiveTheme as _,
};
use ely_platform::AppId;
use std::path::{Path, PathBuf};

use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement,
    Keystroke, ParentElement, Render, SharedString, Styled as _, Window, div,
    prelude::FluentBuilder as _,
};

use super::Shell;
use crate::commands::{
    self, Export, New, Open, OpenElyAgent, OpenElyDoc, OpenElyMail, OpenElyMark, OpenElyPdf,
    OpenElyShow, OpenElyTable, OpenElyTree, Print, Save, SaveAs, ToggleCommandPalette,
};

/// The shared root: shell, file commands, command palette.
pub struct Root {
    app: AppId,
    focus: FocusHandle,
    palette: bool,
    recent: Vec<SharedString>,
    last: Option<SharedString>,
    toaster: Entity<Toaster>,
    content: Option<gpui::AnyView>,
    document: bool,
    file: &'static [&'static commands::Command],
}

/// A window with only its frame.
pub fn blank(app: AppId, window: &mut Window, cx: &mut App) -> Entity<Root> {
    root(app, None, window, cx)
}

pub fn root(
    app: AppId,
    content: Option<gpui::AnyView>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<Root> {
    build(app, content, false, &commands::FILE, Vec::new(), window, cx)
}

/// A tool window: File has New and Open; `menus` follow.
pub fn tool_root(
    app: AppId,
    content: gpui::AnyView,
    menus: Vec<gpui::Menu>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<Root> {
    build(
        app,
        Some(content),
        false,
        &commands::OPENING,
        menus,
        window,
        cx,
    )
}

/// A document window: File holds `file`; `menus` follow.
pub fn document_root(
    app: AppId,
    document: gpui::AnyView,
    file: &'static [&'static commands::Command],
    menus: Vec<gpui::Menu>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<Root> {
    build(app, Some(document), true, file, menus, window, cx)
}

/// Replaces a document window's menus.
pub fn set_document_menus(
    app: AppId,
    file: &'static [&'static commands::Command],
    menus: Vec<gpui::Menu>,
    cx: &mut App,
) {
    cx.set_menus(commands::menus(app, file, true, menus, cx));
}

fn build(
    app: AppId,
    content: Option<gpui::AnyView>,
    document: bool,
    file: &'static [&'static commands::Command],
    menus: Vec<gpui::Menu>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<Root> {
    cx.set_menus(commands::menus(app, file, document, menus, cx));
    cx.new(|cx| {
        let focus = cx.focus_handle();
        // A document takes focus itself.
        if !document {
            window.focus(&focus, cx);
        }
        if let Some(updates) = crate::updates::updater(cx) {
            cx.subscribe(
                &updates,
                |root: &mut Root, _, news: &crate::updates::UpdateNews, cx| match news {
                    crate::updates::UpdateNews::Failed(error) => {
                        let text = crate::i18n::text(cx, "updates.failed", &[("error", error)]);
                        root.toast(text, Severity::Danger, cx);
                    }
                    crate::updates::UpdateNews::Latest => {
                        let args = [
                            ("app", root.app.name()),
                            ("version", env!("CARGO_PKG_VERSION")),
                        ];
                        let text = crate::i18n::text(cx, "updates.latest", &args);
                        root.toast(text, Severity::Success, cx);
                    }
                },
            )
            .detach();
            // A failed install from the last quit says why.
            if let Some(note) = updates.update(cx, |updates, _| updates.note.take()) {
                let text = crate::i18n::text(cx, "updates.failed", &[("error", &note)]);
                cx.defer_in(window, move |root: &mut Root, _, cx| {
                    root.toast(text, Severity::Danger, cx)
                });
            }
        }
        Root {
            app,
            focus,
            palette: false,
            recent: Vec::new(),
            last: None,
            toaster: cx.new(|_| Toaster::default()),
            content,
            document,
            file,
        }
    })
}

impl Root {
    pub fn palette_open(&self) -> bool {
        self.palette
    }

    pub fn recent(&self) -> &[SharedString] {
        &self.recent
    }

    pub fn last(&self) -> Option<&SharedString> {
        self.last.as_ref()
    }

    fn attempt(&mut self, id: &'static str, cx: &mut Context<Self>) {
        if !self.refused(id, cx) {
            self.ran(id, cx);
        }
    }

    /// Runs a palette pick; views act first, so check here.
    pub fn run_from_palette(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.refused(id, cx) {
            commands::run(id, window, cx);
        }
    }

    /// Whether `id` is blocked here; the reason shows.
    fn refused(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        let Some(reason) = commands::blocked(id, self.document, cx) else {
            return false;
        };
        log::info!("command: {id} blocked: {reason}");
        self.last = Some(format!("{id} blocked").into());
        self.toaster.update(cx, |toaster, cx| {
            toaster.push(Toast::new(reason).severity(Severity::Warning), cx);
        });
        cx.notify();
        true
    }

    /// Shows `text` in the window's toaster.
    pub fn toast(
        &mut self,
        text: impl Into<SharedString>,
        severity: Severity,
        cx: &mut Context<Self>,
    ) {
        self.toaster.update(cx, |toaster, cx| {
            toaster.push(Toast::new(text.into()).severity(severity), cx);
        });
    }

    /// The toasts on show: title and severity, oldest first.
    pub fn toasts(&self, cx: &gpui::App) -> Vec<(SharedString, Option<Severity>)> {
        self.toaster.read(cx).shown()
    }

    /// A toast whose button opens the output `path` (§16).
    pub fn toast_output(
        &mut self,
        text: impl Into<SharedString>,
        severity: Severity,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let root = cx.weak_entity();
        let open = crate::i18n::text(cx, "toast.open", &[]);
        let toast = Toast::new(text.into())
            .severity(severity)
            .action(open, move |_, cx| {
                let (root, path) = (root.clone(), path.clone());
                // Opened off the UI thread; a failure comes back.
                cx.spawn(async move |cx| {
                    let opened = cx.background_spawn(async move { view(&path) }).await;
                    if let Err(error) = opened {
                        log::error!("apps: the output did not open: {error}");
                        cx.update(|cx| {
                            let said = crate::i18n::text(cx, "toast.open_failed", &[("error", &error.to_string())]);
                            root.update(cx, |root, cx| root.toast(said, Severity::Danger, cx)).ok();
                        });
                    }
                })
                .detach();
            });
        self.toaster.update(cx, |toaster, cx| {
            toaster.push(toast, cx);
        });
    }

    /// Starts another Ely app; a failure says why.
    fn switch(&mut self, app: AppId, cx: &mut Context<Self>) {
        cx.spawn(async move |root, cx| {
            let opened = cx.background_spawn(async move { ely_platform::apps::open(app, &[]) }).await;
            if let Err(error) = opened {
                log::error!("apps: {} did not open: {error}", app.name());
                root.update(cx, |root, cx| {
                    let said = crate::i18n::text(cx, "menu.app.switch_failed", &[("app", app.name()), ("error", &error.to_string())]);
                    root.toast(said, Severity::Danger, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    pub fn close_palette(&mut self, cx: &mut Context<Self>) {
        self.palette = false;
        cx.notify();
    }

    fn ran(&mut self, id: &'static str, cx: &mut Context<Self>) {
        log::info!("command: {id} ran in {}", self.app.name());
        self.last = Some(id.into());
        self.recent.retain(|value| value != id);
        self.recent.insert(0, id.into());
        self.recent.truncate(5);
        cx.notify();
    }
}

impl Render for Root {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let body = match &self.content {
            Some(view) => view.clone().into_any_element(),
            None => Shell::new(self.app.name(), &self.focus).into_any_element(),
        };
        let has = |id| self.file.iter().any(|command| command.id == id);
        // Text without its own color, overlays too, takes the theme's.
        let frame = div()
            .size_full()
            .text_color(cx.theme().colors.fg)
            .key_context(commands::CONTEXT)
            .on_action(cx.listener(|this, _: &New, _, cx| this.attempt("file.new", cx)))
            .on_action(cx.listener(|this, _: &Open, _, cx| this.attempt("file.open", cx)))
            .when(has("file.save"), |frame| {
                frame
                    .on_action(cx.listener(|this, _: &Save, _, cx| this.attempt("file.save", cx)))
                    .on_action(
                        cx.listener(|this, _: &SaveAs, _, cx| this.attempt("file.save_as", cx)),
                    )
                    .on_action(
                        cx.listener(|this, _: &Export, _, cx| this.attempt("file.export", cx)),
                    )
                    .on_action(cx.listener(|this, _: &Print, _, cx| this.attempt("file.print", cx)))
            })
            .on_action(cx.listener(|this, _: &OpenElyDoc, _, cx| this.switch(AppId::Doc, cx)))
            .on_action(cx.listener(|this, _: &OpenElyTable, _, cx| this.switch(AppId::Table, cx)))
            .on_action(cx.listener(|this, _: &OpenElyShow, _, cx| this.switch(AppId::Show, cx)))
            .on_action(cx.listener(|this, _: &OpenElyPdf, _, cx| this.switch(AppId::Pdf, cx)))
            .on_action(cx.listener(|this, _: &OpenElyMail, _, cx| this.switch(AppId::Mail, cx)))
            .on_action(cx.listener(|this, _: &OpenElyTree, _, cx| this.switch(AppId::Tree, cx)))
            .on_action(cx.listener(|this, _: &OpenElyMark, _, cx| this.switch(AppId::Mark, cx)))
            .on_action(cx.listener(|this, _: &OpenElyAgent, _, cx| this.switch(AppId::Agent, cx)))
            .on_action(cx.listener(|this, _: &ToggleCommandPalette, window, cx| {
                if this.palette {
                    window.on_next_frame(|window, cx| {
                        window.dispatch_keystroke(
                            Keystroke {
                                key: "escape".into(),
                                ..Default::default()
                            },
                            cx,
                        );
                    });
                } else {
                    this.palette = true;
                    cx.notify();
                }
            }))
            .child(body);
        let frame = frame.child(raise(
            "toasts",
            ToastViewport::new("toast-viewport", &self.toaster),
        ));
        if !self.palette {
            return frame.into_any_element();
        }
        let run = cx.entity();
        let palette = CommandPalette::new("command-palette", move |_, cx| {
            this.update(cx, |this, cx| this.close_palette(cx));
        })
        .group(
            crate::i18n::text(cx, "palette.group.file", &[]),
            self.file.iter().map(|c| {
                PaletteCommand::new(
                    c.id,
                    crate::i18n::text(cx, &format!("command.{}", c.id), &[]),
                )
                .keys(c.keys)
            }),
        )
        .recent(self.recent.clone())
        .on_run(move |value, window, cx| run.update(cx, |root, cx| root.run_from_palette(value, window, cx)));
        frame.child(raise("palette", palette)).into_any_element()
    }
}

/// PDFs open in ElyPDF if installed; others, the system's app.
fn view(path: &Path) -> std::io::Result<()> {
    let pdf = path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"));
    if pdf && ely_platform::apps::installed(AppId::Pdf) {
        return ely_platform::apps::open(AppId::Pdf, &[path.to_path_buf()]);
    }
    ely_platform::apps::open_default(path)
}

#[cfg(test)]
mod tests {
    use gpui::{FocusHandle, InteractiveElement as _, IntoElement, ParentElement as _, Render, Styled as _, TestAppContext, div};

    /// A focused view that counts the saves it gets.
    struct Saves(FocusHandle, usize);

    impl Render for Saves {
        fn render(&mut self, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
            div().track_focus(&self.0).on_action(cx.listener(|saves, _: &crate::commands::Save, _, _| saves.1 += 1))
        }
    }

    #[gpui::test]
    fn a_blocked_palette_pick_never_reaches_the_view(cx: &mut TestAppContext) {
        cx.update(|cx| {
            ely_gpui_component::init_for_tests(cx);
            crate::settings::install_defaults(cx);
            crate::i18n::install_as("en-US", cx);
        });
        let window = cx.add_window(|_, cx| Saves(cx.focus_handle(), 0));
        let (saves, root) = window
            .update(cx, |view, window, cx| {
                let saves = cx.entity();
                let root = super::tool_root(ely_platform::AppId::Tree, saves.clone().into(), Vec::new(), window, cx);
                // The root takes focus as it builds; give it back.
                window.focus(&view.0, cx);
                (saves, root)
            })
            .expect("the window lives");
        cx.run_until_parked();
        // Save without a document is blocked.
        window
            .update(cx, |_, window, cx| root.update(cx, |root, cx| root.run_from_palette("file.save", window, cx)))
            .expect("the window lives");
        cx.run_until_parked();
        assert_eq!(cx.update(|cx| saves.read(cx).1), 0, "the view never gets it");
        assert_eq!(cx.update(|cx| root.read(cx).toasts(cx).len()), 1, "the reason shows");
    }

    type Seen = std::rc::Rc<std::cell::Cell<Option<gpui::Hsla>>>;

    /// Records its drawn text color, and an overlay's.
    struct Inked(Seen, Seen);

    fn ink(seen: Seen) -> impl IntoElement {
        gpui::canvas(move |_, window, _| seen.set(Some(window.text_style().color)), |_, _, _, _| {})
    }

    impl Render for Inked {
        fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(ink(self.0.clone()))
                .child(ely_gpui_component::primitives::raise("inked-overlay", ink(self.1.clone())))
        }
    }

    #[gpui::test]
    fn text_without_a_color_takes_the_theme_overlays_too(cx: &mut TestAppContext) {
        use ely_gpui_component::theme::{ActiveTheme as _, Mode, Theme};
        cx.update(|cx| {
            ely_gpui_component::init_for_tests(cx);
            crate::settings::install_defaults(cx);
            crate::i18n::install_as("en-US", cx);
            Theme::set_mode_now(Mode::Dark, cx);
        });
        let (page, overlay) = (Seen::default(), Seen::default());
        let (shown_page, shown_overlay) = (page.clone(), overlay.clone());
        let window = cx
            .update(|cx| {
                cx.open_window(Default::default(), |window, cx| {
                    let inked = gpui::AppContext::new(cx, |_| Inked(shown_page, shown_overlay));
                    super::tool_root(ely_platform::AppId::Tree, inked.into(), Vec::new(), window, cx)
                })
            })
            .expect("a window");
        let cx = &mut gpui::VisualTestContext::from_window(window.into(), cx);
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        let fg = cx.update(|_, cx| cx.theme().colors.fg);
        assert_ne!(fg, gpui::black(), "dark text would hide on a dark page");
        assert_eq!(page.get(), Some(fg), "the page");
        assert_eq!(overlay.get(), Some(fg), "an overlay over it");
    }
}
