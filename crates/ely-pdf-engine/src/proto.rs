//! What the app asks the PDF worker, and the answers.

use std::io;

use serde_json::Value;

/// Request tags; replies are OK or ERR.
pub mod tag {
    pub const OPEN: u8 = 1;
    pub const UNLOCK: u8 = 2;
    pub const PAGES: u8 = 3;
    pub const BOUNDS: u8 = 4;
    pub const RENDER: u8 = 5;
    pub const TEXT: u8 = 6;
    pub const SEARCH: u8 = 7;
    pub const OUTLINE: u8 = 8;
    pub const CLOSE: u8 = 9;
    /// The worker's build stamp, checked on spawn.
    pub const HELLO: u8 = 10;
    /// Edits answer with the history mark after them.
    pub const HISTORY: u8 = 11;
    pub const UNDO: u8 = 12;
    pub const DELETE: u8 = 13;
    pub const MOVE: u8 = 14;
    pub const ROTATE: u8 = 15;
    pub const CROP: u8 = 16;
    pub const BLANK: u8 = 17;
    pub const INSERT: u8 = 18;
    pub const EXTRACT: u8 = 19;
    pub const SAVE: u8 = 20;
    pub const TEXT_BLOCKS: u8 = 21;
    pub const OBJECTS: u8 = 22;
    /// Writes text; with a scale, only previews it.
    pub const PUT_TEXT: u8 = 23;
    pub const PLACE: u8 = 24;
    pub const ADD_IMAGE: u8 = 25;
    /// A page's annotations and fields, as JSON.
    pub const ANNOTS: u8 = 26;
    pub const ANNOTATE: u8 = 27;
    pub const CHANGE_ANNOT: u8 = 28;
    pub const DELETE_ANNOT: u8 = 29;
    pub const ADD_FIELD: u8 = 30;
    pub const FILL: u8 = 31;
    pub const CHANGE_FIELD: u8 = 32;
    /// Every field name in the form.
    pub const FIELD_NAMES: u8 = 33;
    /// Reads one page's lines; changes nothing.
    pub const OCR_PAGE: u8 = 34;
    pub const TEXT_LAYER: u8 = 35;
    pub const OUTLINE_EDIT: u8 = 36;
    pub const LINKS: u8 = 37;
    pub const LINK_EDIT: u8 = 38;
    pub const METADATA: u8 = 39;
    pub const SET_METADATA: u8 = 40;
    pub const ATTACHMENTS: u8 = 41;
    /// An attachment's bytes.
    pub const ATTACHMENT: u8 = 42;
    pub const ATTACH: u8 = 43;
    pub const DETACH: u8 = 44;
    /// Writes a new file with regions removed, then checks it.
    pub const REDACT: u8 = 45;
    pub const CLEAN_SCAN: u8 = 46;
    pub const CLEAN: u8 = 47;
    pub const PERMISSIONS: u8 = 48;
    /// Tries an owner password; a wrong one changes nothing.
    pub const OWNER: u8 = 49;
    pub const PROTECT: u8 = 50;
    /// A file's signature fields and their checks, as JSON.
    pub const SIGNATURES: u8 = 51;
    /// Writes a signed copy; the document is untouched.
    pub const SIGN: u8 = 52;
    /// How many fields hold a signature; nothing checked.
    pub const SIGNED: u8 = 53;
    pub const EXPORT_IMAGE: u8 = 54;
    pub const EXPORT_TEXT: u8 = 55;
    /// Writes a copy compressed and downsampled.
    pub const OPTIMIZE: u8 = 56;
    /// Writes a copy to print; needs leave to print.
    pub const PRINT_COPY: u8 = 57;
    /// What MuPDF warned when it repaired the file.
    pub const REPAIR_REPORT: u8 = 58;
    /// Decodes every picture; answers whether any failed.
    pub const CHECK_PICTURES: u8 = 59;
    /// A folder's HTML page as a new PDF; no document.
    pub const HTML_PDF: u8 = 60;
    pub const OK: u8 = 100;
    pub const ERR: u8 = 101;
}

/// What a document allows, as Ely's bits.
pub mod may {
    pub const PRINT: u32 = 1;
    pub const COPY: u32 = 2;
    pub const EDIT: u32 = 4;
    pub const ANNOTATE: u32 = 8;
    pub const FILL: u32 = 16;
    pub const ASSEMBLE: u32 = 32;
    /// Opened with the owner password, or not protected.
    pub const OWNER: u32 = 64;
    pub const NAMES: [(u32, &str); 7] = [
        (PRINT, "print"),
        (COPY, "copy"),
        (EDIT, "edit"),
        (ANNOTATE, "annotate"),
        (FILL, "fill"),
        (ASSEMBLE, "assemble"),
        (OWNER, "owner"),
    ];
    /// A refusal's error text starts with this.
    pub const REFUSED: &str = "restricted:";
}

/// The stamp of the sources this was built from.
pub const BUILD: &str = env!("ELY_PDF_BUILD");

/// One character on a page, in reading order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Char {
    /// Which text line it is on, counted over the page.
    pub line: i32,
    pub c: char,
    /// x0, y0, x1, y1 in page points.
    pub rect: [f32; 4],
}

/// Parses a BOUNDS reply: `count` page boxes, each with area.
pub fn boxes(bytes: &[u8], count: usize) -> io::Result<Vec<[f32; 4]>> {
    let mut reply = ely_ipc::Unpack::new(bytes);
    let mut boxes = Vec::with_capacity(count);
    for _ in 0..count {
        let mut one = [0f32; 4];
        for value in &mut one {
            *value = f32::from_bits(reply.u64()? as u32);
        }
        let [x0, y0, x1, y1] = one;
        // Layout divides by these.
        if !one.iter().all(|value| value.is_finite()) || x1 <= x0 || y1 <= y0 {
            return Err(io::Error::other(format!("a page box {one:?} without finite area")));
        }
        boxes.push(one);
    }
    match reply.rest().is_empty() {
        true => Ok(boxes),
        false => Err(io::Error::other("a bounds reply too long")),
    }
}

/// Bytes a character takes in a TEXT reply.
pub const CHAR_BYTES: usize = 24;

/// Parses a TEXT reply: line, code point, four box floats.
pub fn chars(bytes: &[u8]) -> io::Result<Vec<Char>> {
    if !bytes.len().is_multiple_of(CHAR_BYTES) {
        return Err(io::Error::other("a text reply of odd length"));
    }
    let word =
        |at: &[u8], n: usize| <[u8; 4]>::try_from(&at[n * 4..n * 4 + 4]).expect("four bytes");
    bytes
        .chunks_exact(CHAR_BYTES)
        .map(|one| {
            let code = u32::from_ne_bytes(word(one, 1));
            Ok(Char {
                line: i32::from_ne_bytes(word(one, 0)),
                c: char::from_u32(code)
                    .ok_or_else(|| io::Error::other(format!("no character {code}")))?,
                rect: [2, 3, 4, 5].map(|n| f32::from_ne_bytes(word(one, n))),
            })
        })
        .collect()
}

/// Parses a SEARCH reply: per hit, count then quads.
pub fn hits(bytes: &[u8]) -> io::Result<Vec<Vec<[f32; 8]>>> {
    let short = || io::Error::other("a short search reply");
    let (mut hits, mut at) = (Vec::new(), 0);
    let word = |at: &mut usize| -> io::Result<[u8; 4]> {
        let four = bytes.get(*at..*at + 4).ok_or_else(short)?;
        *at += 4;
        Ok(four.try_into().expect("four bytes"))
    };
    while at < bytes.len() {
        let count = usize::try_from(i32::from_ne_bytes(word(&mut at)?)).map_err(|_| short())?;
        let mut quads = Vec::with_capacity(count);
        for _ in 0..count {
            let mut quad = [0f32; 8];
            for value in &mut quad {
                *value = f32::from_ne_bytes(word(&mut at)?);
            }
            quads.push(quad);
        }
        hits.push(quads);
    }
    Ok(hits)
}

/// One bookmark and those under it.
#[derive(Clone, Debug, PartialEq)]
pub struct Bookmark {
    pub title: String,
    /// Its page from zero; none for outside links.
    pub page: Option<usize>,
    /// Where on that page, in points from the top.
    pub y: f32,
    pub down: Vec<Bookmark>,
}

fn bookmark(value: &Value) -> io::Result<Bookmark> {
    let bad = || io::Error::other(format!("a bad bookmark {value}"));
    let page = value["page"].as_i64().ok_or_else(bad)?;
    Ok(Bookmark {
        title: value["title"].as_str().ok_or_else(bad)?.to_string(),
        page: usize::try_from(page).ok(),
        y: value["y"].as_f64().ok_or_else(bad)? as f32,
        down: bookmarks(&value["down"])?,
    })
}

fn bookmarks(value: &Value) -> io::Result<Vec<Bookmark>> {
    let list = value
        .as_array()
        .ok_or_else(|| io::Error::other(format!("no bookmark list in {value}")))?;
    list.iter().map(bookmark).collect()
}

/// Parses an OUTLINE reply.
pub fn outline(json: &str) -> io::Result<Vec<Bookmark>> {
    bookmarks(&serde_json::from_str(json).map_err(io::Error::other)?)
}

/// A request for whatever state the document is in.
pub const ANY: u64 = u64::MAX;
/// How a refusal for a changed document begins.
pub const STALE: &str = "stale: ";

#[cfg(test)]
mod tests {
    fn reply(boxes: &[[f32; 4]]) -> Vec<u8> {
        let pack = boxes.iter().flatten().fold(ely_ipc::Pack::new(), |pack, value| pack.u64(u64::from(value.to_bits())));
        pack.bytes()
    }

    #[test]
    fn page_boxes_need_a_finite_area() {
        let letter = [0.0, 0.0, 612.0, 792.0];
        assert_eq!(super::boxes(&reply(&[letter]), 1).expect("a page"), [letter]);
        let wide = [0.0, 0.0, f32::INFINITY, 792.0];
        let unread = [0.0, 0.0, f32::NAN, 792.0];
        for bad in [wide, unread, [0.0, 0.0, 0.0, 792.0], [0.0, 10.0, 612.0, 10.0]] {
            let said = super::boxes(&reply(&[letter, bad]), 2).expect_err("refused");
            assert!(said.to_string().contains("without finite area"), "{said}");
        }
    }
}
