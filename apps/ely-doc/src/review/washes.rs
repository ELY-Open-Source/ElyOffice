//! Commented words washed; the picked thread darker (DOC-11).

use ely_lok::{Rect, review::Note};
use gpui::{App, Context, Hsla, SharedString};

use super::{Review, threads::author};

/// The collab library's wash and picked strengths.
const WASH: f32 = 0.18;
const PICKED: f32 = 0.36;

impl Review {
    /// Each open thread's words, washed in its author's color.
    pub(super) fn wash(&self, cx: &mut Context<Self>) {
        let Some(shown) = &self.shown else {
            return;
        };
        let washes = washes(&self.notes, self.active.as_deref(), cx);
        shown.view.update(cx, |view, cx| view.wash_notes(washes, cx));
    }

    /// The caret inside a thread picks it; leaving lets go.
    pub(super) fn follow_caret(&mut self, cx: &mut Context<Self>) {
        let Some(shown) = &self.shown else {
            return;
        };
        let caret = shown.view.read(cx).cursor();
        let at = caret.and_then(|caret| thread_at(&self.notes, caret)).map(SharedString::from);
        if at == self.inside {
            return;
        }
        // The caret's pick leaves with it; the sidebar's stays.
        if self.by_caret && self.active == self.inside {
            log::info!("doc: the caret leaves its thread");
            self.active = None;
        }
        self.by_caret = false;
        if let Some(key) = at.as_ref().filter(|key| self.active.as_ref() != Some(*key)) {
            log::info!("doc: the caret picks thread {key}");
            (self.active, self.by_caret) = (Some(key.clone()), true);
        }
        self.inside = at;
        self.wash(cx);
        cx.notify();
    }
}

/// The open thread whose words hold the caret's middle.
fn thread_at(notes: &[Note], (x, y, _, h): Rect) -> Option<&str> {
    let middle = y + h / 2;
    let holds = |&(left, top, width, height): &Rect| {
        (left..=left + width).contains(&x) && (top..=top + height).contains(&middle)
    };
    notes
        .iter()
        .filter(|note| note.parent == "0" && !note.resolved)
        .find(|note| note.ranges.iter().any(holds))
        .map(|note| note.id.as_str())
}

/// Each open thread's first comment's rects; resolved ones stay plain.
pub(super) fn washes(notes: &[Note], picked: Option<&str>, cx: &App) -> Vec<(Rect, Hsla)> {
    notes
        .iter()
        .filter(|note| note.parent == "0" && !note.resolved)
        .flat_map(|note| {
            let strength = if picked == Some(note.id.as_str()) { PICKED } else { WASH };
            let wash = author(notes, note).color(cx).opacity(strength);
            note.ranges.iter().map(move |&rect| (rect, wash))
        })
        .collect()
}
