//! Zooming a document: keys, View menu items, the status control.

use ely_gpui_component::secondary;
use ely_gpui_component::shell::{ZoomControl, ZoomStep};
use ely_ui::commands;
use gpui::{Action, App, Context, Div, InteractiveElement as _, KeyBinding, MenuItem, actions};

use crate::{Host, Tabs as _, lok::Lok, text};

actions!(office, [ZoomIn, ZoomOut, ActualSize]);

/// The document apps' zoom keys.
pub fn bind_zoom_keys(cx: &mut App) {
    bind_zooms(cx, (ZoomIn, ZoomOut, ActualSize), Some(commands::CONTEXT));
}

/// Command with =, - and 0; menus show these.
pub fn bind_zooms(cx: &mut App, (zoom_in, zoom_out, actual): (impl Action + Clone, impl Action + Clone, impl Action), at: Option<&str>) {
    let shown = [
        KeyBinding::new(secondary!("="), zoom_in.clone(), at),
        KeyBinding::new(secondary!("-"), zoom_out.clone(), at),
        KeyBinding::new(secondary!("0"), actual, at),
    ];
    // A layout's own + key; Linux's keypad names.
    let more = [
        KeyBinding::new(secondary!("+"), zoom_in.clone(), at),
        KeyBinding::new(secondary!("add"), zoom_in, at),
        KeyBinding::new(secondary!("subtract"), zoom_out, at),
    ];
    // macOS menus show the first binding, others the last.
    if cfg!(target_os = "macos") {
        cx.bind_keys(shown);
        cx.bind_keys(more);
    } else {
        cx.bind_keys(more);
        cx.bind_keys(shown);
    }
}

/// Zoom In, Zoom Out, and Actual Size (or Fit Slide).
pub fn zoom_items(fits: bool, cx: &App) -> [MenuItem; 3] {
    let actual = if fits { "office.zoom.fit" } else { "office.zoom.actual" };
    [
        MenuItem::action(text(cx, "office.zoom.in", &[]), ZoomIn),
        MenuItem::action(text(cx, "office.zoom.out", &[]), ZoomOut),
        MenuItem::action(text(cx, actual, &[]), ActualSize),
    ]
}

/// The active document's zoom, by key, menu or status control.
pub trait Zooming: Host<Engine = Lok> {
    /// Actual size fits a whole page instead (ElyShow).
    const FITS: bool = false;

    fn zoom(&mut self, step: ZoomStep, cx: &mut Context<Self>) {
        let Some(view) = self.active_tab().map(|tab| tab.view.clone()) else {
            return;
        };
        view.update(cx, |view, cx| match step {
            ZoomStep::In => view.zoom_step(true, cx),
            ZoomStep::Out => view.zoom_step(false, cx),
            ZoomStep::Reset if Self::FITS => view.fit_page(cx),
            ZoomStep::Reset => view.set_zoom(1.0, cx),
        });
        cx.notify();
    }

    /// The window root, answering the zoom actions.
    fn bind_zoom(root: Div, cx: &mut Context<Self>) -> Div {
        root.on_action(cx.listener(|host: &mut Self, _: &ZoomIn, _, cx| host.zoom(ZoomStep::In, cx)))
            .on_action(cx.listener(|host: &mut Self, _: &ZoomOut, _, cx| host.zoom(ZoomStep::Out, cx)))
            .on_action(cx.listener(|host: &mut Self, _: &ActualSize, _, cx| host.zoom(ZoomStep::Reset, cx)))
    }

    /// Minus, the percent, plus; none without a document.
    fn zoom_control(&self, cx: &mut Context<Self>) -> Option<ZoomControl> {
        let view = self.active_tab()?.view.clone();
        let percent = (view.read(cx).zoom() * 100.0).round() as u32;
        let this = cx.entity().downgrade();
        Some(ZoomControl::new("office-zoom").owned(percent, move |step, _, cx| {
            if let Err(error) = this.update(cx, |host, cx| host.zoom(step, cx)) {
                log::warn!("office: zoom without a window: {error}");
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use super::*;

    #[test]
    fn the_menu_shows_equals_and_minus_and_every_zoom_key_works() {
        let cx = TestAppContext::single();
        cx.update(|app| {
            bind_zoom_keys(app);
            let keymap = app.key_bindings();
            let keymap = keymap.borrow();
            let keys = |action: &dyn Action| -> Vec<String> {
                (keymap.bindings_for_action(action))
                    .map(|binding| binding.keystrokes()[0].inner())
                    .inspect(|key| assert!(key.modifiers.secondary(app), "{key:?}"))
                    .map(|key| key.key.clone())
                    .collect()
            };
            let (zoom_in, zoom_out) = (keys(&ZoomIn), keys(&ZoomOut));
            // The menu's key: macOS takes the first, others the last.
            let shown = |keys: &[String]| if cfg!(target_os = "macos") { keys.first() } else { keys.last() }.cloned();
            assert_eq!(shown(&zoom_in).as_deref(), Some("="), "{zoom_in:?}");
            assert_eq!(shown(&zoom_out).as_deref(), Some("-"), "{zoom_out:?}");
            assert_eq!(shown(&keys(&ActualSize)).as_deref(), Some("0"));
            let sorted = |mut keys: Vec<String>| {
                keys.sort();
                keys
            };
            assert_eq!(sorted(zoom_in), ["+", "=", "add"]);
            assert_eq!(sorted(zoom_out), ["-", "subtract"]);
        });
    }
}
