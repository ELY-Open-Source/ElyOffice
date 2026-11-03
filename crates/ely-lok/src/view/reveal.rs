//! A screen shows whole: tiles wait for the rest.

use std::{
    collections::HashSet,
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};

use gpui::{Bounds, Pixels, Point, RenderImage};

use super::{Geom, LokView, TILE_TWIPS};

/// One painted tile; stale ones wait for a repaint.
pub(super) struct TileEntry {
    pub image: Arc<RenderImage>,
    pub stale: bool,
}

/// A tile's place, image and clipping pane.
pub(super) type Placed = (Bounds<Pixels>, Arc<RenderImage>, Bounds<Pixels>);

/// Scroll, geom and margins a screen was drawn at.
pub(super) type Seen = ((i32, i32), Geom, (f32, f32));

/// The last whole screen, and whether it stands in now.
pub(super) struct Whole {
    origin: Point<Pixels>,
    pages: Vec<Bounds<Pixels>>,
    tiles: Vec<Placed>,
    seen: Seen,
    holding: bool,
}

/// What the lane may still paint: one geom, these keys.
pub(super) type Wanted = Arc<Mutex<(Geom, HashSet<(i32, i32)>)>>;

/// The wanted set, for the view and the lane.
pub(super) fn wanted(wanted: &Wanted) -> MutexGuard<'_, (Geom, HashSet<(i32, i32)>)> {
    wanted.lock().expect("the wanted lock")
}

impl LokView {
    /// Drawn no more: queued tiles go; drawing asks again.
    pub fn hide(&mut self) {
        self.on_screen.clear();
        self.filling = None;
        wanted(&self.wanted).1.clear();
    }

    /// Takes `image` for `key`; it shows with its screen.
    pub(super) fn land(&mut self, key: (i32, i32), image: Arc<RenderImage>) {
        if let Some(old) = self.tiles.insert(key, TileEntry { image, stale: false }) {
            self.retired.push(old.image);
        }
        self.reveal_if_whole();
    }

    /// This frame's pages and tiles, or the last whole screen's.
    pub(super) fn screen(&mut self, origin: Point<Pixels>, pages: Vec<Bounds<Pixels>>, tiles: Vec<Placed>) -> (Vec<Bounds<Pixels>>, Vec<Placed>) {
        if !self.waiting() {
            let seen = (self.scroll, self.geom, self.spare);
            self.whole = Some(Whole { origin, pages: pages.clone(), tiles: tiles.clone(), seen, holding: false });
            return (pages, tiles);
        }
        let Some(whole) = self.whole.as_mut() else {
            return (pages, Vec::new());
        };
        whole.holding = true;
        let moved = origin - whole.origin;
        let shift = |b: &Bounds<Pixels>| Bounds { origin: b.origin + moved, size: b.size };
        let tiles = whole.tiles.iter().map(|(place, image, clip)| (shift(place), Arc::clone(image), shift(clip))).collect();
        (whole.pages.iter().map(shift).collect(), tiles)
    }

    /// On-screen keys still to come; failed ones never do.
    fn waiting(&self) -> bool {
        self.on_screen.iter().any(|key| {
            !self.failed.contains(&(*key, self.generation)) && self.tiles.get(key).is_none_or(|tile| tile.stale)
        })
    }

    /// A frame being prepared maps by the target geometry.
    pub(super) fn aim(&mut self) {
        if let Some(whole) = self.whole.as_mut() {
            whole.holding = false;
        }
    }

    /// The geometry on screen: the held one, else the view's.
    pub(super) fn seen(&self) -> Seen {
        match &self.whole {
            Some(whole) if whole.holding => whole.seen,
            _ => (self.scroll, self.geom, self.spare),
        }
    }

    /// Logs a screen that lacked tiles once it lacks none.
    pub(super) fn reveal_if_whole(&mut self) {
        if self.waiting() {
            self.filling.get_or_insert_with(Instant::now);
            return;
        }
        if let Some(since) = self.filling.take() {
            log::debug!("lok: screen complete in {} ms, {} tiles", since.elapsed().as_millis(), self.on_screen.len());
        }
    }

    /// The next screen's keys, the way the view last moved.
    fn ahead(&self, reach: i32, extent: (i32, i32)) -> Vec<(i32, i32)> {
        let rows = || self.on_screen.iter().map(|key| key.1);
        let (Some(top), Some(bottom)) = (rows().min(), rows().max()) else {
            return Vec::new();
        };
        let ahead = if self.heading < 0 { top - reach..top } else { bottom + 1..bottom + 1 + reach };
        let mut columns: Vec<i32> = self.on_screen.iter().map(|key| key.0).collect();
        columns.sort_unstable();
        columns.dedup();
        ahead
            .filter(|ky| *ky >= 0 && ky * TILE_TWIPS < extent.1)
            .flat_map(|ky| columns.iter().map(move |&kx| (kx, ky)))
            .collect()
    }

    /// Lets the lane drop jobs nobody waits for; paints ahead.
    pub(super) fn want(&mut self, reach: i32, extent: (i32, i32)) {
        // The screen first; ahead only once it is whole.
        let ahead = if self.waiting() { Vec::new() } else { self.ahead(reach, extent) };
        let keys: HashSet<(i32, i32)> = self.on_screen.iter().chain(&ahead).copied().collect();
        {
            let mut wanted = wanted(&self.wanted);
            if wanted.0 != self.geom || wanted.1 != keys {
                *wanted = (self.geom, keys);
            }
        }
        for key in ahead {
            if self.tiles.get(&key).is_none_or(|tile| tile.stale) {
                self.request_tile(key);
            }
        }
    }
}
