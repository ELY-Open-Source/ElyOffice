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

/// A visible area in tile twips and its tile scale.
pub(super) type Asked = ((i32, i32, i32, i32), (u32, i32));

/// The last whole screen.
pub(super) struct Whole {
    origin: Point<Pixels>,
    pages: Vec<Bounds<Pixels>>,
    tiles: Vec<Placed>,
    seen: Seen,
}

/// Document start, end and view start, across then down.
pub(super) type Pane = ((i32, i32, i32), (i32, i32, i32));

/// What the lane may still paint: one geom, these keys.
pub(super) type Wanted = Arc<Mutex<(Geom, HashSet<(i32, i32)>)>>;

/// The wanted set, for the view and the lane.
pub(super) fn wanted(wanted: &Wanted) -> MutexGuard<'_, (Geom, HashSet<(i32, i32)>)> {
    wanted.lock().expect("the wanted lock")
}

impl LokView {
    /// Drawn no more: queued tiles go; drawing asks again.
    pub fn hide(&mut self) {
        self.clock.hidden();
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
            self.whole = Some(Whole { origin, pages: pages.clone(), tiles: tiles.clone(), seen });
            return (pages, tiles);
        }
        let Some(whole) = self.whole.as_ref() else {
            return (pages, Vec::new());
        };
        let moved = origin - whole.origin;
        let shift = |b: &Bounds<Pixels>| Bounds { origin: b.origin + moved, size: b.size };
        let tiles = whole.tiles.iter().map(|(place, image, clip)| (shift(place), Arc::clone(image), shift(clip))).collect();
        (whole.pages.iter().map(shift).collect(), tiles)
    }

    /// On-screen keys still to come.
    fn waiting(&self) -> bool {
        self.lacks(&self.on_screen)
    }

    /// Whether `keys` still lack tiles; failed ones never come.
    fn lacks(&self, keys: &[(i32, i32)]) -> bool {
        // A move retries failed tiles, as `prepare` will.
        let moved = self.moved();
        keys.iter().any(|key| {
            (moved || !self.failed.contains(&(*key, self.generation)))
                && self.tiles.get(key).is_none_or(|tile| tile.stale)
        })
    }

    /// The visible area and scale asked of the kernel.
    pub(super) fn asked(&self) -> Asked {
        let (vw, vh) = (self.geom.twips_of(self.viewport.0), self.geom.twips_of(self.viewport.1));
        ((self.scroll.0, self.scroll.1, vw, vh), (self.geom.tile_pixels(), TILE_TWIPS))
    }

    /// Whether the asked area differs from the last one sent.
    pub(super) fn moved(&self) -> bool {
        self.shown != Some(self.asked())
    }

    /// The screen last drawn; input maps by it.
    pub(super) fn drawn(&self) -> Seen {
        self.whole.as_ref().map_or((self.scroll, self.geom, self.spare), |whole| whole.seen)
    }

    /// The screen the next frame draws; drawing maps by it.
    pub(super) fn seen(&self) -> Seen {
        let target = (self.scroll, self.geom, self.spare);
        match &self.whole {
            // Read before the next paint too, as headers are.
            Some(whole) if whole.seen != target && self.lacks(&self.screen_keys()) => whole.seen,
            _ => target,
        }
    }

    /// The tile keys the view's own screen needs.
    fn screen_keys(&self) -> Vec<(i32, i32)> {
        let (vw, vh) = (self.geom.twips_of(self.viewport.0), self.geom.twips_of(self.viewport.1));
        self.tiling((vw, vh), self.extent()).into_iter().flat_map(|(_, keys)| keys).collect()
    }

    /// Each visible pane and its tile keys.
    pub(super) fn tiling(&self, (vw, vh): (i32, i32), extent: (i32, i32)) -> Vec<(Pane, Vec<(i32, i32)>)> {
        let (ax, ay) = self.axes();
        let span = |(start, end, _): (i32, i32, i32)| start.div_euclid(TILE_TWIPS)..=end.div_euclid(TILE_TWIPS);
        let mut tiling = Vec::new();
        for across in ax.panes(vw) {
            for down in ay.panes(vh) {
                let keys = span(across)
                    .flat_map(|kx| span(down).map(move |ky| (kx, ky)))
                    .filter(|key| key.0 * TILE_TWIPS < extent.0 && key.1 * TILE_TWIPS < extent.1)
                    .collect();
                tiling.push(((across, down), keys));
            }
        }
        tiling
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

    /// Publishes the keys the lane may paint, then requests them.
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
        // Published first: the lane skips keys missing from it.
        let screen = self.on_screen.clone();
        for key in screen.into_iter().chain(ahead) {
            if self.tiles.get(&key).is_none_or(|tile| tile.stale) {
                self.request_tile(key);
            }
        }
    }
}
