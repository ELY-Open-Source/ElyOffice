//! The shim's annotation and form calls (PDF-05, PDF-06).

use std::{
    ffi::{c_char, c_int},
    io,
};

use crate::ffi::{Context, Document, Mupdf, Open, checked, page, text};

unsafe extern "C" {
    fn ely_annots(ctx: *mut Context, doc: *mut Document, page: c_int, data: *mut *mut u8, len: *mut usize) -> c_int;
    fn ely_annotate(ctx: *mut Context, doc: *mut Document, json: *const c_char) -> c_int;
    fn ely_change_annot(ctx: *mut Context, doc: *mut Document, page: c_int, id: c_int, json: *const c_char) -> c_int;
    fn ely_delete_annot(ctx: *mut Context, doc: *mut Document, page: c_int, id: c_int, widget: c_int) -> c_int;
    fn ely_add_field(ctx: *mut Context, doc: *mut Document, page: c_int, json: *const c_char) -> c_int;
    fn ely_fill(ctx: *mut Context, doc: *mut Document, page: c_int, id: c_int, value: *const c_char) -> c_int;
    fn ely_field_names(ctx: *mut Context, doc: *mut Document, data: *mut *mut u8, len: *mut usize) -> c_int;
    fn ely_change_field(ctx: *mut Context, doc: *mut Document, page: c_int, id: c_int, json: *const c_char) -> c_int;
}

fn id(value: u64) -> io::Result<c_int> {
    c_int::try_from(value).map_err(|_| io::Error::other("an object number too large"))
}

impl Mupdf {
    pub fn annots(&self, doc: &Open, at: usize) -> io::Result<Vec<u8>> {
        let at = page(at)?;
        self.owned(|data, len| unsafe { ely_annots(self.ctx, doc.0, at, data, len) })
    }

    pub fn field_names(&self, doc: &Open) -> io::Result<Vec<u8>> {
        self.owned(|data, len| unsafe { ely_field_names(self.ctx, doc.0, data, len) })
    }

    pub fn annotate(&self, doc: &Open, json: &str) -> io::Result<()> {
        let json = text(json)?;
        checked(unsafe { ely_annotate(self.ctx, doc.0, json.as_ptr()) }).map(|_| ())
    }

    pub fn change_annot(&self, doc: &Open, at: usize, annot: u64, json: &str) -> io::Result<()> {
        let json = text(json)?;
        checked(unsafe { ely_change_annot(self.ctx, doc.0, page(at)?, id(annot)?, json.as_ptr()) }).map(|_| ())
    }

    pub fn delete_annot(&self, doc: &Open, at: usize, annot: u64, field: bool) -> io::Result<()> {
        checked(unsafe { ely_delete_annot(self.ctx, doc.0, page(at)?, id(annot)?, c_int::from(field)) }).map(|_| ())
    }

    pub fn add_field(&self, doc: &Open, at: usize, json: &str) -> io::Result<()> {
        let json = text(json)?;
        checked(unsafe { ely_add_field(self.ctx, doc.0, page(at)?, json.as_ptr()) }).map(|_| ())
    }

    pub fn fill(&self, doc: &Open, at: usize, field: u64, value: &str) -> io::Result<()> {
        let value = text(value)?;
        checked(unsafe { ely_fill(self.ctx, doc.0, page(at)?, id(field)?, value.as_ptr()) }).map(|_| ())
    }

    pub fn change_field(&self, doc: &Open, at: usize, field: u64, json: &str) -> io::Result<()> {
        let json = text(json)?;
        checked(unsafe { ely_change_field(self.ctx, doc.0, page(at)?, id(field)?, json.as_ptr()) }).map(|_| ())
    }
}
