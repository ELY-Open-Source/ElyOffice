//! Calc reports print twips; tiles and clicks take tile twips.

use std::{io, sync::Mutex};

use gpui::Context;

use super::{Geom, Layout, LokView};
use crate::{Doc, Rect, Worker, sheet::Geometry};

/// A layout and the client zoom it was measured at.
pub(super) type Measured = io::Result<(Layout, (i32, i32))>;

/// Reads the layout under the worker lock.
pub(super) fn measure(worker: &Mutex<Worker>, doc: &Doc) -> Measured {
    let mut worker = worker.lock().expect("the worker lock");
    let (size, zoom) = doc.size_at(&mut worker)?;
    Ok(((size, doc.pages(&mut worker)?), zoom))
}

impl LokView {
    /// Lays the sheet out the kernel's way.
    pub fn set_geometry(&mut self, geometry: Geometry, cx: &mut Context<Self>) {
        if self.sheet.as_ref() != Some(&geometry) {
            self.sheet = Some(geometry);
            self.clamp_scroll();
            cx.notify();
        }
    }

    /// A print-twips point in tile twips.
    pub(super) fn tile_of(&self, at: (i32, i32)) -> (i32, i32) {
        self.tile_at(self.geom, at)
    }

    /// A print-twips point in tile twips at `geom`.
    pub(super) fn tile_at(&self, geom: Geom, (x, y): (i32, i32)) -> (i32, i32) {
        match &self.sheet {
            Some(sheet) => (sheet.columns.tile(x, geom.ppt()), sheet.rows.tile(y, geom.ppt())),
            None => (x, y),
        }
    }

    /// A tile-twips point in print twips.
    pub(super) fn print_of(&self, at: (i32, i32)) -> (i32, i32) {
        self.print_at(self.geom, at)
    }

    /// A tile-twips point at `geom` in print twips.
    pub(super) fn print_at(&self, geom: Geom, (x, y): (i32, i32)) -> (i32, i32) {
        match &self.sheet {
            Some(sheet) => (sheet.columns.print(x, geom.ppt()), sheet.rows.print(y, geom.ppt())),
            None => (x, y),
        }
    }

    /// A print-twips rect in tile twips.
    pub(super) fn tile_rect(&self, rect: Rect) -> Rect {
        self.tile_rect_at(self.geom, rect)
    }

    /// A print-twips rect in tile twips at `geom`.
    pub(super) fn tile_rect_at(&self, geom: Geom, rect: Rect) -> Rect {
        self.sheet.as_ref().map_or(rect, |sheet| sheet.text_rect(rect, None, geom.ppt()))
    }

    /// The frozen extent in tile twips.
    pub(super) fn frozen_tiles(&self) -> (i32, i32) {
        self.tile_of(self.frozen)
    }

    /// Rescales, keeping the scrolling pane's start in place.
    pub(super) fn keep_scroll(&mut self, change: impl FnOnce(&mut Self)) {
        // The view's own start, not a held screen's.
        let (fx, fy) = self.frozen_tiles();
        let start = self.print_of((fx + self.scroll.0, fy + self.scroll.1));
        self.carry_size(change);
        let ((x, y), (fx, fy)) = (self.tile_of(start), self.frozen_tiles());
        self.scroll = (x - fx, y - fy);
    }

    /// Rescales; the measured size moves to the new scale too.
    pub(super) fn carry_size(&mut self, change: impl FnOnce(&mut Self)) {
        // Until measured again, scrolls clamp against this.
        let print = self.print_of(self.doc_size);
        change(self);
        self.doc_size = self.tile_of(print);
    }

    /// A sheet size the kernel measured at `zoom`, at ours.
    pub(super) fn at_scale(&self, size: (i32, i32), (pixels, twips): (i32, i32)) -> (i32, i32) {
        let Some(sheet) = self.sheet.as_ref().filter(|_| pixels > 0 && twips > 0) else {
            return size;
        };
        let theirs = f64::from(pixels) / f64::from(twips);
        let print = (sheet.columns.print(size.0, theirs), sheet.rows.print(size.1, theirs));
        self.tile_of(print)
    }

    /// Where the shown scrolling pane starts, in print twips.
    pub fn scrolled_start(&self) -> (i32, i32) {
        let (scroll, geom, _) = self.seen();
        let (fx, fy) = self.tile_at(geom, self.frozen);
        self.print_at(geom, (fx + scroll.0, fy + scroll.1))
    }
}
