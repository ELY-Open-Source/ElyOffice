//! Document events: tiles, cursor, state, dialogs, a11y (LOK-03..08).

use std::{
    collections::{HashMap, HashSet},
    ffi::{CStr, c_char, c_void},
    sync::{Condvar, Mutex, OnceLock},
    time::{Duration, Instant},
};

use crate::ffi::{Document, Raw};

mod cell;
mod parse;
mod text;
#[cfg(test)]
mod tests;
pub use text::TextAnchors;
pub use cell::{Cell, Function, Mark, Request, offered};
use cell::{functions_of, marks_of, request_of, usage_of};
pub use parse::A11y;
use parse::{
    Dialog, Unreadable, a11y_update, dialog_of, fonts_of, formula_text, found_of, rect_of,
    result_of, state_of, tabs_of,
};

/// LOK_CALLBACK kinds Ely reads.
const INVALIDATE_TILES: i32 = 0;
const CURSOR: i32 = 1;
const SELECTION: i32 = 2;
const CURSOR_VISIBLE: i32 = 5;
const GRAPHIC: i32 = 6;
const STATE: i32 = 8;
const RULER: i32 = 35;
const WINDOW: i32 = 36;
const JSDIALOG: i32 = 46;
const RESULT: i32 = 16;
const A11Y_FOCUS: i32 = 62;
const A11Y_CARET: i32 = 63;
const A11Y_SELECTION: i32 = 64;
const A11Y_CELL: i32 = 67;
const A11Y_EDITING: i32 = 68;
const A11Y_OBJECT: i32 = 69;
const REDLINES: i32 = 30;
const REDLINE: i32 = 31;
const COMMENT: i32 = 32;
const NOT_FOUND: i32 = 12;
const FOUND: i32 = 15;
const MENU: i32 = 23;
const FONTS: i32 = 57;
const SIZE_CHANGED: i32 = 13;
const SET_PART: i32 = 14;
const CELL_CURSOR: i32 = 17;
const MOUSE_POINTER: i32 = 18;
const CELL_FORMULA: i32 = 19;
const HEADER: i32 = 33;
const GEOMETRY: i32 = 50;
const CELL_ADDRESS: i32 = 34;
const REFERENCE_MARKS: i32 = 45;
const FUNCTION_LIST: i32 = 47;
const TOOLTIP: i32 = 71;
const TABLE: i32 = 44;

/// A mixed state; kernel values never hold NUL.
pub const MIXED: &str = "\u{0}mixed";

/// A rectangle in twips: x, y, width, height.
pub type Rect = (i32, i32, i32, i32);

/// One drained batch of document events.
#[derive(Clone, Debug, Default)]
pub struct Events {
    /// Bumps on every event below.
    pub seq: u64,
    /// The document's revision when the batch was sent.
    pub revision: u64,
    /// Bumps on tile invalidations only.
    pub generation: u64,
    pub all: bool,
    pub rects: Vec<Rect>,
    pub cursor: Option<Rect>,
    pub cursor_visible: bool,
    /// The CSS pointer name; empty until the kernel says.
    pub pointer: String,
    pub selection: Vec<Rect>,
    /// Print-twips anchors for relative text rectangles.
    pub text: TextAnchors,
    /// The selected object's frame, if any.
    pub graphic: Option<Rect>,
    /// The selected image is in crop mode.
    pub cropping: bool,
    /// Bumps on comment and tracked-change events.
    pub review: u64,
    /// Bumps on each search outcome.
    pub searched: u64,
    /// Bumps when the document's size changes.
    pub resized: u64,
    /// Bumps when a dialog Ely closed has ended.
    pub dismissed: u64,
    /// The last search's matches; empty when none.
    pub found: Vec<Rect>,
    /// A context menu the kernel offered, as JSON.
    pub menu: Option<String>,
    /// Kernel messages Ely could not read.
    pub faults: Vec<String>,
    /// Font families the document names and the system lacks.
    pub fonts: Vec<String>,
    /// Kernel states changed since the last drain (LOK-05).
    pub states: Vec<(String, String)>,
    /// Dialogs the kernel opened and Ely closed (LOK-08).
    pub blocked: Vec<String>,
    pub a11y: A11y,
    /// The paragraph's own tab stops, in 1/100 mm.
    pub tabs: Vec<i32>,
    pub cell: Cell,
    pub table: Option<crate::table::Table>,
}

#[derive(Default)]
struct Slot {
    batch: Events,
    closed: HashSet<u32>,
    titles: HashMap<u32, String>,
    /// Every state the kernel reported, for the worker's checks.
    known: HashMap<String, String>,
    /// The last command completion: name and success.
    result: Option<(String, bool)>,
    /// The seq last pushed to the client.
    pushed: u64,
    /// The client zoom last set: tile pixels, tile twips.
    zoom: (i32, i32),
}

impl Slot {
    /// The batch so far; drains the one-shot parts.
    fn drain(&mut self) -> Events {
        let batch = self.batch.clone();
        self.batch.all = false;
        self.batch.rects.clear();
        self.batch.states.clear();
        self.batch.blocked.clear();
        self.batch.menu = None;
        self.batch.faults.clear();
        batch
    }
}

fn slots() -> &'static Mutex<HashMap<u64, Slot>> {
    static SLOTS: OnceLock<Mutex<HashMap<u64, Slot>>> = OnceLock::new();
    SLOTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn signal() -> &'static Condvar {
    static SIGNAL: OnceLock<Condvar> = OnceLock::new();
    SIGNAL.get_or_init(Condvar::new)
}

const KINDS: [i32; 35] = [
    TABLE,
    MOUSE_POINTER,
    REFERENCE_MARKS,
    FUNCTION_LIST,
    TOOLTIP,
    GEOMETRY,
    SIZE_CHANGED,
    SET_PART,
    CELL_CURSOR,
    CELL_FORMULA,
    HEADER,
    CELL_ADDRESS,
    FONTS,
    NOT_FOUND,
    FOUND,
    MENU,
    REDLINES,
    REDLINE,
    COMMENT,
    RULER,
    INVALIDATE_TILES,
    CURSOR,
    SELECTION,
    CURSOR_VISIBLE,
    GRAPHIC,
    STATE,
    RESULT,
    WINDOW,
    JSDIALOG,
    A11Y_FOCUS,
    A11Y_CARET,
    A11Y_SELECTION,
    A11Y_CELL,
    A11Y_EDITING,
    A11Y_OBJECT,
];

unsafe extern "C" fn on_event(kind: i32, payload: *const c_char, data: *mut c_void) {
    if !KINDS.contains(&kind) || data.is_null() {
        return;
    }
    let (id, raw) = unsafe { *data.cast::<(u64, Raw)>() };
    let text = if payload.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(payload) }
            .to_string_lossy()
            .into_owned()
    };
    let close = {
        let mut slots = slots().lock().expect("the events lock");
        let Some(slot) = slots.get_mut(&id) else { return };
        let close = record(slot, kind, &text);
        slot.batch.seq += 1;
        close
    };
    if let Some(window) = close {
        crate::worker::closing::later(id, raw, window);
    }
    signal().notify_all();
}

/// A rectangle payload; an unreadable one is a fault.
fn rect_in(batch: &mut Events, what: &str, text: &str) -> Option<Rect> {
    rect_of(text).unwrap_or_else(|Unreadable| {
        batch.faults.push(format!("an unreadable {what}: {text}"));
        None
    })
}

/// Folds one callback in; returns a dialog to close.
fn record(slot: &mut Slot, kind: i32, text: &str) -> Option<u32> {
    let batch = &mut slot.batch;
    match kind {
        INVALIDATE_TILES => {
            batch.generation += 1;
            match rect_in(batch, "invalidation", text) {
                Some(rect) => batch.rects.push(rect),
                None => batch.all = true,
            }
        }
        CURSOR => text::cursor(batch, text),
        SELECTION => text::selection(batch, text),
        CURSOR_VISIBLE => batch.cursor_visible = text == "true",
        MOUSE_POINTER => batch.pointer = text.to_string(),
        GRAPHIC => {
            batch.graphic = rect_in(batch, "graphic selection", text);
            batch.cropping = batch.graphic.is_some() && text.contains("\"isCropMode\": true");
        }
        STATE => {
            if let Some(state) = state_of(text) {
                slot.known.insert(state.0.clone(), state.1.clone());
                batch.states.retain(|(name, _)| *name != state.0);
                batch.states.push(state);
            }
        }
        RESULT => slot.result = result_of(text),
        RULER => batch.tabs = tabs_of(text),
        REDLINES | REDLINE | COMMENT => batch.review += 1,
        NOT_FOUND => {
            batch.searched += 1;
            batch.found.clear();
        }
        FOUND => match found_of(text) {
            Some(found) => {
                batch.searched += 1;
                batch.found = found;
            }
            None => batch
                .faults
                .push(format!("an unreadable search result: {text}")),
        },
        MENU => batch.menu = Some(text.to_string()),
        TABLE => crate::table::read(text).map_or_else(|fault| batch.faults.push(fault), |table| batch.table = table),
        CELL_CURSOR => batch.cell.cursor = rect_in(batch, "cell cursor", text),
        CELL_FORMULA => batch.cell.formula = text.to_string(),
        CELL_ADDRESS => batch.cell.address = text.to_string(),
        REFERENCE_MARKS => match marks_of(text) {
            Some(marks) => batch.cell.marks = marks,
            None => batch.faults.push(format!("unreadable marks: {text}")),
        },
        FUNCTION_LIST => match functions_of(text) {
            Some(functions) => {
                // "hidetip" also ends the usage hint.
                if functions.is_empty() {
                    batch.cell.usage.clear();
                }
                batch.cell.functions = functions;
            }
            None => batch.faults.push(format!("unreadable functions: {text}")),
        },
        TOOLTIP => {
            if let Some(usage) = usage_of(text) {
                batch.cell.usage = usage;
            } else if let Some(request) = request_of(text) {
                log::info!("lok: the kernel asks for {}", request.kind());
                batch.cell.requests += 1;
                batch.cell.request = Some(request);
            }
        }
        SET_PART => batch.cell.sheets += 1,
        SIZE_CHANGED => {
            batch.cell.sheets += 1;
            batch.resized += 1;
        }
        HEADER | GEOMETRY => batch.cell.headers += 1,
        FONTS => match fonts_of(text) {
            Some(fonts) => batch.fonts = fonts,
            None => batch
                .faults
                .push(format!("an unreadable font list: {text}")),
        },
        JSDIALOG if formula_text(text).is_some() => {
            // Calc's input line holds the current cell.
            batch.a11y.text = formula_text(text).expect("checked above");
        }
        WINDOW | JSDIALOG => match dialog_of(kind == JSDIALOG, text)? {
            Dialog::Titled { window, title } => { slot.titles.insert(window, title); }
            Dialog::Gone { window } if slot.closed.contains(&window) => {
                log::info!("lok: kernel dialog {window} ended");
                batch.dismissed += 1;
            }
            Dialog::Gone { .. } => {}
            Dialog::Opened { window, title } => {
                if !slot.closed.insert(window) {
                    return None;
                }
                let title = title
                    .or_else(|| slot.titles.get(&window).cloned())
                    .unwrap_or_else(|| format!("window {window}"));
                log::warn!("lok: closing kernel dialog {window} ({title})");
                batch.blocked.push(title);
                return Some(window);
            }
        },
        _ => a11y_update(&mut batch.a11y, kind, text),
    }
    None
}

/// Registers the trampoline on `doc` (LOK-03).
pub fn watch(id: u64, doc: &Document) {
    // The box leaks; a callback may still read it.
    let data = Box::into_raw(Box::new((id, doc.raw())));
    let mut slot = Slot::default();
    slot.batch.cursor_visible = true;
    slots().lock().expect("the events lock").insert(id, slot);
    doc.initialize_for_rendering();
    doc.set_callback(Some(on_event), data.cast());
}

/// Stops the trampoline and drops the slot.
pub fn unwatch(id: u64, doc: &Document) {
    doc.set_callback(None, std::ptr::null_mut());
    slots().lock().expect("the events lock").remove(&id);
    signal().notify_all();
}

/// Stops every watch before the documents die.
pub fn stop_all(docs: &HashMap<u64, Document>) {
    for doc in docs.values() {
        doc.set_callback(None, std::ptr::null_mut());
    }
    slots().lock().expect("the events lock").clear();
    signal().notify_all();
}

/// The current generation of `id`.
pub fn generation(id: u64) -> u64 {
    slots().lock().expect("the events lock").get(&id).map_or(0, |slot| slot.batch.generation)
}

/// Whether `id` shows a selection, object or caret.
pub fn picked(id: u64) -> bool {
    let slots = slots().lock().expect("the events lock");
    let Some(batch) = slots.get(&id).map(|slot| &slot.batch) else { return false };
    batch.graphic.is_some() || !batch.selection.is_empty() || batch.cursor_visible && batch.cursor.is_some()
}

/// Blocks until documents have unpushed events.
pub fn wait_unpushed() {
    let mut slots = slots().lock().expect("the events lock");
    while !slots.values().any(|slot| slot.batch.seq > slot.pushed) {
        slots = signal().wait(slots).expect("the events wait");
    }
}

/// Drains the documents with unpushed events.
pub fn unpushed() -> Vec<(u64, Events)> {
    let mut slots = slots().lock().expect("the events lock");
    slots
        .iter_mut()
        .filter(|(_, slot)| slot.batch.seq > slot.pushed)
        .map(|(id, slot)| {
            slot.pushed = slot.batch.seq;
            (*id, slot.drain())
        })
        .collect()
}

/// Forgets the last completion before a dispatch.
pub fn clear_result(id: u64) {
    if let Some(slot) = slots().lock().expect("the events lock").get_mut(&id) {
        slot.result = None;
    }
}

/// The completion the last dispatch reported.
pub fn take_result(id: u64) -> Option<(String, bool)> {
    slots().lock().expect("the events lock").get_mut(&id)?.result.take()
}

/// The bridged a11y text now.
pub fn a11y_text(id: u64) -> Option<String> {
    Some(
        slots()
            .lock()
            .expect("the events lock")
            .get(&id)?
            .batch
            .a11y
            .text
            .clone(),
    )
}

/// Waits for the a11y text to leave `before`.
pub fn wait_a11y_change(id: u64, before: &str, timeout: Duration) -> Option<String> {
    let deadline = Instant::now() + timeout;
    let mut slots = slots().lock().expect("the events lock");
    loop {
        let text = slots.get(&id)?.batch.a11y.text.clone();
        let now = Instant::now();
        if text != before || now >= deadline {
            return Some(text);
        }
        slots = signal()
            .wait_timeout(slots, deadline - now)
            .expect("the events wait")
            .0;
    }
}

/// Waits for `uno` to read `want`; returns the last state.
pub fn wait_state(id: u64, uno: &str, want: &str, timeout: Duration) -> Option<String> {
    let deadline = Instant::now() + timeout;
    let mut slots = slots().lock().expect("the events lock");
    loop {
        let state = slots.get(&id)?.known.get(uno).cloned();
        let now = Instant::now();
        if state.as_deref() == Some(want) || now >= deadline {
            return state;
        }
        slots = signal()
            .wait_timeout(slots, deadline - now)
            .expect("the events wait")
            .0;
    }
}

/// Waits until the selected image is in crop mode.
pub fn wait_cropping(id: u64, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    let mut slots = slots().lock().expect("the events lock");
    loop {
        let cropping = slots.get(&id).is_some_and(|slot| slot.batch.cropping);
        let now = Instant::now();
        if cropping || now >= deadline {
            return cropping;
        }
        slots = signal()
            .wait_timeout(slots, deadline - now)
            .expect("the events wait")
            .0;
    }
}

/// The client zoom last set on `id`, in tile units.
pub fn zoom(id: u64) -> (i32, i32) {
    slots().lock().expect("the events lock").get(&id).map_or((0, 0), |slot| slot.zoom)
}

/// A new client zoom resizes the document unannounced (Calc).
pub fn rescaled(id: u64, zoom: (i32, i32)) {
    if let Some(slot) = slots().lock().expect("the events lock").get_mut(&id)
        && slot.zoom != zoom
    {
        slot.zoom = zoom;
        slot.batch.resized += 1;
        slot.batch.seq += 1;
    }
    signal().notify_all();
}

/// Seeds the a11y text from the kernel's snapshot (§7 tail).
pub fn seed_a11y(id: u64, paragraph: &str) {
    if let Some(slot) = slots().lock().expect("the events lock").get_mut(&id) {
        a11y_update(&mut slot.batch.a11y, A11Y_FOCUS, paragraph);
        slot.batch.seq += 1;
    }
    signal().notify_all();
}
