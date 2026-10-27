//! Kernel-bound controls the property panels share.

use ely_gpui_component::{
    buttons::{Button, ToggleButton, ToggleItem},
    forms::{Choice, ColorPalette, ColorSwatch, Combobox, Select, TextInput},
    overlays::Popover,
    primitives::{IconName, Tooltip},
    theme::{ActiveTheme as _, ControlSize},
};
use ely_lok::show::Paint;
use gpui::{
    AnyElement, App, Context, Entity, Hsla, InteractiveElement as _, IntoElement,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    rgb,
};

/// Sizes the size menu offers, in points.
const SIZES: [&str; 17] = [
    "8", "9", "10", "10.5", "11", "12", "14", "16", "18", "20", "22", "24", "26", "28", "36", "48",
    "72",
];

/// Palette colors, 0xRRGGBB.
pub const COLORS: [u32; 10] = [
    0x000000, 0x808080, 0xFF0000, 0xFF8000, 0xFFFF00, 0x00A933, 0x00FFFF, 0x2A6099, 0x800080,
    0xFFFFFF,
];

/// A panel that edits one document through table commands.
pub trait Panel: Sized + 'static {
    /// The kernel's state for `uno`.
    fn state(&self, uno: &str, cx: &App) -> Option<SharedString>;
    /// Runs table command `id` on the document.
    fn run(&mut self, id: &str, values: &[&str], cx: &mut Context<Self>);
}

fn text(cx: &App, key: &str) -> SharedString {
    ely_ui::i18n::text(cx, key, &[])
}

fn uno_of(command: &str) -> &'static str {
    ely_lok::commands::find(command)
        .expect("a table command")
        .uno
}

/// Runs `command` with `values` through `panel`.
pub fn send<P: Panel>(panel: &Entity<P>, command: &'static str, values: &[&str], cx: &mut App) {
    panel.update(cx, |panel, cx| panel.run(command, values, cx));
}

/// A kernel-lit toggle; `why` explains it when disabled.
pub fn toggle<P: Panel>(
    panel: &P,
    command: &'static str,
    (icon, label): (Option<IconName>, &str),
    (tip, why): (&str, &str),
    cx: &Context<P>,
) -> AnyElement {
    let this = cx.entity();
    let state = panel.state(uno_of(command), cx);
    let mut item = ToggleItem::new(command);
    if let Some(icon) = icon {
        item = item.icon(icon);
    }
    let shown = match state.as_deref() == Some(ely_lok::MIXED) {
        true => format!("{label} {}", text(cx, "office.props.mixed")),
        false => label.to_string(),
    };
    if !shown.trim().is_empty() {
        item = item.label(shown.trim().to_string());
    }
    let disabled = state.as_deref() == Some("disabled");
    if disabled {
        item = item.tooltip(text(cx, why));
    } else if !tip.is_empty() {
        item = item.tooltip(text(cx, tip));
    }
    ToggleButton::new(
        SharedString::from(command),
        item,
        state.as_deref() == Some("true"),
    )
    .disabled(disabled)
    .size(ControlSize::Sm)
    .on_toggle(move |_, _, cx| send(&this, command, &[], cx))
    .into_any_element()
}

/// A button for `command`; `why` explains it when disabled.
pub fn button<P: Panel>(
    panel: &P,
    command: &'static str,
    key: &str,
    why: &str,
    cx: &Context<P>,
) -> AnyElement {
    let this = cx.entity();
    let off = panel.state(uno_of(command), cx).as_deref() == Some("disabled");
    let button = Button::new(SharedString::from(command), text(cx, key))
        .size(ControlSize::Sm)
        .on_click(move |_, _, cx| send(&this, command, &[], cx));
    guarded(button, command, off.then(|| text(cx, why)))
}

/// A clear button: label key, command, value.
pub type Clear = (&'static str, &'static str, &'static str);

/// A color swatch; `empty` names no color.
pub fn palette<P: Panel>(
    panel: &Entity<P>,
    command: &'static str,
    (current, empty): (Paint, &str),
    clear: Option<Clear>,
    cx: &App,
) -> AnyElement {
    let (shown, tip) = match current {
        Paint::Color(color) => (Hsla::from(rgb(color)), format!("#{color:06X}").into()),
        Paint::Mixed => (gpui::transparent_black(), text(cx, "office.props.mixed")),
        Paint::None => (gpui::transparent_black(), text(cx, empty)),
    };
    let label = (current == Paint::Mixed).then(|| muted(tip.clone(), cx));
    let clear = clear.map(|(key, run, value)| (text(cx, key), run, value));
    let this = panel.clone();
    let popover = Popover::with_opener(
        SharedString::from(format!("{command}-popover")),
        move |toggle| {
            ColorSwatch::new(SharedString::from(format!("{command}-swatch")), shown)
                .tooltip(tip)
                .on_click(move |window, cx| toggle(window, cx))
        },
        move |close, _, _| {
            let swatches = COLORS.map(|color| (format!("#{color:06X}"), Hsla::from(rgb(color))));
            let picked = this.clone();
            let mut palette = ColorPalette::new(SharedString::from(command), swatches.clone())
                .columns(5)
                .on_change({
                    let close = close.clone();
                    move |color, window, cx| {
                        let at = swatches
                            .iter()
                            .position(|(_, swatch)| *swatch == color)
                            .expect("the palette hands back its own colors");
                        close(window, cx);
                        send(&picked, command, &[&COLORS[at].to_string()], cx)
                    }
                });
            if let Paint::Color(color) = current
                && COLORS.contains(&color)
            {
                palette = palette.selected(rgb(color));
            }
            let this = this.clone();
            let clear = clear.clone().map(|(label, run, value)| {
                Button::new(SharedString::from(format!("{command}-clear")), label)
                    .size(ControlSize::Sm)
                    .on_click(move |_, window, cx| {
                        close(window, cx);
                        send(&this, run, &[value], cx)
                    })
            });
            div()
                .flex()
                .flex_col()
                .gap_2()
                .p_2()
                .child(palette)
                .children(clear)
        },
    );
    row([popover.into_any_element()].into_iter().chain(label))
}

/// The kernel's state for `uno` as a color.
pub fn color_state<P: Panel>(panel: &P, uno: &str, cx: &App) -> Paint {
    let state = panel.state(uno, cx);
    if state.as_deref() == Some(ely_lok::MIXED) {
        return Paint::Mixed;
    }
    state
        .and_then(|color| color.parse::<u32>().ok())
        .map_or(Paint::None, Paint::Color)
}

/// The font box; `id` keys its text field.
pub fn font_box<P: Panel>(
    panel: &P,
    (id, label): (&'static str, SharedString),
    fonts: &[SharedString],
    window: &mut Window,
    cx: &mut Context<P>,
) -> AnyElement {
    let this = cx.entity();
    let input = window.use_keyed_state(
        SharedString::from(format!("{id}-input")),
        cx,
        TextInput::new,
    );
    let choices = fonts
        .iter()
        .map(|font| Choice::new(font.clone(), font.clone()));
    let mut font = Combobox::new(id, &input, choices)
        .size(ControlSize::Sm)
        .on_change(move |value, _, cx| send(&this, "format.font", &[value.as_ref()], cx));
    let name = panel.state(".uno:CharFontName", cx);
    let hint = match name.as_deref() == Some(ely_lok::MIXED) {
        true => text(cx, "office.props.mixed"),
        false => SharedString::default(),
    };
    if input.read(cx).label_text() != &label {
        input.update(cx, |input, cx| input.set_label(label, cx));
    }
    if input.read(cx).placeholder_text() != &hint {
        input.update(cx, |input, cx| input.set_placeholder(hint, cx));
    }
    if let Some(name) = name.filter(|name| known(name)) {
        font = font.selected(name);
    }
    font.into_any_element()
}

/// The size menu, in points.
pub fn size_menu<P: Panel>(panel: &P, (id, name): (&'static str, SharedString), cx: &Context<P>) -> AnyElement {
    let this = cx.entity();
    let reported = panel.state(".uno:FontHeight", cx);
    let size = reported.clone().filter(|size| known(size));
    let mut sizes: Vec<SharedString> = SIZES.iter().map(|size| (*size).into()).collect();
    if let Some(size) = size.clone().filter(|size| !sizes.contains(size)) {
        sizes.push(size);
    }
    let sizes = ordered(sizes);
    let mut menu = Select::new(
        id,
        sizes
            .into_iter()
            .map(|size| Choice::new(size.clone(), size)),
    )
    .size(ControlSize::Sm)
    .label(name)
    .on_change(move |value, _, cx| send(&this, "format.size", &[value.as_ref()], cx));
    if reported.as_deref() == Some(ely_lok::MIXED) {
        menu = menu.placeholder(text(cx, "office.props.mixed"));
    }
    if let Some(size) = size {
        menu = menu.selected(size);
    }
    menu.into_any_element()
}

/// Controls side by side, wrapping when narrow.
pub fn row(items: impl IntoIterator<Item = AnyElement>) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_1()
        .children(items)
        .into_any_element()
}

/// `button`, or it disabled with `why` on hover.
pub fn guarded(button: Button, id: &str, why: Option<SharedString>) -> AnyElement {
    match why {
        None => button.into_any_element(),
        Some(why) => div()
            .id(SharedString::from(format!("{id}-why")))
            .child(button.disabled(true))
            .tooltip(Tooltip::text(why))
            .into_any_element(),
    }
}

/// Quiet text in place of a control.
pub fn muted(text: SharedString, cx: &App) -> AnyElement {
    div()
        .text_color(cx.theme().colors.fg_muted)
        .child(text)
        .into_any_element()
}

/// A real kernel value: neither empty nor mixed.
pub fn known(state: &str) -> bool {
    !state.is_empty() && state != ely_lok::MIXED
}

/// Sizes by points; one the kernel words oddly shows last.
fn ordered(sizes: Vec<SharedString>) -> Vec<SharedString> {
    let (mut sized, mut odd) = (Vec::new(), Vec::new());
    for size in sizes {
        match size.parse::<f64>() {
            Ok(points) => sized.push((points, size)),
            Err(_) => odd.push(size),
        }
    }
    sized.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut sizes: Vec<SharedString> = sized.into_iter().map(|(_, size)| size).collect();
    sizes.append(&mut odd);
    sizes
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_odd_size_shows_last_and_panics_nothing() {
        let sizes = ["12", "disabled", "9", "10.5"]
            .map(gpui::SharedString::from)
            .to_vec();
        assert_eq!(super::ordered(sizes), ["9", "10.5", "12", "disabled"]);
    }
}
