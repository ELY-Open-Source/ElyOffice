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
        .child(div().w_48().child(slider))
        .child(format!("{size}%"))
        .child(reset);
    SettingsRow::new(label)
        .description(ely_ui::i18n::text(cx, "mark.settings.size.hint", &[]))
        .control(control)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use ely_gpui_component::settings::SettingsSection;
    use gpui::{IntoElement, Modifiers, ParentElement as _, Render, Styled as _, div, point, px};

    use crate::desk::tests::{open, start};

    /// The documents page as settings draws it; counts its draws.
    struct Page(Rc<Cell<usize>>);

    impl Render for Page {
        fn render(&mut self, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
            self.0.set(self.0.get() + 1);
            let rows = (cx.global::<ely_ui::settings::DocumentRows>().0)(cx);
            let section = rows.into_iter().fold(SettingsSection::new("Documents"), |section, row| section.row(row));
            div().w(px(720.)).child(section)
        }
    }

    #[gpui::test]
    fn the_drawn_slider_sets_the_size_and_follows_the_menu(cx: &mut gpui::TestAppContext) {
        let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
        let path = files.path().join("a.md");
        std::fs::write(&path, "a\n").expect("a file");
        let (desk, cx) = start(cx, data.path());
        open(&desk, &path, cx);
        let draws = Rc::new(Cell::new(0));
        let page = cx.add_window({
            let draws = draws.clone();
            move |_, _| Page(draws)
        });
        let mut shown = gpui::VisualTestContext::from_window(page.into(), cx);
        shown.update(|window, cx| window.draw(cx).clear(cx));
        let track = shown.debug_bounds("slider-root").expect("the slider shows");
        assert!(track.size.width > px(100.), "a usable track: {track:?}");
        let size = |shown: &mut gpui::VisualTestContext| shown.update(|_, cx| desk.read(cx).look.size);
        let y = track.center().y;
        // The track runs half a thumb in from each side.
        shown.simulate_click(point(track.right() - px(9.), y), Modifiers::none());
        assert_eq!(size(&mut shown), 200, "the far end");
        shown.update(|window, cx| window.draw(cx).clear(cx));
        shown.simulate_click(point(track.left() + px(9.), y), Modifiers::none());
        assert_eq!(size(&mut shown), 70, "the near end");
        shown.update(|window, cx| window.draw(cx).clear(cx));
        shown.run_until_parked();
        let before = draws.get();
        let focus = desk.read_with(cx, |desk, _| desk.core.focus.clone());
        cx.update(|window, cx| window.focus(&focus, cx));
        cx.dispatch_action(super::super::Larger);
        cx.run_until_parked();
        assert_eq!(desk.read_with(cx, |desk, _| desk.look.size), 80);
        assert!(draws.get() > before, "the open page draws the menu's size");
    }
}

