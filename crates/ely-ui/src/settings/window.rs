//! The settings window: shared settings and this app's own.

use ely_gpui_component::{
    buttons::{Button, SegmentedControl},
    feedback::Banner,
    layout::ScrollArea,
    forms::{FontPicker, Input, InputEvent, Switch, TextInput},
    primitives::{IconName, Severity},
    settings::{self as kit, SettingsLayout, SettingsRow, SettingsSection, ThemeSelector},
    theme::ActiveTheme as _,
};
use ely_platform::{
    AppId,
    settings::{
        APPEARANCE, AUTHOR, Appearance, CHECK_UPDATES, LANGUAGE, Language, MOTION, Motion, OMARCHY_SYNC, Store, UI_FONT, word,
    },
};
use gpui::{
    AppContext as _, Context, Entity, FocusHandle, Global, InteractiveElement as _, IntoElement, ParentElement, Render, SharedString, Styled,
    Subscription, Window, WindowHandle, div,
};
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use super::{current, set};
use crate::{i18n::text, shell::Shell, updates::Checked};

/// The open settings window, one per app.
#[derive(Clone, Copy)]
struct Opened(Option<WindowHandle<SettingsView>>);

impl Global for Opened {}

/// Opens the settings window, or brings it forward.
pub fn open(app: AppId, cx: &mut gpui::App) {
    let shown = cx.try_global::<Opened>().and_then(|opened| opened.0);
    if let Some(handle) = shown
        && handle.update(cx, |_, window, _| window.activate_window()).is_ok()
    {
        return;
    }
    let mut options = crate::run::window_options(app, gpui::size(gpui::px(720.0), gpui::px(520.0)), cx);
    if let Some(bar) = options.titlebar.as_mut() {
        bar.title = Some(text(cx, "settings.title", &[]));
    }
    match cx.open_window(options, |window, cx| cx.new(|cx| SettingsView::new(app, window, cx))) {
        Ok(handle) => cx.set_global(Opened(Some(handle))),
        Err(error) => log::error!("settings: the window did not open: {error:#}"),
    }
}

pub struct SettingsView {
    pub(super) app: AppId,
    focus: FocusHandle,
    pub(super) section: SharedString,
    author: Entity<TextInput>,
    /// The name last read from or saved to the file.
    author_saved: String,
    /// The latest author commit; older writes are dropped.
    author_asked: Arc<AtomicU64>,
    ui_font: Entity<TextInput>,
    git: Option<Entity<TextInput>>,
    git_error: Option<SharedString>,
    /// The latest Git commit; older checks are dropped.
    git_asked: Arc<AtomicU64>,
    pub(super) diagnostics: super::diagnostics::Diagnostics,
    pub(super) storage: super::storage::Storage,
    _fields: Vec<Subscription>,
}

impl SettingsView {
    pub(super) fn new(app: AppId, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let author = cx.new(|cx| TextInput::new(window, cx).label(text(cx, "settings.author", &[])));
        let account = ely_platform::user::account().unwrap_or_else(|error| {
            log::warn!("settings: no account name: {error}");
            String::new()
        });
        let named = current(cx).author.clone();
        author.update(cx, |field, cx| {
            field.set_placeholder(account, cx);
            field.set_text(named.clone(), cx);
        });
        let mut fields = vec![
            cx.subscribe(&author, |this, field, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::Submit) {
                    this.commit_author(&field, cx);
                }
            }),
            cx.observe_global::<super::Current>(|this, cx| this.follow_author(cx)),
        ];
        // A check's outcome shows as it lands.
        if let Some(updates) = crate::updates::updater(cx) {
            fields.push(cx.observe(&updates, |_, _, cx| cx.notify()));
        }
        let git = (app == AppId::Tree).then(|| cx.new(|cx| TextInput::new(window, cx).label(text(cx, "settings.git.path", &[]))));
        let mut git_error = None;
        if let Some(git) = &git {
            let stored = git_setting().unwrap_or_else(|error| {
                log::error!("settings: the Git setting is unreadable: {error}");
                git_error = Some(text(cx, "settings.unreadable", &[("error", &error.to_string())]));
                String::new()
            });
            git.update(cx, |field, cx| field.set_text(stored, cx));
            fields.push(cx.subscribe(git, |this, field, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::Submit) {
                    this.commit_git(&field, cx);
                }
            }));
        }
        let ui_font = cx.new(|cx| TextInput::new(window, cx).label(text(cx, "settings.font", &[])));
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            app,
            focus,
            section: "general".into(),
            author,
            author_saved: named,
            author_asked: Arc::default(),
            ui_font,
            git,
            git_error,
            git_asked: Arc::default(),
            diagnostics: Default::default(),
            storage: Default::default(),
            _fields: fields,
        }
    }

    /// A name written elsewhere replaces an unedited field.
    fn follow_author(&mut self, cx: &mut Context<Self>) {
        let named = current(cx).author.clone();
        let shown = self.author.read(cx).text().trim().to_string();
        if named != self.author_saved && shown == self.author_saved {
            self.author.update(cx, |field, cx| field.set_text(named.clone(), cx));
            self.author_saved = named;
        }
    }

    /// Writes an edit; a failed one stays for retry.
    fn commit_author(&mut self, field: &Entity<TextInput>, cx: &mut Context<Self>) {
        let typed = field.read(cx).text().trim().to_string();
        // Every commit outdates the writes still waiting.
        let asked = self.author_asked.fetch_add(1, Ordering::SeqCst) + 1;
        if typed == self.author_saved {
            return;
        }
        let turn = (Arc::clone(&self.author_asked), asked);
        let write = super::saved(AUTHOR, Value::String(typed.clone()), Some(turn), cx);
        cx.spawn(async move |this, cx| {
            if write.await {
                this.update(cx, |this, _| this.author_saved = typed).ok();
            }
        })
        .detach();
    }

    /// Checks a system Git, then keeps it; latest wins.
    fn commit_git(&mut self, field: &Entity<TextInput>, cx: &mut Context<Self>) {
        let typed = field.read(cx).text().trim().to_string();
        // Every commit outdates the checks still running.
        let asked = self.git_asked.fetch_add(1, Ordering::SeqCst) + 1;
        if git_setting().is_ok_and(|stored| stored == typed) {
            self.git_error = None;
            return cx.notify();
        }
        let latest = Arc::clone(&self.git_asked);
        let check = cx.background_executor().spawn({
            let typed = typed.clone();
            async move {
                if !typed.is_empty() {
                    ely_platform::git::system_version(std::path::Path::new(&typed))?;
                }
                // Under the file lock: a later commit skips this.
                let still = || latest.load(Ordering::SeqCst) == asked;
                Store::app(AppId::Tree)?.set_if(ely_platform::git::PROGRAM, Value::String(typed), still)
            }
        });
        cx.spawn(async move |this, cx| {
            let checked = check.await;
            this.update(cx, |this, cx| {
                if this.git_asked.load(Ordering::SeqCst) != asked {
                    return log::info!("settings: Git check {asked} was overtaken");
                }
                this.git_error = checked.err().map(|error| {
                    log::warn!("settings: Git setting refused: {error}");
                    text(cx, "settings.git.bad", &[("path", &typed), ("error", &error.to_string())])
                });
                cx.notify();
            })
        })
        .detach();
    }

    pub(super) fn sections(&self, cx: &gpui::App) -> Vec<(&'static str, SharedString, IconName)> {
        let mut sections = vec![("general", text(cx, "settings.general", &[]), IconName::Settings)];
        if matches!(self.app, AppId::Doc | AppId::Table | AppId::Show | AppId::Pdf | AppId::Mark) {
            sections.push(("documents", text(cx, "settings.documents", &[]), IconName::FileText));
        }
        if self.git.is_some() {
            sections.push(("git", text(cx, "settings.git", &[]), IconName::GitBranch));
        }
        if self.app == AppId::Mail {
            sections.push(("mail", text(cx, "settings.mail", &[]), IconName::Bell));
        }
        sections.push(("storage", text(cx, "settings.storage", &[]), IconName::HardDrive));
        sections.push(("diagnostics", text(cx, "settings.diagnostics", &[]), IconName::Bug));
        sections
    }

    fn general(&self, cx: &gpui::App) -> SettingsSection {
        let shared = current(cx);
        let appearance = match shared.appearance {
            Appearance::System => kit::Appearance::System,
            Appearance::Light => kit::Appearance::Light,
            Appearance::Dark => kit::Appearance::Dark,
        };
        let theme = ThemeSelector::new("settings-appearance", appearance, |chosen, _, cx| {
            let chosen = match chosen {
                kit::Appearance::System => Appearance::System,
                kit::Appearance::Light => Appearance::Light,
                kit::Appearance::Dark => Appearance::Dark,
            };
            set(APPEARANCE, word(&Appearance::ALL, &chosen).into(), cx);
        });
        let language = choices("settings-language", &Language::ALL, &shared.language, |choice, cx| match choice {
            Language::System => text(cx, "settings.language.system", &[]),
            Language::English => "English".into(),
            Language::Chinese => "简体中文".into(),
        }, LANGUAGE, cx);
        let motion = choices("settings-motion", &Motion::ALL, &shared.motion, |choice, cx| match choice {
            Motion::System => text(cx, "settings.motion.system", &[]),
            Motion::Reduced => text(cx, "settings.motion.reduce", &[]),
            Motion::Full => text(cx, "settings.motion.full", &[]),
        }, MOTION, cx);
        let section = SettingsSection::new(text(cx, "settings.general", &[]))
            .row(SettingsRow::new(text(cx, "settings.appearance", &[])).control(theme))
            .row(
                SettingsRow::new(text(cx, "settings.language", &[]))
                    .description(text(cx, "settings.language.next", &[]))
                    .control(language),
            )
            .row(SettingsRow::new(text(cx, "settings.motion", &[])).control(motion))
            .row(SettingsRow::new(text(cx, "settings.font", &[])).control(self.font(cx)))
            .row(SettingsRow::new(text(cx, "settings.text.size", &[])).control(super::picks::text_size(shared.ui_text_size, cx)));
        let mut section = match crate::updates::updater(cx) {
            Some(updates) => self.updates(section, &updates.read(cx).checked, shared.check_updates, cx),
            // A dev build does not update itself.
            None => section,
        };
        // Only where Omarchy is installed.
        if ely_platform::omarchy_current_dir().is_some() {
            let follow = Switch::new("settings-omarchy", shared.omarchy_sync)
                .name(text(cx, "settings.omarchy", &[]))
                .on_change(|on, _, cx| set(OMARCHY_SYNC, Value::Bool(on), cx));
            section = section.row(SettingsRow::new(text(cx, "settings.omarchy", &[])).control(follow));
        }
        section
    }

    /// The system's families; the one in force is chosen.
    fn font(&self, cx: &gpui::App) -> impl IntoElement {
        let picker = FontPicker::new("settings-ui-font", &self.ui_font)
            .selected(match super::picks::waiting(UI_FONT, cx) {
                Some(Value::String(family)) => family.into(),
                _ => cx.theme().font_family.clone(),
            })
            .on_change(|family, _, cx| super::picks::pick(UI_FONT, Value::String(family.to_string()), cx));
        div().w_full().max_w_64().child(picker)
    }

    /// The launch switch, and a check the user starts.
    fn updates(&self, section: SettingsSection, checked: &Option<Checked>, at_launch: bool, cx: &gpui::App) -> SettingsSection {
        let switch = Switch::new("settings-updates", at_launch)
            .name(text(cx, "settings.updates", &[]))
            .on_change(|on, _, cx| set(CHECK_UPDATES, Value::Bool(on), cx));
        let now = Button::new("settings-check-now", text(cx, "settings.check.now", &[]))
            .disabled(matches!(checked, Some(Checked::Asking)))
            .on_click(|_, _, cx| crate::updates::check_now(cx));
        section
            .row(
                SettingsRow::new(text(cx, "settings.updates", &[]))
                    .description(text(cx, "settings.updates.hint", &[]))
                    .control(switch),
            )
            .row(SettingsRow::new(text(cx, "settings.check", &[])).description(status(self.app, checked, cx)).control(
                div().debug_selector(|| "settings-check-now".into()).child(now),
            ))
    }

    fn page(&self, this: gpui::WeakEntity<Self>, cx: &gpui::App) -> gpui::AnyElement {
        match self.section.as_ref() {
            "documents" => {
                let printing = SettingsRow::new(text(cx, "settings.two_sided", &[]))
                    .description(text(cx, "settings.two_sided.hint", &[]))
                    .control(div().debug_selector(|| "settings-two-sided".into()).w_full().max_w_64().child(super::picks::two_sided(current(cx).two_sided, cx)));
                let section = SettingsSection::new(text(cx, "settings.documents", &[]));
                // ElyMark prints, but has no author, autosave or zoom.
                if self.app == AppId::Mark {
                    return section.row(printing).into_any_element();
                }
                let autosave = Switch::new("settings-autosave", super::switches::on(super::switches::AUTOSAVE, cx))
                    .name(text(cx, "settings.autosave", &[]))
                    .on_change(|on, _, cx| super::switches::set(super::switches::AUTOSAVE, on, cx));
                section
                    .row(
                        SettingsRow::new(text(cx, "settings.author", &[]))
                            .description(text(cx, "settings.author.hint", &[]))
                            .control(div().w_full().max_w_64().child(Input::new(&self.author))),
                    )
                    .row(
                        SettingsRow::new(text(cx, "settings.autosave", &[]))
                            .description(text(cx, "settings.autosave.hint", &[]))
                            .control(autosave),
                    )
                    .row(
                        SettingsRow::new(text(cx, "settings.zoom", &[]))
                            .description(text(cx, "settings.zoom.hint", &[]))
                            .control(div().w_full().max_w_64().child(super::picks::doc_zoom(current(cx).doc_zoom, cx))),
                    )
                    .row(printing)
                    .into_any_element()
            }
            "git" => {
                let field = self.git.clone().expect("the git section is ElyTree's");
                let section = SettingsSection::new(text(cx, "settings.git", &[])).row(
                    SettingsRow::new(text(cx, "settings.git.path", &[]))
                        .description(text(cx, "settings.git.hint", &[]))
                        .control(div().w_full().max_w_64().child(Input::new(&field).invalid(self.git_error.is_some()))),
                );
                match self.git_error.clone() {
                    Some(error) => section.row(Banner::new("settings-git-error", Severity::Danger, error)),
                    None => section,
                }
                .into_any_element()
            }
            "mail" => {
                let notify = Switch::new("settings-notify", super::switches::on(super::switches::NOTIFY, cx))
                    .name(text(cx, "settings.notify", &[]))
                    .on_change(|on, _, cx| super::switches::set(super::switches::NOTIFY, on, cx));
                SettingsSection::new(text(cx, "settings.mail", &[]))
                    .row(
                        SettingsRow::new(text(cx, "settings.notify", &[]))
                            .description(text(cx, "settings.notify.hint", &[]))
                            .control(notify),
                    )
                    .into_any_element()
            }
            "storage" => self.storage_page(this, cx),
            "diagnostics" => self.diagnostics_page(this, cx),
            _ => self.general(cx).into_any_element(),
        }
    }
}

/// A segmented choice that writes `key` when picked.
fn choices<T: Copy + PartialEq + 'static>(
    id: &'static str,
    all: &'static [(T, &'static str)],
    chosen: &T,
    label: impl Fn(&T, &gpui::App) -> SharedString,
    key: &'static str,
    cx: &gpui::App,
) -> SegmentedControl {
    all.iter()
        .fold(SegmentedControl::new(id, word(all, chosen)), |control, (choice, word)| {
            control.segment(*word, label(choice, cx), None)
        })
        .on_change(move |picked, _, cx| set(key, Value::String(picked.to_string()), cx))
}

/// This version and the last check's outcome.
pub(super) fn status(app: AppId, checked: &Option<Checked>, cx: &gpui::App) -> SharedString {
    let mut args = vec![("app", app.name()), ("version", env!("CARGO_PKG_VERSION"))];
    let (newer, error);
    let key = match checked {
        None => "settings.check.current",
        Some(Checked::Asking) => "settings.check.asking",
        Some(Checked::Latest) => "settings.check.latest",
        Some(Checked::Newer(version)) => {
            newer = version.to_string();
            args.push(("newer", &newer));
            "settings.check.newer"
        }
        Some(Checked::Failed(said)) => {
            error = said.clone();
            args.push(("error", &error));
            "settings.check.failed"
        }
    };
    text(cx, key, &args)
}

/// ElyTree's Git program; empty is its own.
fn git_setting() -> std::io::Result<String> {
    match Store::app(AppId::Tree)?.read()?.get(ely_platform::git::PROGRAM) {
        None => Ok(String::new()),
        Some(Value::String(path)) => Ok(path.clone()),
        Some(other) => Err(std::io::Error::other(format!("settings: git cannot be {other}"))),
    }
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sections = self.sections(cx);
        let this = cx.entity().downgrade();
        let page = self.page(this.clone(), cx);
        let dialog = self.storage_dialog(this.clone(), cx);
        let layout = SettingsLayout::new("settings", sections, self.section.clone(), move |key, _, cx| {
            let key = key.clone();
            this.update(cx, |this, cx| {
                if key == "diagnostics" {
                    this.read_diagnostics(cx);
                }
                if key == "storage" {
                    this.read_storage(cx);
                }
                this.section = key;
                cx.notify();
            })
            .ok();
        })
        .page(page);
        let current = cx.global::<super::Current>();
        let unreadable = current.error.clone().map(|error| {
            Banner::new("settings-unreadable", Severity::Danger, text(cx, "settings.unreadable", &[("error", &error)]))
        });
        let app_error: Vec<_> = current
            .app_error
            .iter()
            .map(|(kind, error)| {
                let id = SharedString::from(format!("settings-app-unreadable-{kind}"));
                Banner::new(id, Severity::Danger, text(cx, "settings.app_unreadable", &[("error", error)]))
            })
            .collect();
        let unsaved: Vec<_> = current
            .unsaved
            .iter()
            .map(|(kind, error)| {
                Banner::new(SharedString::from(format!("settings-unsaved-{kind}")), Severity::Danger, text(cx, "settings.unsaved", &[("error", error)]))
            })
            .collect();
        Shell::new(text(cx, "settings.title", &[]), &self.focus)
            .secondary()
            .child(
                div().size_full().p_6().flex().flex_col().gap_4().children(unreadable).children(app_error).children(unsaved).child(
                    ScrollArea::new("settings-scroll").vertical().flex_1().min_h_0().child(layout),
                ),
            )
            .children(dialog)
    }
}
