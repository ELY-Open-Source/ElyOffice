//! A document's requests: size, pages, tiles, saving.

use std::io;

use super::{Doc, Rect, Tile, Worker};
use crate::ipc::{Pack, Unpack, tag};

/// How a PDF goes out: pages, comments, tracked changes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pdf {
    /// First and last page, from 1; None is every page.
    pub pages: Option<(u32, u32)>,
    /// Comments print in the margin.
    pub comments: bool,
    /// Tracked changes show as marked.
    pub changes: bool,
    /// Slides per handout page; 0 prints the slides.
    pub handouts: u8,
}

/// One sheet: its name and whether it shows.
#[derive(Clone, Debug, PartialEq)]
pub struct Sheet {
    pub name: String,
    pub visible: bool,
}

/// The sheets of a spreadsheet and the shown one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sheets {
    pub current: usize,
    pub list: Vec<Sheet>,
}

impl Doc {
    /// The sheets, in order, and the one shown.
    pub fn sheets(&self, worker: &mut Worker) -> io::Result<Sheets> {
        let payload = worker.request(tag::SHEETS, &Pack::new().u64(worker.doc_id(self)?).bytes())?;
        let mut unpack = Unpack::new(&payload);
        let current = usize::try_from(unpack.u64()?).map_err(io::Error::other)?;
        let count = unpack.u64()?;
        let list = (0..count)
            .map(|_| {
                let name = unpack.str()?;
                let info: serde_json::Value = serde_json::from_str(&unpack.str()?)
                    .map_err(|error| io::Error::other(format!("bad part info: {error}")))?;
                let visible = match info["visible"].as_u64() {
                    Some(1) => true,
                    Some(0) => false,
                    _ => return Err(io::Error::other(format!("no visibility: {info}"))),
                };
                Ok(Sheet { name, visible })
            })
            .collect::<io::Result<_>>()?;
        Ok(Sheets { current, list })
    }

    /// Tells the kernel the shown area and the tile scale.
    pub fn visible_area(&self, worker: &mut Worker, (x, y, w, h): Rect, (tile, tile_twips): (u32, i32)) -> io::Result<()> {
        let request = Pack::new()
            .u64(worker.doc_id(self)?)
            .i64(x.into())
            .i64(y.into())
            .i64(w.into())
            .i64(h.into())
            .i64(tile.into())
            .i64(tile_twips.into())
            .bytes();
        worker.request(tag::VISIBLE_AREA, &request)?;
        Ok(())
    }

    /// The selection as plain text; Calc joins cells by tabs.
    pub fn selection_text(&self, worker: &mut Worker) -> io::Result<String> {
        let payload = worker.request(tag::SELECTION_TEXT, &Pack::new().u64(worker.doc_id(self)?).bytes())?;
        Unpack::new(&payload).str()
    }

    /// Completes `name` if still offered (TABLE-03).
    pub fn complete_function(
        &self,
        worker: &mut Worker,
        name: Option<&str>,
        key: Option<(u32, i32)>,
    ) -> io::Result<bool> {
        let pack = Pack::new().u64(worker.doc_id(self)?);
        let pack = match name {
            Some(name) => pack.u64(1).str(name),
            None => pack.u64(0),
        };
        // Key code 0: no key, as for a click.
        let (char_code, key_code) = key.unwrap_or((0, 0));
        let request = pack
            .i64(i64::from(char_code))
            .i64(i64::from(key_code))
            .bytes();
        let reply = worker.request(tag::COMPLETE_FUNCTION, &request)?;
        Ok(Unpack::new(&reply).u64()? == 1)
    }

    /// Shows sheet `part`, from 0.
    pub fn show_sheet(&self, worker: &mut Worker, part: usize) -> io::Result<()> {
        let request = Pack::new().u64(worker.doc_id(self)?).u64(part as u64).bytes();
        worker.request(tag::SHOW_SHEET, &request)?;
        Ok(())
    }

    /// Page count or area in LOK twips.
    pub fn size(&self, worker: &mut Worker) -> io::Result<(i64, i64)> {
        let payload = worker.request(tag::DOC_SIZE, &Pack::new().u64(worker.doc_id(self)?).bytes())?;
        let mut unpack = Unpack::new(&payload);
        Ok((unpack.i64()?, unpack.i64()?))
    }

    /// The size and the client zoom it was measured at.
    pub fn size_at(&self, worker: &mut Worker) -> io::Result<((i64, i64), (i32, i32))> {
        let payload = worker.request(tag::DOC_SIZE, &Pack::new().u64(worker.doc_id(self)?).bytes())?;
        let mut unpack = Unpack::new(&payload);
        let size = (unpack.i64()?, unpack.i64()?);
        let number = |value: i64| i32::try_from(value).map_err(io::Error::other);
        Ok((size, (number(unpack.i64()?)?, number(unpack.i64()?)?)))
    }

    /// Page rectangles in twips; empty without pages.
    pub fn pages(&self, worker: &mut Worker) -> io::Result<Vec<Rect>> {
        let payload = worker.request(tag::PAGES, &Pack::new().u64(worker.doc_id(self)?).bytes())?;
        let text = Unpack::new(&payload).str()?;
        text.split(';')
            .filter(|part| !part.trim().is_empty())
            .map(|part| {
                let numbers: Vec<i32> = part
                    .split(',')
                    .map(|n| {
                        n.trim()
                            .parse()
                            .map_err(|_| io::Error::other(format!("a bad page rect: {part}")))
                    })
                    .collect::<io::Result<_>>()?;
                match numbers.as_slice() {
                    [x, y, w, h] => Ok((*x, *y, *w, *h)),
                    _ => Err(io::Error::other(format!("a bad page rect: {part}"))),
                }
            })
            .collect()
    }

    /// Paints one tile: twips rect into a pixel size.
    pub fn paint_tile(
        &self,
        worker: &mut Worker,
        twips: (i32, i32, i32, i32),
        pixels: (u32, u32),
    ) -> io::Result<Tile> {
        let (x, y, w, h) = twips;
        let (pw, ph) = pixels;
        let payload = worker.request(
            tag::PAINT_TILE,
            &Pack::new()
                .u64(worker.doc_id(self)?)
                .i64(x as i64)
                .i64(y as i64)
                .i64(w as i64)
                .i64(h as i64)
                .i64(pw as i64)
                .i64(ph as i64)
                .bytes(),
        )?;
        let mut unpack = Unpack::new(&payload);
        let generation = unpack.u64()?;
        Ok(Tile {
            generation,
            pixels: unpack.rest().to_vec(),
        })
    }

    /// Slide or master `part` whole, as `pixels` of BGRA.
    pub fn paint_part(
        &self,
        worker: &mut Worker,
        (part, master): (usize, bool),
        pixels: (u32, u32),
    ) -> io::Result<Vec<u8>> {
        let request = Pack::new()
            .u64(worker.doc_id(self)?)
            .u64(part as u64)
            .u64(u64::from(master))
            .u64(u64::from(pixels.0))
            .u64(u64::from(pixels.1))
            .bytes();
        let painted = worker.request(tag::PAINT_PART, &request)?;
        crate::show::sized(&painted, pixels)?;
        Ok(painted)
    }

    /// Slide `part`'s slideshow layers at about `pixels`.
    pub fn slide_layers(
        &self,
        worker: &mut Worker,
        part: usize,
        pixels: (u32, u32),
    ) -> io::Result<crate::show::Layers> {
        let request = Pack::new()
            .u64(worker.doc_id(self)?)
            .u64(part as u64)
            .u64(u64::from(pixels.0))
            .u64(u64::from(pixels.1))
            .bytes();
        let head = worker.request(tag::SLIDE_LAYERS, &request)?;
        let mut unpack = Unpack::new(&head);
        let (width, height, count) = (unpack.u64()?, unpack.u64()?, unpack.u64()?);
        // Layers arrive in pieces that keep each frame small.
        let mut whole = Pack::new().u64(width).u64(height).u64(count);
        for index in 0..count {
            let (json, painted) = (unpack.str()?, unpack.u64()?);
            whole = whole.str(&json).u64(painted);
            if painted != 0 {
                whole = whole.blob(&self.slide_layer(worker, index)?);
            }
        }
        crate::show::layers(&whole.bytes())
    }

    /// Layer `index` of the last layers read, piece by piece.
    fn slide_layer(&self, worker: &mut Worker, index: u64) -> io::Result<Vec<u8>> {
        let id = worker.doc_id(self)?;
        let mut pixels = Vec::new();
        loop {
            let ask = Pack::new().u64(id).u64(index).u64(pixels.len() as u64).bytes();
            let reply = worker.request(tag::SLIDE_LAYER, &ask)?;
            let mut unpack = Unpack::new(&reply);
            let (total, piece) = (unpack.u64()?, unpack.blob()?);
            pixels.extend_from_slice(&piece);
            if pixels.len() as u64 >= total {
                return Ok(pixels);
            }
            if piece.is_empty() {
                return Err(io::Error::other(format!("layer {index} stopped at byte {}", pixels.len())));
            }
        }
    }

    /// Saves to `url` as `format`; failure surfaces (LOK-09).
    pub fn save_as(&self, worker: &mut Worker, url: &str, format: &str) -> io::Result<()> {
        self.store(worker, url, format, "")
    }

    /// Writes the shown sheet as text split by `separator`.
    pub fn export_text(&self, worker: &mut Worker, url: &str, separator: char) -> io::Result<()> {
        let request = Pack::new()
            .u64(worker.doc_id(self)?)
            .str(url)
            .u64(u64::from(separator))
            .bytes();
        worker.request(tag::EXPORT_TEXT, &request)?;
        Ok(())
    }

    /// Writes a PDF of `pages` (from 1, inclusive) to `url`.
    pub fn export_pdf(&self, worker: &mut Worker, url: &str, pdf: &Pdf) -> io::Result<()> {
        let (from, to) = pdf.pages.unwrap_or((0, 0));
        let request = Pack::new()
            .u64(worker.doc_id(self)?)
            .str(url)
            .u64(u64::from(from))
            .u64(u64::from(to))
            .u64(u64::from(pdf.comments))
            .u64(u64::from(pdf.changes))
            .u64(u64::from(pdf.handouts))
            .bytes();
        worker.request(tag::EXPORT_PDF, &request)?;
        Ok(())
    }

    /// Saves and moves the document to `url`; modified clears.
    pub fn save_owned(&self, worker: &mut Worker, url: &str, format: &str) -> io::Result<()> {
        self.store(worker, url, format, "TakeOwnership")
    }

    fn store(&self, worker: &mut Worker, url: &str, format: &str, options: &str) -> io::Result<()> {
        worker.request(
            tag::SAVE_AS,
            &Pack::new()
                .u64(worker.doc_id(self)?)
                .str(url)
                .str(format)
                .str(options)
                .bytes(),
        )?;
        Ok(())
    }
}
