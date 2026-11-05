//! What the view knows, for apps to read.

use gpui::{FocusHandle, Pixels, Point};

use super::LokView;
use crate::Rect;

impl LokView {
    /// Whether the kernel was told Ely's scheme.
    pub fn lit(&self) -> bool {
        self.lit
    }

    /// The kernel's state for `uno`, as LOK reports it (LOK-05).
    pub fn state(&self, uno: &str) -> Option<&str> {
        self.states.get(uno).map(String::as_str)
    }

    /// The selected object's frame in twips, if any.
    pub fn graphic(&self) -> Option<Rect> {
        self.graphic
    }

    /// Bumps when comments or tracked changes change.
    pub fn review(&self) -> u64 {
        self.review
    }

    /// The cursor in twips, if the kernel shows one.
    pub fn cursor(&self) -> Option<Rect> {
        self.cursor
    }

    /// Whether the kernel shows a caret.
    pub fn caret(&self) -> bool {
        self.cursor_visible && self.cursor.is_some()
    }

    /// Counts cursor moves the kernel reported.
    pub fn moves(&self) -> u64 {
        self.moves
    }

    /// What a spreadsheet reports about its current cell.
    pub fn cell(&self) -> &crate::Cell {
        &self.cell
    }

    /// Document twips as a point inside the view.
    pub fn place(&self, at: (i32, i32)) -> gpui::Point<gpui::Pixels> {
        self.window_point(at) - self.origin
    }

    /// The view's size in pixels at the last paint.
    pub fn viewport(&self) -> (f32, f32) {
        self.viewport
    }

    /// Frozen columns and rows, in twips.
    pub fn frozen(&self) -> (i32, i32) {
        self.frozen
    }

    /// The view's top left in window coordinates.
    pub fn origin(&self) -> gpui::Point<gpui::Pixels> {
        self.origin
    }

    /// A view-local point as document twips.
    pub fn twips_at(&self, at: gpui::Point<gpui::Pixels>) -> (i32, i32) {
        self.doc_point(at + self.origin)
    }

    /// Each page's frame in twips.
    pub fn pages(&self) -> &[Rect] {
        &self.pages
    }

    /// The selection's rectangles in twips.
    pub fn selection(&self) -> &[Rect] {
        &self.selection
    }

    /// The last search's count and matches.
    pub fn search(&self) -> (u64, &[Rect]) {
        (self.search.0, &self.search.1)
    }

    /// Fonts the document names and the system lacks.
    pub fn missing_fonts(&self) -> &[String] {
        &self.fonts
    }

    /// Takes the kernel's last menu and the click that asked.
    pub fn take_menu(&mut self) -> Option<(String, Option<Point<Pixels>>)> {
        let menu = self.menu.take()?;
        Some((menu, self.right_at.take()))
    }

    /// The paragraph's tab stops in 1/100 mm, and their document.
    pub fn tabs(&self) -> (&[i32], crate::Doc) {
        let mut doc = self.doc.clone();
        doc.rev = self.tabs.1;
        (&self.tabs.0, doc)
    }

    /// The handle apps focus to type into the document.
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    /// Whether the clipboard `html` is this view's last copy.
    pub fn holds_copy(&self, html: &str) -> bool {
        super::clip::mark_of(html).is_some_and(|mark| self.copied.as_deref() == Some(mark))
    }
}
