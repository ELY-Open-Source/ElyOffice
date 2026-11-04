//! Zooming the page view about the pointer (LOK-04).

use gpui::{Context, EventEmitter, PinchEvent, Pixels, Point, ScrollDelta, ScrollEvent, Window};

use super::LokView;

/// The zoom changed, by key, wheel or pinch.
pub struct Zoomed;

impl EventEmitter<Zoomed> for LokView {}

/// The steps of zooming in and out.
pub const ZOOM_STEPS: [f32; 10] = [0.25, 0.5, 0.67, 0.8, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0];
const LEAST: f32 = 0.25;
const MOST: f32 = 4.0;

impl LokView {
    /// The zoom now; 1.0 is the page's actual size.
    pub fn zoom(&self) -> f32 {
        self.geom.zoom
    }

    /// Whether the zoom follows the viewport (a whole page).
    pub fn fitting(&self) -> bool {
        self.fit
    }

    /// Sets the zoom; the grid is repainted (LOK-04).
    pub fn set_zoom(&mut self, zoom: f32, cx: &mut Context<Self>) {
        self.zoom_at(zoom, None, cx);
    }

    /// The next step in or out from the zoom now.
    pub fn zoom_step(&mut self, larger: bool, cx: &mut Context<Self>) {
        self.zoom_at(next_step(self.geom.zoom, larger), None, cx);
    }

    /// Zooms keeping `at` (window pixels) still; none keeps the anchor.
    pub fn zoom_at(&mut self, zoom: f32, at: Option<Point<Pixels>>, cx: &mut Context<Self>) {
        // A fit below the least stays reachable.
        let zoom = zoom.clamp(LEAST.min(self.geom.zoom), MOST);
        let sheet = self.doc.is_sheet();
        let (mut ax, mut ay) = match at {
            Some(at) => (f32::from(at.x - self.origin.x), f32::from(at.y - self.origin.y)),
            // A sheet keeps its top-left cell, a page its middle.
            None if sheet => (0.0, 0.0),
            None => (self.viewport.0 / 2.0, self.viewport.1 / 2.0),
        };
        // The offset now, not the last frame's: zooms may bunch.
        let spare = self.slack();
        let (tx, ty) = (self.geom.twips_of(ax - spare.0), self.geom.twips_of(ay - spare.1));
        // Over frozen cells, the scrolling pane's start anchors.
        let (fx, fy) = self.frozen_tiles();
        let (pin_x, pin_y) = (tx < fx, ty < fy);
        // Anchored in print twips: tile twips change with the zoom.
        let mut under = self.print_of((self.scroll.0 + tx.max(fx), self.scroll.1 + ty.max(fy)));
        // A pointed sheet keeps the pointed cell's corner still.
        if let (Some(cells), Some(_)) = (self.sheet.as_ref(), at) {
            let corner = (cells.columns.start(cells.columns.at(under.0)), cells.rows.start(cells.rows.at(under.1)));
            let to = self.tile_of(corner);
            if !pin_x {
                (ax, under.0) = (self.geom.px_of(to.0 - self.scroll.0) + spare.0, corner.0);
            }
            if !pin_y {
                (ay, under.1) = (self.geom.px_of(to.1 - self.scroll.1) + spare.1, corner.1);
            }
        }
        self.fit = false;
        if (zoom - self.geom.zoom).abs() > 0.0001 {
            self.carry_size(|view| view.geom.zoom = zoom);
            self.retire_tiles();
            self.failed.clear();
        }
        let (under, spare, (fx, fy)) = (self.tile_of(under), self.slack(), self.frozen_tiles());
        let x = if pin_x { fx } else { self.geom.twips_of(ax - spare.0) };
        let y = if pin_y { fy } else { self.geom.twips_of(ay - spare.1) };
        self.scroll = (under.0 - x, under.1 - y);
        // Keys and menus keep the page's left edge in view.
        if let (None, false, Some(page)) = (at, sheet, self.pages.first()) {
            self.scroll.0 = self.scroll.0.min(self.tile_of((page.0, page.1)).0);
        }
        self.clamp_scroll();
        log::debug!("lok: zoom {:.0}%, view starts at {:?}", zoom * 100.0, self.print_of((fx + self.scroll.0, fy + self.scroll.1)));
        cx.emit(Zoomed);
        cx.notify();
    }

    /// Ctrl (macOS: Cmd) wheel zooms; else it turns or scrolls.
    pub(super) fn on_wheel(&mut self, event: &ScrollEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.wheel_zoom(event, cx) && !self.turn_wheel(event) {
            self.on_scroll(event, window, cx);
        }
    }

    /// A notch is one step; a trackpad zooms smoothly.
    pub(super) fn wheel_zoom(&mut self, event: &ScrollEvent, cx: &mut Context<Self>) -> bool {
        if !event.modifiers.secondary(cx) {
            return false;
        }
        let zoom = match event.delta {
            ScrollDelta::Lines(lines) if lines.y == 0.0 => return true,
            ScrollDelta::Lines(lines) => next_step(self.geom.zoom, lines.y > 0.0),
            ScrollDelta::Pixels(point) if point.y == gpui::px(0.0) => return true,
            ScrollDelta::Pixels(point) => self.geom.zoom * (f32::from(point.y) / 200.0).exp(),
        };
        self.zoom_at(zoom, Some(event.position), cx);
        true
    }

    pub(super) fn on_pinch(&mut self, event: &PinchEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.pinch_zoom(event, cx);
    }

    /// A trackpad pinch zooms about its centre.
    pub(super) fn pinch_zoom(&mut self, event: &PinchEvent, cx: &mut Context<Self>) {
        // Start and end carry no magnification.
        if event.delta == 0.0 {
            return;
        }
        let zoom = self.geom.zoom * (1.0 + event.delta);
        self.zoom_at(zoom, Some(event.position), cx);
    }
}

/// The next step in or out from `now`.
fn next_step(now: f32, larger: bool) -> f32 {
    match larger {
        true => ZOOM_STEPS.into_iter().find(|step| *step > now + 0.001).unwrap_or(MOST),
        false => ZOOM_STEPS.into_iter().rev().find(|step| *step < now - 0.001).unwrap_or(LEAST.min(now)),
    }
}
