//! A page keeps its last whole draw until the next.

use std::sync::Arc;

use gpui::RenderImage;

use crate::layout::{Layout, Placed, layout, visible};
use crate::pages::{TILE, TileKey};
use crate::view::PdfView;

/// A drawn tile: key, image, and its page's size then.
pub type Drawn = (TileKey, Arc<RenderImage>, (f32, f32));

/// A page on screen: place, device size, tiles in view.
pub struct Showing {
    pub place: Placed,
    pub device: (i32, i32),
    pub keys: Vec<(TileKey, [i32; 4])>,
}

/// Where `key`'s tile sat in its page, as fractions.
fn fraction(key: &TileKey, size: (f32, f32)) -> [f32; 4] {
    let (w, h) = match key.rotate.rem_euclid(180) {
        90 => (size.1, size.0),
        _ => size,
    };
    let scale = key.scale as f32 / 1000.0;
    let (dw, dh) = ((w * scale).ceil(), (h * scale).ceil());
    let (x, y) = ((key.tx * TILE) as f32, (key.ty * TILE) as f32);
    [x / dw, y / dh, (TILE as f32).min(dw - x) / dw, (TILE as f32).min(dh - y) / dh]
}

impl PdfView {
    /// The pages in view and the tiles each needs.
    pub fn showing(&self, sf: f32) -> Vec<Showing> {
        let at = (-f32::from(self.scroll.offset().x), -f32::from(self.scroll.offset().y));
        self.showing_in(&self.laid_out(), sf, at)
    }

    /// The screen the next turn shows, to paint ahead.
    pub fn ahead(&self, sf: f32) -> Vec<Showing> {
        let last = self.sizes.len() as isize - 1;
        let page = (self.current as isize + self.heading).clamp(0, last.max(0)) as usize;
        let room = (f32::from(self.room.size.width), f32::from(self.room.size.height));
        let laid = layout(&self.sizes, self.rotate, (self.mode, self.zoom), room, page);
        match self.landing(&laid, page, 0.0).filter(|_| page != self.current) {
            Some(at) => self.showing_in(&laid, sf, at),
            None => Vec::new(),
        }
    }

    /// The pages `laid` shows from `(left, top)`, and their tiles.
    fn showing_in(&self, laid: &Layout, sf: f32, (left, top): (f32, f32)) -> Vec<Showing> {
        let (room_w, room_h) = (f32::from(self.room.size.width), f32::from(self.room.size.height));
        let scale = (laid.scale * sf * 1000.0).round() as u32;
        visible(laid, top, top + room_h)
            .filter(|p| p.x < left + room_w && p.x + p.w > left)
            .map(|place| {
                let (dw, dh) = ((place.w * sf).ceil() as i32, (place.h * sf).ceil() as i32);
                let ys = crate::pages::span(top - place.y, top + room_h - place.y, sf, dh);
                let xs = crate::pages::span(left - place.x, left + room_w - place.x, sf, dw);
                let keys = ys
                    .flat_map(|ty| xs.clone().map(move |tx| (tx, ty)))
                    .map(|(tx, ty)| {
                        let key = TileKey { page: place.page, scale, rotate: self.rotate, tx, ty };
                        (key, [tx * TILE, ty * TILE, TILE.min(dw - tx * TILE), TILE.min(dh - ty * TILE)])
                    })
                    .collect();
                Showing { place: *place, device: (dw, dh), keys }
            })
            .collect()
    }

    /// What a page draws, bottom first, in page fractions.
    pub fn pieces(&self, showing: &Showing) -> Vec<(Arc<RenderImage>, [f32; 4])> {
        let (dw, dh) = (showing.device.0 as f32, showing.device.1 as f32);
        let current: Option<Vec<_>> = showing
            .keys
            .iter()
            .map(|(key, rect)| {
                let at = [rect[0] as f32 / dw, rect[1] as f32 / dh, rect[2] as f32 / dw, rect[3] as f32 / dh];
                self.tiles.get(key).map(|image| (image.clone(), at))
            })
            .collect();
        if let Some(current) = current {
            return current;
        }
        // New tiles wait; the last whole draw stands in.
        let (page, rotate) = (showing.place.page, self.rotate);
        let size = self.sizes[page];
        let last = self.whole.get(&page).into_iter().flatten();
        let fits = |(key, _, at): &&Drawn| key.rotate == rotate && *at == size;
        last.filter(fits).map(|(key, image, at)| (image.clone(), fraction(key, *at))).collect()
    }

    /// Keeps each whole page's draw; drops tiles out of reach.
    pub fn settle(&mut self, showing: &[Showing], ahead: &[Showing]) {
        for s in showing.iter() {
            let drawn: Option<Vec<_>> = s.keys.iter().map(|(key, _)| self.tiles.get(key).map(|image| (*key, image.clone(), self.sizes[key.page]))).collect();
            if let Some(drawn) = drawn {
                self.whole.insert(s.place.page, drawn);
            }
        }
        self.whole.retain(|page, _| showing.iter().any(|s| s.place.page == *page));
        let listed = |screen: &[Showing], key: &TileKey| screen.iter().any(|s| s.keys.iter().any(|(k, _)| k == key));
        let seen = |key: &TileKey| listed(showing, key) || listed(ahead, key);
        let leaving: Vec<TileKey> = self.tiles.keys().filter(|key| !seen(key)).copied().collect();
        for key in leaving {
            let image = self.tiles.remove(&key).expect("a tile just listed");
            self.retired.push(image);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tile_keeps_its_place_as_a_fraction_of_its_page() {
        let key = TileKey { page: 0, scale: 2000, rotate: 0, tx: 1, ty: 0 };
        // A 612 by 792 page at 2x: 1224 by 1584.
        assert_eq!(fraction(&key, (612.0, 792.0)), [512.0 / 1224.0, 0.0, 512.0 / 1224.0, 512.0 / 1584.0]);
        let turned = TileKey { rotate: 90, ..key };
        assert_eq!(fraction(&turned, (612.0, 792.0)), [512.0 / 1584.0, 0.0, 512.0 / 1584.0, 512.0 / 1224.0]);
    }
}
