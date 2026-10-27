//! Scrolling the page view (LOK-04).

use std::rc::Rc;

use ely_gpui_component::layout::{ScrollModel, ScrollPlace, ScrollSource, Scrollbar};
use gpui::{
    App, Axis, Bounds, Context, Entity, Pixels, Point, ScrollDelta, ScrollEvent, Window,
    point, px,
};

use super::{LokView, input::Input};
use crate::{Mouse, Rect};

impl LokView {
    /// Scrolls by view pixels, clamped to the document (LOK-04).
    pub(super) fn scroll_by(&mut self, dx: f32, dy: f32, cx: &mut Context<Self>) {
        self.scroll.0 -= self.geom.twips_of(dx);
        self.scroll.1 -= self.geom.twips_of(dy);
        self.clamp_scroll();
        cx.notify();
    }

    /// Keeps the viewport inside the document (LOK-04).
    pub(super) fn clamp_scroll(&mut self) {
        let (vw, vh) = (
            self.geom.twips_of(self.viewport.0),
            self.geom.twips_of(self.viewport.1),
        );
        let extent = self.extent();
        self.scroll.0 = self.scroll.0.clamp(0, (extent.0 - vw).max(0));
        self.scroll.1 = self.scroll.1.clamp(0, (extent.1 - vh).max(0));
    }

    /// Scrolls the wheel in twips, clamped to the document (LOK-04).
    pub(super) fn on_scroll(
        &mut self,
        event: &ScrollEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (dx, dy) = match event.delta {
            ScrollDelta::Pixels(point) => (f32::from(point.x), f32::from(point.y)),
            ScrollDelta::Lines(point) => (point.x * 40.0, point.y * 40.0),
        };
        self.scroll_by(dx, dy, cx);
    }

    /// A newly selected object, such as a chart, shows.
    pub(super) fn follow_graphic(&mut self, frame: Option<Rect>, cx: &mut Context<Self>) {
        if let (None, Some(frame), true) = (self.graphic, frame, self.viewport.1 > 0.0) {
            self.show(frame, cx);
        }
    }

    /// Scrolls print-twips `rect` into view; frozen parts always show.
    pub fn show(&mut self, rect: Rect, cx: &mut Context<Self>) {
        let (x, y, w, h) = self.tile_rect(rect);
        let (fx, fy) = self.frozen_tiles();
        let (vw, vh) = (
            self.geom.twips_of(self.viewport.0) - fx,
            self.geom.twips_of(self.viewport.1) - fy,
        );
        // Only a sheet grows where it jumps.
        if self.sheet.is_some() {
            let need = self.print_of((x + w + vw + fx, y + h + vh + fy));
            self.reach = (self.reach.0.max(need.0), self.reach.1.max(need.1));
        }
        if x >= fx {
            self.scroll.0 = least(x - fx, w, self.scroll.0, vw);
        }
        if y >= fy {
            self.scroll.1 = least(y - fy, h, self.scroll.1, vh);
        }
        self.clamp_scroll();
        cx.notify();
    }

    /// Freezes what lies before `frozen`, in print twips.
    pub fn set_frozen(&mut self, frozen: (i32, i32), cx: &mut Context<Self>) {
        if self.frozen != frozen {
            log::info!("lok: frozen at {frozen:?}");
            self.frozen = frozen;
            cx.notify();
        }
    }

    /// Scrolls `rect` into view and puts the cursor there.
    pub fn reveal(&mut self, rect: Rect, cx: &mut Context<Self>) {
        self.show(rect, cx);
        // Clicks land in tile twips.
        let (x, y, _, h) = self.tile_rect(rect);
        log::info!("lok: reveal at {x},{y}");
        for kind in [0, 1] {
            let (count, buttons, modifiers) = (1, 1, 0);
            let y = y + h / 2;
            self.send(Input::Mouse(Mouse {
                kind,
                x,
                y,
                count,
                buttons,
                modifiers,
            }));
        }
    }

    /// The scroll offset in twips.
    pub fn scroll(&self) -> (i32, i32) {
        self.scroll
    }

    /// What the document holds, before a sheet's room ahead.
    fn content(&self) -> (i32, i32) {
        // A fitted page is the page, whatever the cursor reached.
        if self.fit {
            return self.doc_size;
        }
        // The kernel sizes a sheet in tile twips already.
        let reach = self.tile_of(self.reach);
        (self.doc_size.0.max(reach.0), self.doc_size.1.max(reach.1))
    }

    /// The document extent the view may scroll over.
    pub(super) fn extent(&self) -> (i32, i32) {
        let (w, h) = self.content();
        let Some(sheet) = self.sheet.as_ref().filter(|_| !self.fit) else {
            return (w, h);
        };
        // Sheets scroll ahead of the kernel, to their end.
        let end = self.tile_of((sheet.columns.start(sheet.columns.count()), sheet.rows.start(sheet.rows.count())));
        let (fx, fy) = self.frozen_tiles();
        let ahead = (
            fx + self.scroll.0 + 2 * self.geom.twips_of(self.viewport.0),
            fy + self.scroll.1 + 2 * self.geom.twips_of(self.viewport.1),
        );
        (w.max(ahead.0.min(end.0)), h.max(ahead.1.min(end.1)))
    }

    /// The print-twips document point under a view position (LOK-04).
    pub fn doc_point(&self, position: Point<Pixels>) -> (i32, i32) {
        self.print_of(self.tile_point(position))
    }

    /// The tile-twips point under a view position.
    pub(super) fn tile_point(&self, position: Point<Pixels>) -> (i32, i32) {
        let local = position - self.origin;
        let (ax, ay) = self.axes();
        (
            ax.doc(self.geom.twips_of(f32::from(local.x) - self.spare.0)),
            ay.doc(self.geom.twips_of(f32::from(local.y) - self.spare.1)),
        )
    }

    /// The panes of each axis.
    pub(super) fn axes(&self) -> (super::panes::Axis, super::panes::Axis) {
        let axis = |frozen, scroll| super::panes::Axis { frozen, scroll };
        let (fx, fy) = self.frozen_tiles();
        (axis(fx, self.scroll.0), axis(fy, self.scroll.1))
    }

    /// Whether a print-twips rectangle intersects the visible panes.
    pub fn is_visible(&self, rect: Rect) -> bool {
        let rect = self.tile_rect(rect);
        rect.2 > 0 && rect.3 > 0 && self.tile_visible(rect)
    }

    pub(super) fn tile_visible(&self, (x, y, w, h): Rect) -> bool {
        let (ax, ay) = self.axes();
        ax.visible(x, w, self.geom.twips_of(self.viewport.0))
            && ay.visible(y, h, self.geom.twips_of(self.viewport.1))
    }

    /// The window point of a print-twips document point (LOK-04).
    pub fn window_point(&self, at: (i32, i32)) -> gpui::Point<Pixels> {
        self.tile_window_point(self.tile_of(at))
    }

    /// The window point of a tile-twips point.
    pub(super) fn tile_window_point(&self, (x, y): (i32, i32)) -> gpui::Point<Pixels> {
        let (ax, ay) = self.axes();
        let (x, y) = (self.geom.px_of(ax.show(x)), self.geom.px_of(ay.show(y)));
        self.origin + point(px(self.spare.0 + x), px(self.spare.1 + y))
    }

    /// A document rect in window pixels, panes applied.
    pub(super) fn screen_rect(&self, (x, y, w, h): crate::Rect) -> Bounds<Pixels> {
        Bounds::from_corners(self.window_point((x, y)), self.window_point((x + w, y + h)))
    }

    /// A tile-twips rect in window pixels.
    pub(super) fn tile_screen_rect(&self, (x, y, w, h): crate::Rect) -> Bounds<Pixels> {
        Bounds::from_corners(self.tile_window_point((x, y)), self.tile_window_point((x + w, y + h)))
    }

    /// The cursor rect in window pixels.
    pub fn cursor_bounds(&self) -> Option<Bounds<Pixels>> {
        let mut bounds = self.text_caret()?;
        bounds.size.width = bounds.size.width.max(px(self.geom.px_of(1)));
        Some(bounds)
    }
}

/// The view's scroll, as its bars read and move it.
struct Kept(Entity<LokView>);

impl ScrollModel for Kept {
    fn place(&self, axis: Axis, cx: &App) -> Option<ScrollPlace> {
        let view = self.0.read(cx);
        let (span, full, at) = view.along(axis);
        let shown = view.geom.twips_of(span);
        // Content, or as far as the view went.
        let full = full.max(at + shown);
        (span > 0.0 && full > shown).then(|| ScrollPlace {
            span: px(span),
            shown: shown as f32 / full as f32,
            passed: at as f32 / (full - shown) as f32,
        })
    }

    fn set(&self, axis: Axis, ratio: f32, cx: &mut App) {
        self.0.update(cx, |view, cx| {
            let (span, full, at) = view.along(axis);
            let shown = view.geom.twips_of(span);
            let to = (ratio * (full.max(at + shown) - shown) as f32) as i32;
            match axis {
                Axis::Horizontal => view.scroll.0 = to,
                Axis::Vertical => view.scroll.1 = to,
            }
            view.clamp_scroll();
            cx.notify();
        });
    }

    fn offset(&self, cx: &App) -> Point<Pixels> {
        let (x, y) = self.0.read(cx).scroll;
        point(px(x as f32), px(y as f32))
    }
}

impl LokView {
    /// View pixels, content and scroll twips along `axis`.
    fn along(&self, axis: Axis) -> (f32, i32, i32) {
        let content = self.content();
        match axis {
            Axis::Horizontal => (self.viewport.0, content.0, self.scroll.0),
            Axis::Vertical => (self.viewport.1, content.1, self.scroll.1),
        }
    }

    /// A bar each way; none where everything shows.
    pub(super) fn bars(this: &Entity<Self>) -> [Scrollbar; 2] {
        let kept = Rc::new(Kept(this.clone()));
        let id = this.entity_id().as_u64();
        [
            Scrollbar::new(("lok-bar-x", id), ScrollSource::Model(kept.clone()), Axis::Horizontal),
            Scrollbar::new(("lok-bar-y", id), ScrollSource::Model(kept), Axis::Vertical),
        ]
    }
}

/// The scroll nearest `scroll` that shows `start..start + len`.
fn least(start: i32, len: i32, scroll: i32, view: i32) -> i32 {
    // A span wider than the view keeps covering it.
    let end = start + len - view;
    scroll.clamp(start.min(end), start.max(end))
}
