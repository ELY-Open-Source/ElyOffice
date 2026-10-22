use ely_gpui_component::theme::{ActiveTheme as _, Mode, Theme};
use ely_platform::settings::{Appearance, Shared, UI_TEXT_SIZES};
use ely_platform::{accessibility, env, omarchy_current_dir};
use gpui::{App, Global, SharedString, Window, WindowAppearance};

pub mod omarchy;
mod watch;

use omarchy::OmarchyTheme;

/// `omarchy_sync` lets System follow Omarchy where present.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeSettings {
    pub appearance: Appearance,
    pub omarchy_sync: bool,
}

impl ThemeSettings {
    /// The settings, then `ELY_APPEARANCE` and `ELY_OMARCHY_SYNC` over them.
    fn of(shared: &Shared) -> Self {
        let appearance = match env::value("ELY_APPEARANCE").as_deref() {
            Some("light") => Appearance::Light,
            Some("dark") => Appearance::Dark,
            Some("system") => Appearance::System,
            None => shared.appearance,
            Some(other) => {
                log::warn!("theme: ELY_APPEARANCE={other} is not system, light or dark, ignored");
                shared.appearance
            }
        };
        Self {
            appearance,
            omarchy_sync: env::flag("ELY_OMARCHY_SYNC").unwrap_or(shared.omarchy_sync),
        }
    }
}

impl Global for ThemeSettings {}

/// The theme Omarchy last gave us.
#[derive(Default)]
struct OmarchyState(Option<OmarchyTheme>);

impl Global for OmarchyState {}

/// The theme's own font family, before any setting.
struct OwnFont(SharedString);

impl Global for OwnFont {}

/// Applies motion and appearance settings, and starts following Omarchy.
pub fn install(cx: &mut App) {
    cx.set_global(OwnFont(cx.theme().font_family.clone()));
    text(cx);
    motion(cx);
    cx.set_global(ThemeSettings::of(crate::settings::current(cx)));
    cx.set_global(OmarchyState::default());
    match omarchy_current_dir() {
        Some(dir) => watch::start(dir, cx),
        None => log::info!("theme: no Omarchy on this machine"),
    }
    apply(cx, true);
}

/// Follows changed settings: text, motion, appearance and Omarchy sync.
pub(crate) fn settings_changed(cx: &mut App) {
    text(cx);
    motion(cx);
    cx.set_global(ThemeSettings::of(crate::settings::current(cx)));
    apply(cx, false);
}

/// The interface font and its size.
fn text(cx: &mut App) {
    let shared = crate::settings::current(cx);
    let family: SharedString = match shared.ui_font.as_str() {
        "" => cx.global::<OwnFont>().0.clone(),
        chosen => chosen.to_string().into(),
    };
    let size = shared.ui_text_size;
    // Listing fonts is slow; only a chosen family needs it.
    if !shared.ui_font.is_empty() && !cx.text_system().all_font_names().iter().any(|name| *name == *family) {
        log::warn!("theme: no font family {family} on this system; text falls back");
    }
    Theme::update(cx, |theme| {
        theme.font_family = family.clone();
        theme.font_scale = size as f32 / UI_TEXT_SIZES.2 as f32;
    });
    log::info!("theme: interface text {family} at {size} px");
}

fn motion(cx: &mut App) {
    let reduce = accessibility::reduce_motion(crate::settings::current(cx).motion);
    cx.set_reduce_motion(reduce);
    Theme::update(cx, |theme| theme.reduced_motion = reduce);
    log::info!("theme: reduced motion {}", if reduce { "on" } else { "off" });
}

/// Re-applies the theme when the system appearance changes.
pub fn follow_system(window: &mut Window) {
    window
        .observe_window_appearance(|_, cx| apply(cx, false))
        .detach();
}

fn apply(cx: &mut App, at_once: bool) {
    let settings = cx.global::<ThemeSettings>().clone();
    let omarchy = cx.global::<OmarchyState>().0.clone();
    let set_mode = |mode: Mode, cx: &mut App| {
        if at_once {
            Theme::set_mode_now(mode, cx);
        } else {
            Theme::set_mode(mode, cx);
        }
    };

    if let Some(theme) =
        omarchy.filter(|_| settings.appearance == Appearance::System && settings.omarchy_sync)
    {
        let other = if theme.mode == Mode::Dark {
            Mode::Light
        } else {
            Mode::Dark
        };
        Theme::set_palette(theme.mode, Some(theme.palette.clone()), cx);
        Theme::set_palette(other, None, cx);
        cx.set_window_appearance(Some(window_appearance(theme.mode)));
        set_mode(theme.mode, cx);
        log::info!(
            "theme: omarchy {} applied, {:?}",
            theme.name.as_deref().unwrap_or("unnamed"),
            theme.mode
        );
        return;
    }

    Theme::set_palette(Mode::Light, None, cx);
    Theme::set_palette(Mode::Dark, None, cx);
    let mode = match settings.appearance {
        Appearance::System => {
            cx.set_window_appearance(None);
            Mode::from(cx.window_appearance())
        }
        Appearance::Light => {
            cx.set_window_appearance(Some(WindowAppearance::Light));
            Mode::Light
        }
        Appearance::Dark => {
            cx.set_window_appearance(Some(WindowAppearance::Dark));
            Mode::Dark
        }
    };
    set_mode(mode, cx);
}

fn window_appearance(mode: Mode) -> WindowAppearance {
    match mode {
        Mode::Light => WindowAppearance::Light,
        Mode::Dark => WindowAppearance::Dark,
    }
}
