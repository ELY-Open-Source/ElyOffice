//! The shim's text and object calls (PDF-03, PDF-04).

use std::{
    ffi::{c_char, c_int},
    io, ptr,
};

use ely_ipc::{Pack, Unpack};

use crate::ffi::{Context, Document, Mupdf, Open, checked, page, text};

unsafe extern "C" {
    fn ely_text_blocks(ctx: *mut Context, doc: *mut Document, page: c_int, data: *mut *mut u8, len: *mut usize) -> c_int;
    fn ely_objects(ctx: *mut Context, doc: *mut Document, page: c_int, data: *mut *mut u8, len: *mut usize) -> c_int;
    fn ely_put_text(
        ctx: *mut Context,
        doc: *mut Document,
        page: c_int,
        block: c_int,
        expected: *const c_char,
        text: *const c_char,
        font: *const c_char,
        size: f32,
        rgb: c_int,
        bx: *const f32,
        password: *const c_char,
        scale: f32,
        missing: *mut c_int,
        grew: *mut c_int,
        pixels: *mut *mut u8,
        w: *mut c_int,
        h: *mut c_int,
    ) -> c_int;
    fn ely_place(
        ctx: *mut Context,
        doc: *mut Document,
        page: c_int,
        index: c_int,
        t: *const f32,
        drop: c_int,
        path: *const c_char,
    ) -> c_int;
    fn ely_add_image(ctx: *mut Context, doc: *mut Document, page: c_int, path: *const c_char, rect: *const f32) -> c_int;
}

/// New text and where it goes.
pub struct Writing {
    /// The block replaced, or none for new text.
    pub block: Option<usize>,
    pub expected: String,
    pub text: String,
    pub font: String,
    pub size: f32,
    pub rgb: u32,
    /// Page space: left, top, width.
    pub bx: [f32; 3],
}

/// What a text write did.
pub enum Wrote {
    Done,
    /// A character the font cannot draw; nothing changed.
    Missing(char),
    /// The document's own font cannot write; nothing changed.
    Unwritable,
    /// The would-be result: BGRA, size, and whether it grew.
    Preview(Vec<u8>, (u32, u32), bool),
}

fn number(value: usize) -> io::Result<c_int> {
    c_int::try_from(value).map_err(|_| io::Error::other("a number too large"))
}

impl Mupdf {
    pub fn text_blocks(&self, doc: &Open, at: usize) -> io::Result<Vec<u8>> {
        let at = page(at)?;
        self.owned(|data, len| unsafe { ely_text_blocks(self.ctx, doc.0, at, data, len) })
    }

    pub fn objects(&self, doc: &Open, at: usize) -> io::Result<Vec<u8>> {
        let at = page(at)?;
        self.owned(|data, len| unsafe { ely_objects(self.ctx, doc.0, at, data, len) })
    }

    /// Writes `w` on page `at`; a positive `scale` only previews.
    pub fn put_text(&self, doc: &Open, at: usize, w: &Writing, password: &str, scale: f32) -> io::Result<Wrote> {
        let block = w.block.map_or(Ok(-1), number)?;
        let (expected, words, font) = (text(&w.expected)?, text(&w.text)?, text(&w.font)?);
        let password = text(password)?;
        let rgb = c_int::try_from(w.rgb & 0xff_ffff).map_err(io::Error::other)?;
        let (mut missing, mut grew, mut pixels, mut width, mut height) = (0, 0, ptr::null_mut(), 0, 0);
        let status = checked(unsafe {
            ely_put_text(
                self.ctx,
                doc.0,
                page(at)?,
                block,
                expected.as_ptr(),
                words.as_ptr(),
                font.as_ptr(),
                w.size,
                rgb,
                w.bx.as_ptr(),
                password.as_ptr(),
                scale,
                &mut missing,
                &mut grew,
                &mut pixels,
                &mut width,
                &mut height,
            )
        })?;
        if status == 3 {
            return Ok(Wrote::Unwritable);
        }
        if status == 2 {
            let missing = u32::try_from(missing).ok().and_then(char::from_u32);
            return Ok(Wrote::Missing(missing.ok_or_else(|| io::Error::other("a missing glyph out of range"))?));
        }
        if scale <= 0.0 {
            return Ok(Wrote::Done);
        }
        let size = (u32::try_from(width).map_err(io::Error::other)?, u32::try_from(height).map_err(io::Error::other)?);
        let bytes = unsafe { std::slice::from_raw_parts(pixels, size.0 as usize * size.1 as usize * 4) }.to_vec();
        self.free(pixels);
        Ok(Wrote::Preview(bytes, size, grew != 0))
    }

    /// Moves draw `index` by page-space `t`, swaps or drops it.
    pub fn place(&self, doc: &Open, at: usize, index: usize, t: [f32; 6], drop: bool, swap: Option<&str>) -> io::Result<()> {
        let swap = swap.map(text).transpose()?;
        let path = swap.as_ref().map_or(ptr::null(), |path| path.as_ptr());
        let index = number(index)?;
        checked(unsafe { ely_place(self.ctx, doc.0, page(at)?, index, t.as_ptr(), c_int::from(drop), path) }).map(|_| ())
    }

    /// Draws the image at `path` into page-space `rect`.
    pub fn add_image(&self, doc: &Open, at: usize, path: &str, rect: [f32; 4]) -> io::Result<()> {
        let path = text(path)?;
        checked(unsafe { ely_add_image(self.ctx, doc.0, page(at)?, path.as_ptr(), rect.as_ptr()) }).map(|_| ())
    }
}

impl crate::Worker {
    /// Writes or previews text; a missing glyph changes nothing.
    pub(crate) fn put_text(&mut self, id: u64, ask: &mut Unpack) -> io::Result<Vec<u8>> {
        let at = ask.u64()? as usize;
        let block = usize::try_from(ask.i64()?).ok();
        let (expected, text, font) = (ask.str()?, ask.str()?, ask.str()?);
        let size = f32::from_bits(ask.u64()? as u32);
        let rgb = ask.u64()? as u32;
        let mut bx = [0f32; 3];
        for value in &mut bx {
            *value = f32::from_bits(ask.u64()? as u32);
        }
        let scale = f32::from_bits(ask.u64()? as u32);
        let writing = Writing { block, expected, text, font, size, rgb, bx };
        let doc = self.docs.get_mut(&id).ok_or_else(|| io::Error::other(format!("no document {id}")))?;
        let wrote = self.mupdf.put_text(&doc.open, at, &writing, &doc.password, scale);
        if matches!(wrote, Ok(Wrote::Done)) {
            self.edits += 1;
        }
        let made = matches!(wrote, Ok(Wrote::Done)).then_some(self.edits);
        doc.sync(&self.mupdf, made)?;
        Ok(match wrote? {
            Wrote::Done => Pack::new().u64(0).str(&doc.mark()).bytes(),
            Wrote::Missing(c) => Pack::new().u64(2).u64(u64::from(c)).bytes(),
            Wrote::Unwritable => Pack::new().u64(3).bytes(),
            Wrote::Preview(pixels, (w, h), grew) => {
                Pack::new().u64(1).u64(u64::from(w)).u64(u64::from(h)).u64(u64::from(grew)).blob(&pixels).bytes()
            }
        })
    }
}
