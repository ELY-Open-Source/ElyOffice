//! Only the pages on screen, drawn in tiles (PDF-01, PERF-05).

use std::sync::Arc;

use ely_gpui_component::theme::ActiveTheme as _;
use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _,
    Pixels, Point, RenderImage, StatefulInteractiveElement as _, Styled as _, Window, canvas, div,
    point, px,
};
use image::{Frame, ImageBuffer, Rgba};

use crate::layout::{GAP, Layout, Mode, Placed, layout};
use crate::view::{PdfView, Reported};

/// A tile's side in device pixels.
pub const TILE: i32 = 512;

/// A page tile at one scale and turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TileKey {
    pub page: usize,
    /// Device pixels a point, times a thousand.
    pub scale: u32,
    pub rotate: i32,
    pub tx: i32,
    pub ty: i32,
}

/// A page point in its turned frame, still in points.
pub fn to_view((x, y): (f32, f32), (w, h): (f32, f32), rotate: i32) -> (f32, f32) {
    match rotate.rem_euclid(360) {
        90 => (h - y, x),
        180 => (w - x, h - y),
        270 => (y, w - x),
        _ => (x, y),
    }
}

/// The page point under a turned-frame point.
pub fn from_view((x, y): (f32, f32), (w, h): (f32, f32), rotate: i32) -> (f32, f32) {
    match rotate.rem_euclid(360) {
        90 => (y, h - x),
        180 => (w - x, h - y),
        270 => (w - y, x),
        _ => (x, y),
    }
}

/// A page rectangle on screen, relative to its placed page.
pub fn rect_on(rect: [f32; 4], size: (f32, f32), rotate: i32, scale: f32) -> (f32, f32, f32, f32) {
    let a = to_view((rect[0], rect[1]), size, rotate);
    let b = to_view((rect[2], rect[3]), size, rotate);
    let (x0, y0) = (a.0.min(b.0), a.1.min(b.1));
    let (x1, y1) = (a.0.max(b.0), a.1.max(b.1));
    (x0 * scale, y0 * scale, (x1 - x0) * scale, (y1 - y0) * scale)
}

/// Tiles covering `[from, to)` view pixels of a `side`-pixel page.
pub(crate) fn span(from: f32, to: f32, sf: f32, side: i32) -> std::ops::RangeInclusive<i32> {
    let first = (from.max(0.0) * sf).floor() as i32 / TILE;
    let last = ((to * sf).ceil() as i32 - 1).min(side - 1).max(0) / TILE;
    first..=last
}

/// Draws `image`, logging failure and first success (PERF-05).
fn drawn_tile(image: Arc<RenderImage>, page: usize, view: gpui::Entity<PdfView>) -> gpui::Canvas<()> {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, cx| match window.paint_image(bounds, bounds, Default::default(), image, 0, false) {
            Ok(()) => view.update(cx, |view, _| {
                let area = f64::from(f32::from(bounds.size.width) * f32::from(bounds.size.height));
                if view.painted == 0.0 && area > 0.0 {
                    log::info!("pdf: first tile drawn, page {page}");
                }
                view.painted += area;
            }),
            Err(error) => log::error!("pdf: drawing a page failed: {error}"),
        },
    )
}

/// A tile as an image; GPUI takes BGRA.
pub(crate) fn picture((w, h): (u32, u32), bgra: Vec<u8>) -> Option<Arc<RenderImage>> {
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(w, h, bgra)?;
    Some(Arc::new(RenderImage::new([Frame::new(image)])))
}

impl PdfView {
    /// The layout in the room last painted.
    pub fn laid_out(&self) -> Layout {
        let room = (f32::from(self.room.size.width), f32::from(self.room.size.height));
        layout(&self.sizes, self.rotate, (self.mode, self.zoom), room, self.current)
    }

    /// Percent of actual size, as the default zoom.
    pub fn percent(&self) -> u32 {
        (self.laid_out().scale / crate::layout::ACTUAL * 100.0).round() as u32
    }

    /// A window point as (page, point on it), if any.
    pub fn page_at(&self, at: Point<Pixels>) -> Option<(usize, (f32, f32))> {
        let laid = self.laid_out();
        let offset = self.scroll.offset();
        let x = f32::from(at.x - self.room.origin.x - offset.x);
        let y = f32::from(at.y - self.room.origin.y - offset.y);
        let place = laid.placed.iter().find(|p| x >= p.x && x < p.x + p.w && y >= p.y && y < p.y + p.h)?;
        let local = ((x - place.x) / laid.scale, (y - place.y) / laid.scale);
        Some((place.page, from_view(local, self.sizes[place.page], self.rotate)))
    }

    /// Freezes `page`'s place in the window for a gesture.
    pub fn grip(&mut self, page: usize) {
        let (laid, offset) = (self.laid_out(), self.scroll.offset());
        self.grip = laid.placed.iter().find(|p| p.page == page).map(|place| {
            let origin = point(self.room.origin.x + offset.x + px(place.x), self.room.origin.y + offset.y + px(place.y));
            (page, origin, laid.scale)
        });
    }

    /// A window point on the gripped page, as pressed.
    pub fn held_at(&self, at: Point<Pixels>) -> Option<(usize, (f32, f32))> {
        let (page, origin, scale) = self.grip?;
        let local = (f32::from(at.x - origin.x) / scale, f32::from(at.y - origin.y) / scale);
        Some((page, from_view(local, self.sizes[page], self.rotate)))
    }

    /// Pages in view, and which; missing tiles asked.
    pub fn page_view(&mut self, window: &mut Window, cx: &mut Context<Self>) -> (AnyElement, Vec<usize>) {
        let sf = window.scale_factor();
        let (sunken, paper) = (cx.theme().colors.sunken, gpui::white());
        let laid = self.laid_out();
        if let Some((x, y)) = self.go_to.take() {
            self.scroll.set_offset(point(px(-x), px(-y)));
        }
        let (left, top) = (-f32::from(self.scroll.offset().x), -f32::from(self.scroll.offset().y));
        let showing = self.showing(sf);
        // Half a page past a jump's lead counts.
        let ahead = |p: &Placed| match self.mode {
            Mode::Horizontal => p.x + p.w > left + (GAP * 3.0).min(GAP + p.w / 2.0),
            _ => p.y + p.h > top + (GAP * 3.0).min(GAP * 2.0 + p.h / 2.0),
        };
        let room = (f32::from(self.room.size.width), f32::from(self.room.size.height));
        // Pages past the end never reach the edge.
        let ended = match self.mode {
            Mode::Horizontal => left + room.0 >= laid.size.0 - 0.5,
            _ => top + room.1 >= laid.size.1 - 0.5,
        };
        // The current page: first showing more than a sliver.
        if let Some(first) = showing.iter().map(|s| s.place).find(|p| ahead(p)) {
            let held = showing.iter().any(|s| s.place.page == self.current);
            let now = match self.mode {
                Mode::Single => self.current,
                // Either page of a pair in view may be current.
                Mode::Two if held && first.page / 2 == self.current / 2 => self.current,
                _ if ended && showing.iter().any(|s| s.place.page == self.current && ahead(&s.place)) => self.current,
                _ => first.page,
            };
            if now != self.current {
                self.current = now;
                cx.emit(Reported);
            }
        }
        let wanted: Vec<(TileKey, [i32; 4])> = showing
            .iter()
            .flat_map(|s| s.keys.iter().copied())
            .filter(|(key, _)| !self.tiles.contains_key(key) && !self.failed.contains(key))
            .collect();
        let pages: Vec<(Placed, Vec<AnyElement>)> = showing
            .iter()
            .map(|s| {
                let (w, h) = (s.place.w, s.place.h);
                let tiles = self
                    .pieces(s)
                    .into_iter()
                    .map(|(image, at)| {
                        drawn_tile(image, s.place.page, cx.entity())
                            .absolute()
                            .left(px(at[0] * w))
                            .top(px(at[1] * h))
                            .w(px(at[2] * w))
                            .h(px(at[3] * h))
                            .into_any_element()
                    })
                    .collect();
                (s.place, tiles)
            })
            .collect();
        self.settle(&showing);
        let pages_shown: Vec<usize> = showing.iter().map(|s| s.place.page).collect();
        self.ask_layers(&pages_shown, cx);
        self.ask_notes(pages_shown.iter().copied(), cx);
        self.paint(wanted, cx);
        let pages: Vec<AnyElement> = pages
            .into_iter()
            .map(|(place, tiles)| {
                let mut marks = self.page_marks(&place, laid.scale, cx);
                marks.extend(self.edit_marks(&place, laid.scale, cx));
                marks.extend(self.region_marks(&place, laid.scale, cx));
                marks.extend(self.note_marks(&place, laid.scale, cx));
                div()
                    .absolute()
                    .left(px(place.x))
                    .top(px(place.y))
                    .w(px(place.w))
                    .h(px(place.h))
                    .bg(paper)
                    .children(tiles)
                    .children(marks)
                    .into_any_element()
            })
            .collect();
        let this = cx.entity();
        let (down, moved, up, wheeled) = (this.clone(), this.clone(), this.clone(), this.clone());
        let pages = div()
            .id("pdf-pages")
            .size_full()
            .overflow_scroll()
            .track_scroll(&self.scroll)
            .bg(sunken)
            .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                down.update(cx, |view, cx| {
                    window.focus(&view.focus, cx);
                    match view.tool {
                        tool if tool.selects() => view.select_from(event.position, cx),
                        tool if tool.edits() => view.press(event.position, cx),
                        _ => view.sketch_from(event.position, cx),
                    }
                })
            })
            .on_mouse_move(move |event, _, cx| {
                if event.dragging() {
                    moved.update(cx, |view, cx| match view.tool {
                        tool if tool.selects() => view.select_to(event.position, cx),
                        tool if tool.edits() => view.drag_to(event.position, cx),
                        _ => view.sketch_to(event.position, cx),
                    })
                }
            })
            .on_mouse_up(MouseButton::Left, move |event, _, cx| {
                up.update(cx, |view, cx| match view.tool {
                    tool if tool.selects() => view.finish_select(event.position, cx),
                    tool if tool.edits() => view.release(cx),
                    _ => view.sketch_end(cx),
                })
            })
            .on_scroll(cx.listener(|_, _, _, cx| cx.notify()))
            .child(div().relative().w(px(laid.size.0)).h(px(laid.size.1)).children(pages));
        // Measures the view itself, which does not scroll.
        let measure = canvas(
            move |bounds, _, cx| {
                this.update(cx, |view, cx| {
                    if view.room != bounds {
                        view.room = bounds;
                        cx.notify();
                    }
                })
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let zoom = ely_ui::wheel::zoom_layer(move |event, _, cx| {
            wheeled.update(cx, |view, cx| view.wheel_zoom(event, cx))
        });
        let view = div().relative().size_full().child(pages).child(measure).child(zoom).into_any_element();
        (view, pages_shown)
    }

    /// Asks for the first missing tile, one at a time.
    fn paint(&mut self, wanted: Vec<(TileKey, [i32; 4])>, cx: &mut Context<Self>) {
        let Some((key, rect)) = wanted.first().copied() else {
            return;
        };
        if self.painting {
            return;
        }
        self.painting = true;
        let (doc, scale, version) = (self.doc, key.scale as f32 / 1000.0, self.version);
        self.off(
            move |worker| worker.render(doc, key.page, (scale, key.rotate), rect),
            move |view, painted, cx| {
                view.painting = false;
                if view.version != version {
                    log::info!("pdf: a tile from before an edit is dropped");
                    return cx.notify();
                }
                let painted = painted.map(|tile| {
                    if tile.damaged {
                        view.found_damage(cx);
                    }
                    tile.bgra
                });
                match painted.map(|bgra| picture((rect[2] as u32, rect[3] as u32), bgra)) {
                    Ok(Some(image)) => {
                        log::debug!("pdf: a tile of page {} came", key.page);
                        if let Some(old) = view.tiles.insert(key, image) {
                            view.retired.push(old);
                        }
                    }
                    Ok(None) => log::error!("pdf: a tile came back the wrong size"),
                    Err(error) if ely_pdf_engine::is_stale(&error) => log::info!("pdf: a tile from before an edit is dropped"),
                    Err(error) => {
                        // A failed tile is not asked again; once said.
                        let first = view.failed.is_empty();
                        view.failed.insert(key);
                        if first {
                            view.fail("drawing a page", &error, cx);
                        }
                    }
                }
                cx.notify();
            },
            cx,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_and_back_lands_on_the_same_point() {
        let size = (612.0, 792.0);
        for rotate in [0, 90, 180, 270, -90] {
            let point = (100.0, 50.0);
            let there = to_view(point, size, rotate);
            assert_eq!(from_view(there, size, rotate), point, "{rotate}");
        }
        assert_eq!(to_view((0.0, 0.0), size, 90), (792.0, 0.0), "the top left goes to the right");
        assert_eq!(rect_on([0.0, 0.0, 10.0, 20.0], size, 90, 2.0), (1544.0, 0.0, 40.0, 20.0));
    }

    #[test]
    fn a_view_edge_on_a_tile_edge_takes_no_tile_beyond() {
        assert_eq!(span(0.0, 512.0, 1.0, 2000), 0..=0);
        assert_eq!(span(256.0, 1024.0, 2.0, 4000), 1..=3);
        assert_eq!(span(0.0, 9000.0, 1.0, 2000), 0..=3, "the page ends first");
    }
}
