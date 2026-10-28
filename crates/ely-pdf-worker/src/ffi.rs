//! The C shim's calls; each failure leaves a message behind.

use std::{
    ffi::{CStr, CString, c_char, c_int, c_void},
    io, ptr,
};

#[repr(C)]
pub struct Context {
    _private: [u8; 0],
}

#[repr(C)]
pub struct Document {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn ely_error() -> *const c_char;
    fn ely_init() -> *mut Context;
    fn ely_open(ctx: *mut Context, path: *const c_char, out: *mut *mut Document) -> c_int;
    fn ely_close(ctx: *mut Context, doc: *mut Document);
    fn ely_needs_password(ctx: *mut Context, doc: *mut Document) -> c_int;
    fn ely_authenticate(ctx: *mut Context, doc: *mut Document, password: *const c_char) -> c_int;
    fn ely_count_pages(ctx: *mut Context, doc: *mut Document, out: *mut c_int) -> c_int;
    fn ely_page_bounds(ctx: *mut Context, doc: *mut Document, page: c_int, out: *mut f32) -> c_int;
    fn ely_render(
        ctx: *mut Context,
        doc: *mut Document,
        page: c_int,
        scale: f32,
        rotate: c_int,
        tile: *const c_int,
        bgra: *mut u8,
    ) -> c_int;
    fn ely_text(
        ctx: *mut Context,
        doc: *mut Document,
        page: c_int,
        data: *mut *mut u8,
        len: *mut usize,
    ) -> c_int;
    fn ely_search(
        ctx: *mut Context,
        doc: *mut Document,
        page: c_int,
        needle: *const c_char,
        data: *mut *mut u8,
        len: *mut usize,
    ) -> c_int;
    fn ely_outline(
        ctx: *mut Context,
        doc: *mut Document,
        data: *mut *mut u8,
        len: *mut usize,
    ) -> c_int;
    fn ely_free(ctx: *mut Context, data: *mut c_void);
    fn ely_script_font(data: *const u8, len: usize);
}

/// Great Vibes (OFL), for typed signatures.
static SCRIPT: &[u8] = include_bytes!("../fonts/GreatVibes-Regular.ttf");

/// Hands MuPDF the script face, alive while the worker is.
pub fn script_font() {
    unsafe { ely_script_font(SCRIPT.as_ptr(), SCRIPT.len()) }
}

fn error() -> io::Error {
    let text = unsafe { CStr::from_ptr(ely_error()) };
    io::Error::other(text.to_string_lossy().into_owned())
}

pub(crate) fn checked(code: c_int) -> io::Result<c_int> {
    match code {
        -1 => Err(error()),
        other => Ok(other),
    }
}

pub(crate) fn text(value: &str) -> io::Result<CString> {
    CString::new(value).map_err(|_| io::Error::other("text with a NUL byte"))
}

pub(crate) fn page(number: usize) -> io::Result<c_int> {
    c_int::try_from(number).map_err(|_| io::Error::other("a page number too large"))
}

/// The one MuPDF context; the worker is single threaded.
pub struct Mupdf {
    pub(crate) ctx: *mut Context,
}

/// An open document, closed by `Mupdf::close`.
pub struct Open(pub(crate) *mut Document);

impl Mupdf {
    pub fn new() -> io::Result<Self> {
        let ctx = unsafe { ely_init() };
        match ctx.is_null() {
            true => Err(error()),
            false => Ok(Mupdf { ctx }),
        }
    }

    pub fn open(&self, path: &str) -> io::Result<Open> {
        let path = text(path)?;
        let mut doc = ptr::null_mut();
        checked(unsafe { ely_open(self.ctx, path.as_ptr(), &mut doc) })?;
        Ok(Open(doc))
    }

    pub fn close(&self, doc: Open) {
        unsafe { ely_close(self.ctx, doc.0) }
    }

    pub fn locked(&self, doc: &Open) -> io::Result<bool> {
        checked(unsafe { ely_needs_password(self.ctx, doc.0) }).map(|locked| locked != 0)
    }

    /// MuPDF's access bits; 0 is refused, 4 the owner.
    pub fn unlock(&self, doc: &Open, password: &str) -> io::Result<u32> {
        let password = text(password)?;
        checked(unsafe { ely_authenticate(self.ctx, doc.0, password.as_ptr()) }).map(|access| access as u32)
    }

    pub fn pages(&self, doc: &Open) -> io::Result<usize> {
        let mut count = 0;
        checked(unsafe { ely_count_pages(self.ctx, doc.0, &mut count) })?;
        usize::try_from(count).map_err(io::Error::other)
    }

    pub fn bounds(&self, doc: &Open, number: usize) -> io::Result<[f32; 4]> {
        let mut out = [0f32; 4];
        checked(unsafe { ely_page_bounds(self.ctx, doc.0, page(number)?, out.as_mut_ptr()) })?;
        Ok(out)
    }

    pub fn render(
        &self,
        doc: &Open,
        number: usize,
        (scale, rotate): (f32, i32),
        tile: [i32; 4],
    ) -> io::Result<Vec<u8>> {
        let (w, h) = (usize::try_from(tile[2]), usize::try_from(tile[3]));
        let (Ok(w), Ok(h)) = (w, h) else {
            return Err(io::Error::other("a tile of negative size"));
        };
        let mut pixels = vec![0u8; w * h * 4];
        let code = unsafe {
            ely_render(
                self.ctx,
                doc.0,
                page(number)?,
                scale,
                rotate,
                tile.as_ptr(),
                pixels.as_mut_ptr(),
            )
        };
        checked(code)?;
        Ok(pixels)
    }

    /// Frees memory the shim handed over.
    pub(crate) fn free(&self, data: *mut u8) {
        unsafe { ely_free(self.ctx, data.cast()) }
    }

    /// Takes a buffer the shim made, then frees it.
    pub(crate) fn owned(&self, call: impl FnOnce(*mut *mut u8, *mut usize) -> c_int) -> io::Result<Vec<u8>> {
        let (mut data, mut len) = (ptr::null_mut(), 0usize);
        checked(call(&mut data, &mut len))?;
        let bytes = match data.is_null() {
            true => Vec::new(),
            false => unsafe { std::slice::from_raw_parts(data, len) }.to_vec(),
        };
        unsafe { ely_free(self.ctx, data.cast()) };
        Ok(bytes)
    }

    pub fn text(&self, doc: &Open, number: usize) -> io::Result<Vec<u8>> {
        let number = page(number)?;
        self.owned(|data, len| unsafe { ely_text(self.ctx, doc.0, number, data, len) })
    }

    pub fn outline(&self, doc: &Open) -> io::Result<Vec<u8>> {
        self.owned(|data, len| unsafe { ely_outline(self.ctx, doc.0, data, len) })
    }

    /// Each hit as its quad count, then its quads.
    pub fn search(&self, doc: &Open, number: usize, needle: &str) -> io::Result<Vec<u8>> {
        let (needle, number) = (text(needle)?, page(number)?);
        self.owned(|data, len| unsafe { ely_search(self.ctx, doc.0, number, needle.as_ptr(), data, len) })
    }
}
