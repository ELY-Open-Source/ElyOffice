//! Zooming a document: keys, View menu items, the status control.

use ely_gpui_component::secondary;
use ely_gpui_component::shell::{ZoomControl, ZoomStep};
use ely_ui::commands;
use gpui::{App, Context, Div, InteractiveElement as _, KeyBinding, MenuItem, actions};

use crate::{Host, Tabs as _, lok::Lok, text};

actions!(office, [ZoomIn, ZoomOut, ActualSize]);

/// Command with =, - and 0, as Mac apps do.
pub fn bind_zoom_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new(secondary!("="), ZoomIn, Some(commands::CONTEXT)),
        KeyBinding::new(secondary!("-"), ZoomOut, Some(commands::CONTEXT)),
        KeyBinding::new(secondary!("0"), ActualSize, Some(commands::CONTEXT)),
    ]);
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
