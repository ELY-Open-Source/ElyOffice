//! Formula text keeps its cell anchor across scrolling and zoom.

use gpui::{Bounds, Pixels};

use super::{Geom, LokView, reveal::Seen};
use crate::{Events, Rect};

#[derive(Default)]
pub(super) struct TextGeometry {
    cursor: Option<(Rect, (i32, i32))>,
    selection: Option<(Vec<Rect>, (i32, i32))>,
}

impl LokView {
    /// Whether Calc selected formula text rather than cells.
    pub fn cell_text_selected(&self) -> bool {
        self.text
            .selection
            .as_ref()
            .is_some_and(|(rects, _)| !rects.is_empty())
    }

    fn text_tile_rect(&self, geom: Geom, rect: Rect, anchor: (i32, i32)) -> Rect {
        if let Some(sheet) = &self.sheet {
            sheet.text_rect(rect, Some(anchor), geom.ppt())
        } else {
            (anchor.0 + rect.0, anchor.1 + rect.1, rect.2, rect.3)
        }
    }

    fn text_print_rect(&self, rect: Rect, anchor: (i32, i32)) -> Rect {
        let (x, y, w, h) = self.text_tile_rect(self.geom, rect, anchor);
        let (left, top) = self.print_of((x, y));
        let (right, bottom) = self.print_of((x + w, y + h));
        (left, top, right - left, bottom - top)
    }

    pub(super) fn read_text(&mut self, batch: &mut Events) -> bool {
        let cursor = batch.cursor.zip(batch.text.cursor);
        let moved = match (self.text.cursor, cursor) {
            (None, None) => self.cursor != batch.cursor,
            (before, after) => before != after,
        };
        self.text.cursor = cursor;
        self.text.selection = batch
            .text
            .selection
            .map(|anchor| (std::mem::take(&mut batch.selection), anchor));
        if let Some((rect, anchor)) = self.text.cursor {
            batch.cursor = Some(self.text_print_rect(rect, anchor));
        }
        if let Some((rects, anchor)) = &self.text.selection {
            batch.selection = rects
                .iter()
                .map(|&rect| self.text_print_rect(rect, *anchor))
                .collect();
        }
        moved
    }

    pub(super) fn text_caret(&self, seen: Seen) -> Option<Bounds<Pixels>> {
        let geom = seen.1;
        let rect = match self.text.cursor {
            Some((rect, anchor)) => self.text_tile_rect(geom, rect, anchor),
            None => self.tile_rect_at(geom, self.cursor?),
        };
        (rect.3 > 0 && self.tile_visible(seen, rect)).then(|| self.tile_screen_rect(seen, rect))
    }

    pub(super) fn text_selection(&self) -> Vec<Bounds<Pixels>> {
        let seen = self.seen();
        let geom = seen.1;
        match &self.text.selection {
            Some((rects, anchor)) => rects
                .iter()
                .map(|&rect| self.text_tile_rect(geom, rect, *anchor))
                .filter(|&rect| rect.2 > 0 && rect.3 > 0 && self.tile_visible(seen, rect))
                .map(|rect| self.tile_screen_rect(seen, rect))
                .collect(),
            None => self
                .selection
                .iter()
                .map(|&rect| self.tile_rect_at(geom, rect))
                .filter(|&rect| rect.2 > 0 && rect.3 > 0 && self.tile_visible(seen, rect))
                .map(|rect| self.tile_screen_rect(seen, rect))
                .collect(),
        }
    }
}
