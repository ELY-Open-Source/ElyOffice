//! ElyMark's text size on the settings' documents page.

use std::rc::Rc;

use ely_gpui_component::{buttons::Button, forms::Slider, settings::SettingsRow};
use gpui::{AnyElement, App, IntoElement as _, ParentElement as _, Styled as _, WeakEntity, div};

use super::SIZES;
use crate::desk::Desk;

/// The text size joins the settings' documents page (#85).
pub fn settings_rows(desk: WeakEntity<Desk>, cx: &mut App) {
    let rows = move |cx: &App| match desk.upgrade() {
        Some(this) => vec![size_row(this.read(cx).look.size, desk.clone(), cx)],
        None => Vec::new(),
    };
    ely_ui::settings::add_document_rows(Rc::new(rows), cx);
}

/// A slider over the menu's sizes, and a reset.
fn size_row(size: u32, desk: WeakEntity<Desk>, cx: &App) -> AnyElement {
    let (least, most, step) = SIZES;
    let set = move |size: u32, cx: &mut App| {
        if desk.update(cx, |desk, cx| desk.size_to(size, cx)).is_err() {
            log::error!("mark: the size {size} has no window to take it");
        }
    };
    let label = ely_ui::i18n::text(cx, "mark.settings.size", &[]);
    let slider = Slider::new("mark-settings-size", f64::from(size))
        .range(f64::from(least), f64::from(most))
        .step(f64::from(step))
        .label(label.clone())
        .on_change({
            let set = set.clone();
            move |value, _, cx| set(value.round() as u32, cx)
        });
    let reset = Button::new("mark-settings-size-reset", ely_ui::i18n::text(cx, "mark.look.actual", &[]))
        .disabled(size == 100)
        .on_click(move |_, _, cx| set(100, cx));
    let control = div()
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .max_w_64()
        .child(div().flex_1().child(slider))
        .child(format!("{size}%"))
        .child(reset);
    SettingsRow::new(label)
        .description(ely_ui::i18n::text(cx, "mark.settings.size.hint", &[]))
        .control(control)
        .into_any_element()
}
