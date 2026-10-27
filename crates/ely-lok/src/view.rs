//! The GPUI tile canvas for one document (LOK-03, LOK-04, LOK-06).

use std::{
    collections::{HashMap, HashSet},
    io,
    sync::{Arc, Mutex},
};

use gpui::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Pixels, Point, RenderImage,
    Task, px,
};
use image::{Frame, ImageBuffer, Rgba};

use crate::{Doc, Events, Rect, Worker};

mod borders;
mod clip;
mod clock;
mod edge;
mod formula;
mod geom;
pub use geom::Geom;
mod handles;
mod ime;
mod input;
mod lane;
mod off;
mod panes;
mod reads;
mod render;
mod zoom;
pub use zoom::{ZOOM_STEPS, Zoomed};
mod scroll;
mod sheet;
mod text;
pub use input::{CONTEXT, Copy, Cut, Paste, init};
pub use off::read_off;
pub use render::{SCHEME, paper};
mod turn;
pub use turn::notch;

pub(crate) use crate::{TILE_TWIPS, TWIPS_PER_PIXEL};
/// An input request the engine refused; apps show it.
#[derive(Clone, Debug)]
pub struct LokError(pub String);

/// The kernel reported, or queued input landed.
#[derive(Clone, Debug)]
pub struct Reported;

struct TileEntry {
    image: Arc<RenderImage>,
    /// Shown until its repaint lands, so edits never flash.
    stale: bool,
}

/// Paints one LOK document as a cached tile grid (LOK-03).
pub struct LokView {
    worker: Arc<Mutex<Worker>>,
    doc: Doc,
    geom: Geom,
    scroll: (i32, i32),
    doc_size: (i32, i32),
    generation: u64,
    tiles: HashMap<(i32, i32), TileEntry>,
    /// Tiles no longer drawn, freed at the next paint.
    retired: ely_gpui_component::primitives::Retired,
    fetching: HashSet<(i32, i32)>,
    failed: HashSet<((i32, i32), u64)>,
    /// The area last told to the kernel.
    shown: Option<(Rect, (u32, i32))>,
    viewport: (f32, f32),
    origin: Point<Pixels>,
    /// Pixels a small document sits in: left, top.
    spare: (f32, f32),
    /// Frozen columns and rows, in twips from the top left.
    frozen: (i32, i32),
    /// A spreadsheet's rows and columns, in print twips.
    sheet: Option<crate::sheet::Geometry>,
    /// The farthest point a jump needed; Calc grows to it.
    reach: (i32, i32),
    pages: Vec<Rect>,
    focus: FocusHandle,
    lane: lane::Lane,
    sent: u64,
    applied: u64,
    edited: u64,
    moves: u64,
    last_edit: u64,
    clock: clock::EditClock,
    /// A tile reached the screen; logged once (PERF-03).
    drawn: bool,
    /// Tiles the last frame showed.
    on_screen: Vec<(i32, i32)>,
    /// The mark of this view's last copy.
    copied: Option<String>,
    /// The event watch; it ends with the view.
    watching: Option<Task<()>>,
    cursor: Option<Rect>,
    text: text::TextGeometry,
    cursor_visible: bool,
    pointer: gpui::CursorStyle,
    selection: Vec<Rect>,
    cell: crate::Cell,
    completion: formula::Completion,
    /// Whether the kernel was told Ely's scheme.
    lit: bool,
    /// The zoom follows the viewport so a page fits.
    fit: bool,
    /// Set when the wheel and Up, Down turn parts.
    turning: Option<turn::Turning>,
    /// The sheet shown, and the offered function chosen.
    part: usize,
    picked: usize,
    marked: Option<ime::Marked>,
    dragging: bool,
    edge: edge::Edge,
    grip: Option<handles::Grip>,
    borders: borders::Borders,
    handle_size: Pixels,
    states: HashMap<String, String>,
    graphic: Option<Rect>,
    review: u64,
    /// The last search outcome: its count and matches.
    search: (u64, Vec<Rect>),
    menu: Option<String>,
    right_at: Option<Point<Pixels>>,
    fonts: Vec<String>,
    a11y: crate::A11y,
    /// What readers call the view.
    name: gpui::SharedString,
    tabs: Vec<i32>,
}

/// A document's size and page frames, in twips.
pub type Layout = ((i64, i64), Vec<Rect>);

use sheet::{Measured, measure};

/// Reads `doc`'s size and page frames.
pub fn layout(worker: &mut Worker, doc: &Doc) -> io::Result<Layout> {
    Ok((doc.size(worker)?, doc.pages(worker)?))
}

impl EventEmitter<LokError> for LokView {}

impl EventEmitter<Reported> for LokView {}

impl LokView {
    /// A view of `doc` at `layout`; starts the event watch.
    pub fn open(
        worker: Arc<Mutex<Worker>>,
        doc: Doc,
        (size, pages): Layout,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| Self::sized(worker, doc, size, pages, cx))
    }

    fn sized(
        worker: Arc<Mutex<Worker>>,
        doc: Doc,
        (width, height): (i64, i64),
        pages: Vec<Rect>,
        cx: &mut Context<Self>,
    ) -> Self {
        log::info!("lok: {} pages", pages.len());
        let lane = Self::lane(Arc::clone(&worker), doc.clone(), cx);
        let mut view = Self {
            worker,
            doc,
            geom: Geom { zoom: 1.0, dpi: 1.0 },
            scroll: (0, 0),
            doc_size: (width as i32, height as i32),
            generation: 0,
            tiles: HashMap::new(),
            retired: Default::default(),
            fetching: HashSet::new(),
            failed: HashSet::new(),
            shown: None,
            viewport: (0.0, 0.0),
            origin: Point::default(),
            spare: (0.0, 0.0),
            frozen: (0, 0),
            sheet: None,
            reach: (0, 0),
            pages,
            focus: cx.focus_handle().tab_stop(true),
            lane,
            sent: 0,
            applied: 0,
            edited: 0,
            moves: 0,
            last_edit: 0,
            clock: clock::EditClock::default(),
            drawn: false,
            on_screen: Vec::new(),
            copied: None,
            cursor: None,
            text: text::TextGeometry::default(),
            cursor_visible: true,
            pointer: gpui::CursorStyle::Arrow,
            selection: Vec::new(),
            cell: crate::Cell::default(),
            completion: formula::Completion::default(),
            lit: false,
            fit: false,
            turning: None,
            part: 0,
            picked: 0,
            marked: None,
            dragging: false,
            edge: Default::default(),
            grip: None,
            borders: Default::default(),
            handle_size: px(6.0),
            states: HashMap::new(),
            graphic: None,
            review: 0,
            search: (0, Vec::new()),
            menu: None,
            right_at: None,
            fonts: Vec::new(),
            a11y: crate::A11y::default(),
            name: gpui::SharedString::default(),
            tabs: Vec::new(),
            watching: None,
        };
        view.watching = Some(view.watch(cx));
        view
    }

    /// Repaints every tile; the kernel sent no notice.
    pub fn invalidate(&mut self, cx: &mut Context<Self>) {
        for tile in self.tiles.values_mut() {
            tile.stale = true;
        }
        self.failed.clear();
        cx.notify();
    }

    /// Zooms so one page fits the viewport, from now on.
    pub fn fit_page(&mut self, cx: &mut Context<Self>) {
        if !self.fit {
            self.fit = true;
            cx.emit(Zoomed);
            cx.notify();
        }
    }

    /// The tiles painted for the current state (test evidence).
    #[cfg(test)]
    pub fn tile_count(&self) -> usize {
        self.tiles.values().filter(|tile| !tile.stale).count()
    }

    /// Runs a table command; arguments are checked now (LOK-02).
    pub fn run(&mut self, id: &str, values: &[&str]) -> io::Result<()> {
        let command = crate::commands::find(id)?;
        if command.addressed(values) {
            return Err(io::Error::other(format!("{id} names something by number; run it at the state it was read")));
        }
        self.send(input::Input::Run { commands: vec![command.owned(values)?], at: None });
        Ok(())
    }

    /// The document as the view last saw it.
    pub fn read_at(&self) -> Doc {
        self.doc.clone()
    }

    /// Batches and landings arrive out of order; keep the latest.
    fn seen_revision(&mut self, revision: u64) {
        self.doc.rev = self.doc.rev.max(revision);
    }

    /// Runs `id` against `at`, the document as it was read.
    pub fn run_at(&mut self, at: &crate::Doc, id: &str, values: &[&str]) -> io::Result<()> {
        self.run_all_at(at, &[(id, values)])
    }

    /// Runs `commands` in order, all made from the read `at`.
    pub fn run_all_at(&mut self, at: &crate::Doc, commands: &[(&str, &[&str])]) -> io::Result<()> {
        let commands = commands.iter().map(|(id, values)| crate::commands::find(id)?.owned(values)).collect::<io::Result<_>>()?;
        self.send(input::Input::Run { commands, at: Some(at.clone()) });
        Ok(())
    }

    /// Takes pushed events; the worker lock stays free (LOK-03).
    fn watch(&self, cx: &mut Context<Self>) -> Task<()> {
        let feed = self.worker.lock().expect("the worker lock").feed();
        let (worker, doc) = (Arc::clone(&self.worker), self.doc.clone());
        cx.spawn(async move |this, cx| {
            // The resize count measured; none yet.
            let (mut known, mut resized) = (0u64, None);
            loop {
                if this.upgrade().is_none() {
                    break;
                }
                // Awaited, not blocked: no thread waits for events.
                let batch = match feed.next(&doc, known).await {
                    Ok(batch) => {
                        let measured = if resized != Some(batch.resized) {
                            let (worker, doc) = (Arc::clone(&worker), doc.clone());
                            let read = async move { measure(&worker, &doc) };
                            Some(cx.background_executor().spawn(read).await)
                        } else {
                            None
                        };
                        Ok((batch, measured))
                    }
                    Err(error) => Err(error),
                };
                let batch = match batch {
                    Ok(batch) => batch,
                    Err(error) => {
                        log::error!("lok: event watch ends: {error}");
                        let message = format!("the document engine stopped: {error}");
                        if this.update(cx, |_, cx| cx.emit(LokError(message))).is_err() {
                            log::info!("lok: the view went first");
                        }
                        break;
                    }
                };
                let (batch, measured) = batch;
                (known, resized) = (batch.seq, Some(batch.resized));
                if this
                    .update(cx, |view, cx| view.apply_batch(batch, measured, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
    }

    /// Takes the cursor; drops the tiles a batch dirties (LOK-03).
    fn apply_batch(&mut self, mut batch: Events, measured: Option<Measured>, cx: &mut Context<Self>) {
        let moved = self.read_text(&mut batch);
        // What the view shows is the document at this revision.
        self.seen_revision(batch.revision);
        match measured {
            Some(Ok((((width, height), pages), zoom))) => {
                self.doc_size = self.at_scale((width as i32, height as i32), zoom);
                if pages.len() != self.pages.len() {
                    log::info!("lok: {} pages", pages.len());
                }
                self.pages = pages;
                self.clamp_scroll();
            }
            Some(Err(error)) => log::warn!("lok: document size lost: {error}"),
            None => {}
        }
        if moved {
            self.moves += 1;
            // A moved cursor stays in view, but not mid-drag.
            if self.viewport.1 > 0.0
                && !self.dragging
                && let Some(cursor) = batch.cursor
            {
                self.show(cursor, cx);
            }
        }
        // A moved cell cursor stays in view too.
        if batch.cell.cursor != self.cell.cursor
            && self.viewport.1 > 0.0
            && !self.dragging
            && let Some(cell) = batch.cell.cursor
        {
            self.show(cell, cx);
        }
        self.follow_graphic(batch.graphic, cx);
        self.cursor = batch.cursor;
        self.cursor_visible = batch.cursor_visible;
        if !batch.pointer.is_empty() {
            self.pointer = render::pointer(&batch.pointer);
        }
        self.selection = batch.selection;
        if batch.cell.functions != self.cell.functions {
            self.picked = 0;
        }
        self.cell = batch.cell;
        self.completion.reported(batch.revision);
        self.graphic = batch.graphic;
        self.read_borders(batch.table, batch.revision);
        self.review = batch.review;
        self.search = (batch.searched, batch.found);
        self.fonts = batch.fonts;
        if batch.menu.is_some() {
            self.menu = batch.menu;
        }
        for fault in batch.faults {
            cx.emit(LokError(fault));
        }
        self.a11y = batch.a11y;
        self.tabs = batch.tabs;
        self.states.extend(batch.states);
        for title in batch.blocked {
            cx.emit(LokError(format!(
                "the kernel opened a dialog Ely closed: {title}"
            )));
        }
        cx.emit(Reported);
        if batch.generation == self.generation && !batch.all && batch.rects.is_empty() {
            cx.notify();
            return;
        }
        self.generation = batch.generation;
        self.clock.bumped(batch.revision, batch.generation);
        let rects = &batch.rects;
        for (&(kx, ky), tile) in &mut self.tiles {
            let (tx, ty) = (kx * TILE_TWIPS, ky * TILE_TWIPS);
            tile.stale |= batch.all
                || rects.iter().any(|&(x, y, w, h)| {
                    tx < x + w && x < tx + TILE_TWIPS && ty < y + h && y < ty + TILE_TWIPS
                });
        }
        // Shown tiles repaint now, not a frame later.
        let dirty: Vec<_> = self
            .on_screen
            .iter()
            .copied()
            .filter(|key| self.tiles.get(key).is_some_and(|tile| tile.stale))
            .collect();
        for key in dirty {
            self.request_tile(key);
        }
        cx.notify();
    }

    /// Requests one missing tile on the background executor.
    fn request_tile(&mut self, key: (i32, i32)) {
        if self.failed.contains(&(key, self.generation)) || !self.fetching.insert(key) {
            return;
        }
        let job = lane::Job {
            key,
            geom: self.geom,
        };
        self.lane
            .tiles
            .unbounded_send(job)
            .expect("the lane lives with the view");
    }

    /// Takes a painted tile; stale generations never land (LOK-03).
    fn finish_tile(
        &mut self,
        key: (i32, i32),
        geom: Geom,
        tile: io::Result<crate::Tile>,
        cx: &mut Context<Self>,
    ) {
        self.fetching.remove(&key);
        match tile {
            Ok(tile) if tile.generation >= self.generation && geom == self.geom => {
                let side = geom.tile_pixels();
                // LOK and RenderImage both take BGRA bytes.
                let Some(image) = ImageBuffer::<Rgba<u8>, _>::from_raw(side, side, tile.pixels)
                else {
                    log::warn!("lok: a tile came back the wrong size");
                    return;
                };
                let image = Arc::new(RenderImage::new([Frame::new(image)]));
                self.clock.painted(tile.generation, Arc::as_ptr(&image) as usize);
                if let Some(old) = self.tiles.insert(key, TileEntry { image, stale: false }) {
                    self.retired.push(old.image);
                }
                cx.notify();
            }
            Ok(_) => cx.notify(),
            Err(error) => {
                // Retried at the next change or move.
                log::warn!("lok: tile {key:?} failed: {error}");
                self.failed.insert((key, self.generation));
                cx.emit(LokError(format!("a page failed to paint: {error}")));
            }
        }
    }
}

#[cfg(test)]
mod tests;
