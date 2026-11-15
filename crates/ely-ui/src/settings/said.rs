//! A row's own words: an error or a result.

use ely_gpui_component::{primitives::Severity, theme::ActiveTheme as _};
use gpui::{AnyElement, App, InteractiveElement as _, IntoElement as _, ParentElement as _, SharedString, Styled as _, div};

/// `words` under a row, in `severity`'s color.
pub(super) fn line(id: impl Into<SharedString>, severity: Severity, words: impl Into<SharedString>, cx: &App) -> AnyElement {
    let colors = &cx.theme().colors;
    let color = match severity {
        Severity::Danger => colors.danger,
        Severity::Warning => colors.warning,
        Severity::Success => colors.success,
        Severity::Info => colors.fg_muted,
    };
    let id = id.into();
    div().debug_selector(move || id.to_string()).text_color(color).child(words.into()).into_any_element()
}
