//! Commented words, washed in their author's color (DOC-11).

use gpui::{Bounds, Context, Hsla, Pixels};

use super::LokView;
use crate::Rect;

impl LokView {
    /// Washes these document rects; the last set goes.
    pub fn wash_notes(&mut self, washes: Vec<(Rect, Hsla)>, cx: &mut Context<Self>) {
        if self.notes != washes {
            log::info!("lok: {} commented rects washed", washes.len());
            self.notes = washes;
            cx.notify();
        }
    }

    /// The washes the page paints.
    #[cfg(any(test, feature = "test-support"))]
    pub fn washed(&self) -> &[(Rect, Hsla)] {
        &self.notes
    }

    pub(super) fn note_washes(&self) -> Vec<(Bounds<Pixels>, Hsla)> {
        self.notes.iter().map(|&(rect, wash)| (self.screen_rect(rect), wash)).collect()
    }
}
