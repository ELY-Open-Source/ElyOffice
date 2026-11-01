//! Text blocks and drawn objects: read, write, move (PDF-03, PDF-04).

use std::{io, path::Path};

use ely_ipc::{Pack, Unpack};
use serde_json::Value;

use crate::client::{Doc, Worker};
use crate::proto::tag;

/// A text block as MuPDF reads it, in page points.
#[derive(Clone, Debug, PartialEq)]
pub struct TextBlock {
    pub rect: [f32; 4],
    pub text: String,
    pub font: String,
    pub size: f32,
    pub rgb: u32,
    /// Upright, and every glyph in the page's own content.
    pub editable: bool,
}

/// An image or form XObject drawn on the page.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawn {
    pub form: bool,
    pub rect: [f32; 4],
}

/// Text to write, replacing block `block` or new.
#[derive(Clone, Debug, PartialEq)]
pub struct Writing {
    pub block: Option<usize>,
    /// The block's text as read; a change since refuses.
    pub expected: String,
    pub text: String,
    /// "original", "helvetica", "times", "courier", "cjk-sc" and so on.
    pub font: String,
    pub size: f32,
    pub rgb: u32,
    /// Page points: left, top, width.
    pub bx: [f32; 3],
}

/// What writing did, or would do.
#[derive(Clone, Debug, PartialEq)]
pub enum Wrote {
    /// Done; the history mark after it.
    Done(String),
    /// The font lacks this character; nothing changed.
    Missing(char),
    /// The document's own font cannot write; nothing changed.
    Unwritable,
    /// BGRA of the area, its size, whether text grew.
    Preview(Vec<u8>, (u32, u32), bool),
}

fn rect(value: &Value) -> io::Result<[f32; 4]> {
    let bad = || io::Error::other(format!("a bad rectangle {value}"));
    let list = value.as_array().filter(|list| list.len() == 4).ok_or_else(bad)?;
    let mut out = [0f32; 4];
    for (at, side) in list.iter().enumerate() {
        out[at] = side.as_f64().ok_or_else(bad)? as f32;
    }
    Ok(out)
}

fn list(bytes: &[u8]) -> io::Result<Vec<Value>> {
    match serde_json::from_slice(bytes).map_err(io::Error::other)? {
        Value::Array(list) => Ok(list),
        other => Err(io::Error::other(format!("not a list: {other}"))),
    }
}

fn block(value: &Value) -> io::Result<TextBlock> {
    let bad = || io::Error::other(format!("a bad text block {value}"));
    Ok(TextBlock {
        rect: rect(&value["rect"])?,
        text: value["text"].as_str().ok_or_else(bad)?.to_string(),
        font: value["font"].as_str().ok_or_else(bad)?.to_string(),
        size: value["size"].as_f64().ok_or_else(bad)? as f32,
        rgb: value["color"].as_u64().ok_or_else(bad)? as u32,
        editable: value["editable"].as_bool().ok_or_else(bad)?,
    })
}

fn floats(ask: Pack, values: &[f32]) -> Pack {
    values.iter().fold(ask, |ask, value| ask.u64(u64::from(value.to_bits())))
}

impl Worker {
    pub fn text_blocks(&mut self, doc: Doc, page: usize) -> io::Result<Vec<TextBlock>> {
        let reply = self.request(tag::TEXT_BLOCKS, &self.on(&doc)?.u64(page as u64).bytes())?;
        list(&reply)?.iter().map(block).collect()
    }

    pub fn objects(&mut self, doc: Doc, page: usize) -> io::Result<Vec<Drawn>> {
        let reply = self.request(tag::OBJECTS, &self.on(&doc)?.u64(page as u64).bytes())?;
        list(&reply)?
            .iter()
            .map(|value| Ok(Drawn { form: value["kind"] == "form", rect: rect(&value["rect"])? }))
            .collect()
    }

    /// Writes `w` on `page`; a `scale` above zero only previews.
    pub fn put_text(&mut self, doc: Doc, page: usize, w: &Writing, scale: f32) -> io::Result<Wrote> {
        let block = w.block.map_or(-1, |block| block as i64);
        let ask = self.on(&doc)?.u64(page as u64).i64(block).str(&w.expected).str(&w.text).str(&w.font);
        let ask = floats(ask.u64(u64::from(w.size.to_bits())).u64(u64::from(w.rgb)), &w.bx);
        let reply = self.request(tag::PUT_TEXT, &ask.u64(u64::from(scale.to_bits())).bytes())?;
        let mut reply = Unpack::new(&reply);
        match reply.u64()? {
            0 => Ok(Wrote::Done(reply.str()?)),
            2 => {
                let code = u32::try_from(reply.u64()?).map_err(io::Error::other)?;
                Ok(Wrote::Missing(char::from_u32(code).ok_or_else(|| io::Error::other("a bad missing glyph"))?))
            }
            1 => {
                let size = (reply.u64()? as u32, reply.u64()? as u32);
                let grew = reply.u64()? != 0;
                Ok(Wrote::Preview(reply.blob()?, size, grew))
            }
            3 => Ok(Wrote::Unwritable),
            other => Err(io::Error::other(format!("a text reply of kind {other}"))),
        }
    }

    /// Moves or scales draw `index` by page-space `t`; the mark.
    pub fn place(&mut self, doc: Doc, page: usize, index: usize, t: [f32; 6]) -> io::Result<String> {
        self.placing(doc, page, index, t, false, "")
    }

    pub fn delete_object(&mut self, doc: Doc, page: usize, index: usize) -> io::Result<String> {
        self.placing(doc, page, index, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], true, "")
    }

    /// Swaps image `index` for `path`, fitted in its box.
    pub fn replace_image(&mut self, doc: Doc, page: usize, index: usize, path: &Path) -> io::Result<String> {
        let path = path.to_str().ok_or_else(|| io::Error::other("a path that is not UTF-8"))?;
        self.placing(doc, page, index, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], false, path)
    }

    fn placing(&mut self, doc: Doc, page: usize, index: usize, t: [f32; 6], drop: bool, swap: &str) -> io::Result<String> {
        let ask = floats(self.on(&doc)?.u64(page as u64).u64(index as u64), &t);
        let reply = self.request(tag::PLACE, &ask.u64(u64::from(drop)).str(swap).bytes())?;
        Unpack::new(&reply).str()
    }

    /// Moves text block `block` by `by`, as drawn; the mark.
    pub fn move_text(&mut self, doc: Doc, page: usize, (block, expected): (usize, &str), by: (f32, f32)) -> io::Result<String> {
        let ask = floats(self.on(&doc)?.u64(page as u64).u64(block as u64).str(expected), &[by.0, by.1]);
        Unpack::new(&self.request(tag::MOVE_TEXT, &ask.bytes())?).str()
    }

    /// Draws the image at `path` into page-space `rect`; the mark.
    pub fn add_image(&mut self, doc: Doc, page: usize, path: &Path, rect: [f32; 4]) -> io::Result<String> {
        let path = path.to_str().ok_or_else(|| io::Error::other("a path that is not UTF-8"))?;
        let ask = floats(self.on(&doc)?.u64(page as u64).str(path), &rect);
        Unpack::new(&self.request(tag::ADD_IMAGE, &ask.bytes())?).str()
    }
}
