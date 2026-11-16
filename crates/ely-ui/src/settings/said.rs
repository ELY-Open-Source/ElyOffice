//! A row's own words: an error or a result.

use ely_gpui_component::{primitives::Severity, theme::ActiveTheme as _};
use gpui::{AnyElement, App, InteractiveElement as _, IntoElement as _, ParentElement as _, Role, SharedString, StatefulInteractiveElement as _, Styled as _, div};

/// `words` under a row, in `severity`'s color.
pub(crate) fn line(id: impl Into<SharedString>, severity: Severity, words: impl Into<SharedString>, cx: &App) -> AnyElement {
    let colors = &cx.theme().colors;
    let color = match severity {
        Severity::Danger => colors.danger,
        Severity::Warning => colors.warning,
        Severity::Success => colors.success,
        Severity::Info => colors.fg_muted,
    };
    // Read aloud as the banner it replaced was.
    let role = match severity {
        Severity::Danger | Severity::Warning => Role::Alert,
        Severity::Info | Severity::Success => Role::Status,
    };
    let (id, words) = (id.into(), words.into());
    div()
        .id(id.clone())
        .debug_selector(move || id.to_string())
        .role(role)
        .aria_label(words.clone())
        .text_color(color)
        .child(words)
        .into_any_element()
}
