//! One open PDF: its pages, tiles, text and picks (PDF-01).

use std::{
    collections::{BTreeSet, HashMap, HashSet, VecDeque},
    io,
    sync::{Arc, Mutex},
};

use ely_pdf_engine::{Bookmark, Char, Doc, Worker};
use gpui::{
    Context, EventEmitter, FocusHandle, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, RenderImage, ScrollHandle, Styled as _, Window, div,
};

use crate::engine::{Loaded, MEASURE, Mupdf};
use crate::layout::{Mode, Zoom};
use crate::pages::{TileKey, rect_on};
use crate::select::Selection;

/// A failure to show; the window toasts it.
pub struct Failed(pub String);

/// The document changed what it shows.
pub struct Reported;

/// A page or document read that failed once.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Unread {
    Notes(usize),
    Layer(usize),
    Names,
    Files,
    /// The signatures of the file at this path.
    Signatures(std::path::PathBuf),
}

pub struct PdfView {
    worker: Arc<Mutex<Mupdf>>,
    pub doc: Doc,
    /// Page sizes in points; unmeasured ones copy the first.
    pub sizes: Vec<(f32, f32)>,
    pub mode: Mode,
    pub zoom: Zoom,
    pub rotate: i32,
    /// The page at the top, or the one shown.
    pub current: usize,
    pub scroll: ScrollHandle,
    pub tiles: HashMap<TileKey, Arc<RenderImage>>,
    /// Images no longer drawn, freed at the next render.
    pub retired: ely_gpui_component::primitives::Retired,
    /// Tile pixels painted; the first paint is logged (PERF-05).
    pub painted: f64,
    /// Tiles the worker could not draw; not asked again.
    pub failed: HashSet<TileKey>,
    /// Reads that failed; not asked again until an edit.
    pub unread: HashSet<Unread>,
    /// Old tiles shown under their page until covered.
    pub backdrop: Vec<crate::backdrop::Old>,
    pub text: HashMap<usize, Arc<Vec<Char>>>,
    pub asking_text: HashSet<usize>,
    pub marks: Vec<Bookmark>,
    pub selection: Option<Selection>,
    /// Found words by page, each with its quads.
    pub found: Vec<(usize, Vec<[f32; 8]>)>,
    /// The found word chosen.
    pub chosen: Option<usize>,
    /// Where to scroll once laid out, (x, y) content pixels.
    pub go_to: Option<(f32, f32)>,
    /// A tile is on its way from the worker.
    pub painting: bool,
    /// The view's size and place, as last painted.
    pub room: gpui::Bounds<gpui::Pixels>,
    /// Trackpad pixels toward the next zoom step.
    pub wheel: f32,
    pub focus: FocusHandle,
    /// Counts edits; replies from before one are dropped.
    pub version: u64,
    /// Edits landed so far.
    pub edited: u64,
    /// Edits waiting their turn, and whether one runs.
    pub queue: VecDeque<crate::edits::Queued>,
    pub editing: bool,
    /// The running edit may move pages.
    pub moving: bool,
    /// History marks: now, and at the last load or save.
    pub mark: String,
    pub saved: String,
    /// Saves so far: a read from before one is stale.
    pub saves: u64,
    /// Page pictures for the side panel, by page.
    pub thumbs: HashMap<usize, Arc<RenderImage>>,
    pub thumbs_failed: HashSet<usize>,
    /// Pictures from before an edit; shown until repainted.
    pub thumbs_stale: HashSet<usize>,
    /// A picture is on its way from the worker.
    pub thumbing: bool,
    /// Pages picked in the side panel, and the anchor.
    pub picked: BTreeSet<usize>,
    pub anchor: Option<usize>,
    /// The drop zone under a drag: page, lower half.
    pub aimed: Option<(usize, bool)>,
    pub tool: crate::content::Tool,
    /// What pages hold, read while editing.
    pub layers: crate::content::Layers,
    pub reading_layer: bool,
    pub aim: Option<crate::content::Aim>,
    pub drag: Option<crate::content::Drag>,
    /// The gesture's page, its window origin and scale.
    pub grip: Option<(usize, gpui::Point<gpui::Pixels>, f32)>,
    /// The aim when a gesture began; the panel keeps it.
    pub held: Option<crate::content::Aim>,
    /// Annotations and fields by page, for every tool.
    pub notes: crate::annotate::Notes,
    pub reading_notes: bool,
    /// Links by page, read with the notes.
    pub links: HashMap<usize, Vec<ely_pdf_engine::Link>>,
    /// Every field name in the form, read for new fields.
    pub form_names: Option<Vec<String>>,
    pub reading_names: bool,
    /// The bookmark picked in the side panel, depth first.
    pub mark_pick: Option<usize>,
    /// Attachments, read when listed, and the one picked.
    pub files: Option<Vec<ely_pdf_engine::Attachment>>,
    pub reading_files: bool,
    pub file_pick: Option<String>,
    /// Regions marked to remove; never in the document.
    pub redactions: Vec<ely_pdf_engine::Region>,
    pub sketch: Option<crate::annotate::Sketch>,
    /// A pick to mark once its text is read.
    pub marking: Option<crate::select::Selection>,
    /// Where the last text pick was pressed, in the window.
    pub pressed: Option<gpui::Point<gpui::Pixels>>,
    /// What the document allows, `proto::may` bits.
    pub allowed: u32,
    /// The file read, and its checked signature fields.
    pub signatures: Option<(std::path::PathBuf, Vec<crate::sign::Checked>)>,
    pub reading_signatures: bool,
    pub sign_at: Option<crate::sign::SignAt>,
    /// A visible signature waiting for its place.
    pub stamp: Option<crate::visible::Stamp>,
    /// MuPDF's words, when it repaired the file.
    pub repair: Option<String>,
    /// A draw met damage; the tab hears once.
    pub damage_seen: bool,
}

/// Why a request was refused, by what it needs.
pub fn restricted(needed: &str, cx: &gpui::App) -> gpui::SharedString {
    let what = ely_ui::i18n::text(cx, &format!("pdf.may.{needed}"), &[]);
    ely_ui::i18n::text(cx, "pdf.restricted", &[("what", &what)])
}

/// Page sizes in points; unmeasured pages copy the first.
pub fn sizes(pages: usize, first: &[[f32; 4]]) -> Vec<(f32, f32)> {
    let size = |b: &[f32; 4]| (b[2] - b[0], b[3] - b[1]);
    let mut sizes = vec![size(&first[0]); pages];
    for (at, bounds) in first.iter().enumerate() {
        sizes[at] = size(bounds);
    }
    sizes
}

impl EventEmitter<Failed> for PdfView {}
impl EventEmitter<Reported> for PdfView {}

impl PdfView {
    /// All its images, for a closing tab to free.
    pub fn retire_all(&mut self) -> ely_gpui_component::primitives::Retired {
        let mut images = std::mem::take(&mut self.retired);
        let tiles = self.tiles.drain().map(|(_, image)| image);
        let old = self.backdrop.drain(..).map(|old| old.image);
        for image in tiles.chain(old).chain(self.thumbs.drain().map(|(_, image)| image)) {
            images.push(image);
        }
        images
    }

    pub fn new(worker: Arc<Mutex<Mupdf>>, loaded: Loaded, cx: &mut Context<Self>) -> Self {
        let sizes = sizes(loaded.pages, &loaded.first);
        log::info!("pdf: a view of {} pages", loaded.pages);
        let measured = loaded.first.len();
        let mut view = PdfView {
            worker,
            doc: loaded.doc,
            sizes,
            mode: Mode::Continuous,
            zoom: Zoom::Width,
            rotate: 0,
            current: 0,
            scroll: ScrollHandle::new(),
            tiles: HashMap::new(),
            retired: Default::default(),
            painted: 0.0,
            failed: HashSet::new(),
            unread: HashSet::new(),
            backdrop: Vec::new(),
            text: HashMap::new(),
            asking_text: HashSet::new(),
            marks: loaded.marks,
            selection: None,
            found: Vec::new(),
            chosen: None,
            go_to: None,
            painting: false,
            room: gpui::Bounds::default(),
            wheel: 0.0,
            focus: cx.focus_handle(),
            version: 0,
            edited: 0,
            queue: VecDeque::new(),
            editing: false,
            moving: false,
            saved: loaded.mark.clone(),
            saves: 0,
            mark: loaded.mark,
            thumbs: HashMap::new(),
            thumbs_failed: HashSet::new(),
            thumbs_stale: HashSet::new(),
            thumbing: false,
            picked: BTreeSet::new(),
            anchor: None,
            aimed: None,
            tool: crate::content::Tool::Read,
            layers: HashMap::new(),
            reading_layer: false,
            aim: None,
            drag: None,
            held: None,
            grip: None,
            notes: HashMap::new(),
            reading_notes: false,
            links: HashMap::new(),
            form_names: None,
            reading_names: false,
            mark_pick: None,
            files: None,
            reading_files: false,
            file_pick: None,
            redactions: Vec::new(),
            sketch: None,
            marking: None,
            pressed: None,
            allowed: loaded.allowed,
            signatures: None,
            reading_signatures: false,
            sign_at: None,
            stamp: None,
            repair: loaded.repaired.clone(),
            damage_seen: false,
        };
        view.measure(measured, cx);
        view
    }

    /// Says `what` failed; the window shows it.
    pub fn fail(&self, what: &str, error: &io::Error, cx: &mut Context<Self>) {
        // Engine text may quote the document: the user sees it.
        log::error!("pdf: {what} failed ({:?})", error.kind());
        let text = match ely_pdf_engine::refused(error) {
            _ if ely_pdf_engine::is_stale(error) => ely_ui::i18n::text(cx, "pdf.object.changed", &[]),
            Some(needed) => restricted(needed, cx),
            None => ely_ui::i18n::text(cx, "pdf.failed", &[("what", what), ("error", &error.to_string())]),
        };
        cx.emit(Failed(text.to_string()));
    }

    /// Runs `work` off the window; edits between drop `then`.
    pub fn read<T: Send + 'static>(
        &self,
        work: impl FnOnce(&mut Worker) -> io::Result<T> + Send + 'static,
        then: impl FnOnce(&mut Self, io::Result<T>, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        let version = self.version;
        self.off(
            work,
            move |view, done, cx| match view.version == version {
                true => then(view, done, cx),
                false => log::info!("pdf: a read from before an edit is dropped"),
            },
            cx,
        );
    }

    /// Runs `work` off the window, then `then` here.
    pub fn off<T: Send + 'static>(
        &self,
        work: impl FnOnce(&mut Worker) -> io::Result<T> + Send + 'static,
        then: impl FnOnce(&mut Self, io::Result<T>, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        let worker = self.worker.clone();
        cx.spawn(async move |this, cx| {
            let done = cx
                .background_executor()
                .spawn(async move { work(worker.lock().expect("the worker lock").get()?) })
                .await;
            if this.update(cx, |view, cx| then(view, done, cx)).is_err() {
                log::info!("pdf: a reply outlived its view");
            }
        })
        .detach();
    }

    /// Measures the other pages, a batch at a time.
    pub fn measure(&mut self, from: usize, cx: &mut Context<Self>) {
        let (doc, count) = (self.doc, (self.sizes.len() - from).min(MEASURE));
        if count == 0 {
            return;
        }
        self.read(
            move |worker| worker.bounds(doc, from, count),
            move |view, read, cx| match read {
                Ok(boxes) => {
                    for (at, b) in boxes.iter().enumerate() {
                        view.sizes[from + at] = (b[2] - b[0], b[3] - b[1]);
                    }
                    cx.emit(Reported);
                    cx.notify();
                    view.measure(from + count, cx);
                }
                Err(error) => view.fail("page sizes", &error, cx),
            },
            cx,
        );
    }

    /// Scrolls `page` into view, `y` points near the top.
    pub fn show(&mut self, page: usize, y: f32, cx: &mut Context<Self>) {
        self.current = page;
        let laid = self.laid_out();
        if let Some(place) = laid.placed.iter().find(|place| place.page == page) {
            let (_, top, _, _) = rect_on([0.0, y, 0.0, y], self.sizes[page], self.rotate, laid.scale);
            let x = match self.mode {
                Mode::Horizontal => (place.x - crate::layout::GAP).max(0.0),
                _ => self.across(place),
            };
            self.go_to = Some((x, (place.y + top - crate::layout::GAP * 2.0).max(0.0)));
        }
        cx.notify();
    }

    /// Across: stay unless the page's left is out of view.
    fn across(&self, place: &crate::layout::Placed) -> f32 {
        let (left, wide) = (-f32::from(self.scroll.offset().x), f32::from(self.room.size.width));
        match place.x >= left && place.x < left + wide {
            true => left,
            false => (place.x - crate::layout::GAP).max(0.0),
        }
    }

    /// Every page tile goes; images are freed next render.
    pub fn retire_tiles(&mut self) {
        for old in self.backdrop.drain(..) {
            self.retired.push(old.image);
        }
        for (_, image) in self.tiles.drain() {
            self.retired.push(image);
        }
    }

    /// Changes how pages show; the spot read stays put.
    pub fn change(&mut self, change: impl FnOnce(&mut Self, f32), cx: &mut Context<Self>) {
        // A scroll not yet drawn counts.
        let offset = self.scroll.offset();
        let (left, top) = self.go_to.unwrap_or((-f32::from(offset.x), -f32::from(offset.y)));
        let laid = self.laid_out();
        // The page nearest the top left: across first, then down.
        let near = |place: &crate::layout::Placed| {
            let across = (place.x - left).max(left - place.x - place.w).max(0.0);
            (across, (place.y - top).max(0.0))
        };
        let anchor = (laid.placed.iter().filter(|place| place.y + place.h > top))
            .min_by(|a, b| near(a).partial_cmp(&near(b)).expect("finite places"));
        // Down the page as a share; above it in pixels.
        let (anchor, spot) = anchor.map_or((self.current, (0.0, 0.0, 0.0)), |place| {
            let down = ((top - place.y) / place.h).max(0.0);
            (place.page, ((left - place.x) / place.w, down, (top - place.y).min(0.0)))
        });
        change(self, laid.scale);
        self.retire_tiles();
        let laid = self.laid_out();
        // Else the page read, at its top.
        let held = match laid.placed.iter().find(|place| place.page == anchor) {
            Some(place) => Some((place, spot)),
            None => laid.placed.iter().find(|place| place.page == self.current).map(|place| (place, (spot.0, 0.0, 0.0))),
        };
        if let Some((place, (x, down, above))) = held {
            self.go_to = Some(((place.x + x * place.w).max(0.0), (place.y + down * place.h + above).max(0.0)));
            log::info!("pdf: view changed; page {} held at {x:.2}, {down:.2}, {above:.0} px at scale {:.2}", place.page + 1, laid.scale);
        }
        cx.notify();
    }

    /// Ctrl (Cmd on macOS) and the wheel step the zoom.
    pub fn wheel_zoom(&mut self, event: &gpui::ScrollWheelEvent, cx: &mut Context<Self>) {
        let steps = ely_ui::wheel::steps(event, &mut self.wheel);
        for _ in 0..steps.unsigned_abs() {
            self.change(|view, now| view.zoom = Zoom::Scale(crate::keys::stepped(now, steps > 0)), cx);
        }
    }

    /// Moves `by` pages; continuous views scroll there.
    pub fn turn(&mut self, by: isize, cx: &mut Context<Self>) {
        let last = self.sizes.len().saturating_sub(1) as isize;
        let page = (self.current as isize + by).clamp(0, last) as usize;
        self.show(page, 0.0, cx);
    }
}

impl Render for PdfView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.retired.release(window);
        let pages = self.page_view(window, cx);
        div().size_full().track_focus(&self.focus).child(pages)
    }
}
