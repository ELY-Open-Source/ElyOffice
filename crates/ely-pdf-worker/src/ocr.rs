//! The shim's OCR calls (PDF-08).

use std::{
    ffi::{c_char, c_int},
    io,
};

use crate::ffi::{Context, Document, Mupdf, Open, checked, page, text};

unsafe extern "C" {
    fn ely_ocr_page(
        ctx: *mut Context,
        doc: *mut Document,
        page: c_int,
        lang: *const c_char,
        datadir: *const c_char,
        dpi: f32,
        data: *mut *mut u8,
        len: *mut usize,
    ) -> c_int;
    fn ely_text_layer(ctx: *mut Context, doc: *mut Document, json: *const c_char, replace: c_int) -> c_int;
}

impl Mupdf {
    pub fn ocr_page(&self, doc: &Open, at: usize, lang: &str, datadir: &str, dpi: f32) -> io::Result<Vec<u8>> {
        let (at, lang, datadir) = (page(at)?, text(lang)?, text(datadir)?);
        self.owned(|data, len| unsafe { ely_ocr_page(self.ctx, doc.0, at, lang.as_ptr(), datadir.as_ptr(), dpi, data, len) })
    }

    pub fn text_layer(&self, doc: &Open, json: &str, replace: bool) -> io::Result<()> {
        let json = text(json)?;
        checked(unsafe { ely_text_layer(self.ctx, doc.0, json.as_ptr(), c_int::from(replace)) }).map(|_| ())
    }
}
