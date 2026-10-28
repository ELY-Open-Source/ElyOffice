//! The shim's bookmark, link, property and attachment calls (PDF-07).

use std::{
    ffi::{c_char, c_int},
    io,
};

use crate::ffi::{Context, Document, Mupdf, Open, checked, page, text};

unsafe extern "C" {
    fn ely_outline_edit(ctx: *mut Context, doc: *mut Document, json: *const c_char) -> c_int;
    fn ely_links(ctx: *mut Context, doc: *mut Document, page: c_int, data: *mut *mut u8, len: *mut usize) -> c_int;
    fn ely_link_edit(ctx: *mut Context, doc: *mut Document, page: c_int, json: *const c_char) -> c_int;
    fn ely_metadata(ctx: *mut Context, doc: *mut Document, data: *mut *mut u8, len: *mut usize) -> c_int;
    fn ely_set_metadata(ctx: *mut Context, doc: *mut Document, json: *const c_char) -> c_int;
    fn ely_attachments(ctx: *mut Context, doc: *mut Document, data: *mut *mut u8, len: *mut usize) -> c_int;
    fn ely_attachment(ctx: *mut Context, doc: *mut Document, name: *const c_char, data: *mut *mut u8, len: *mut usize) -> c_int;
    fn ely_attach(ctx: *mut Context, doc: *mut Document, name: *const c_char, path: *const c_char) -> c_int;
    fn ely_detach(ctx: *mut Context, doc: *mut Document, name: *const c_char) -> c_int;
}

impl Mupdf {
    pub fn outline_edit(&self, doc: &Open, json: &str) -> io::Result<()> {
        let json = text(json)?;
        checked(unsafe { ely_outline_edit(self.ctx, doc.0, json.as_ptr()) }).map(|_| ())
    }

    pub fn links(&self, doc: &Open, at: usize) -> io::Result<Vec<u8>> {
        let at = page(at)?;
        self.owned(|data, len| unsafe { ely_links(self.ctx, doc.0, at, data, len) })
    }

    pub fn link_edit(&self, doc: &Open, at: usize, json: &str) -> io::Result<()> {
        let json = text(json)?;
        checked(unsafe { ely_link_edit(self.ctx, doc.0, page(at)?, json.as_ptr()) }).map(|_| ())
    }

    pub fn metadata(&self, doc: &Open) -> io::Result<Vec<u8>> {
        self.owned(|data, len| unsafe { ely_metadata(self.ctx, doc.0, data, len) })
    }

    pub fn set_metadata(&self, doc: &Open, json: &str) -> io::Result<()> {
        let json = text(json)?;
        checked(unsafe { ely_set_metadata(self.ctx, doc.0, json.as_ptr()) }).map(|_| ())
    }

    pub fn attachments(&self, doc: &Open) -> io::Result<Vec<u8>> {
        self.owned(|data, len| unsafe { ely_attachments(self.ctx, doc.0, data, len) })
    }

    pub fn attachment(&self, doc: &Open, name: &str) -> io::Result<Vec<u8>> {
        let name = text(name)?;
        self.owned(|data, len| unsafe { ely_attachment(self.ctx, doc.0, name.as_ptr(), data, len) })
    }

    pub fn attach(&self, doc: &Open, name: &str, path: &str) -> io::Result<()> {
        let (name, path) = (text(name)?, text(path)?);
        checked(unsafe { ely_attach(self.ctx, doc.0, name.as_ptr(), path.as_ptr()) }).map(|_| ())
    }

    pub fn detach(&self, doc: &Open, name: &str) -> io::Result<()> {
        let name = text(name)?;
        checked(unsafe { ely_detach(self.ctx, doc.0, name.as_ptr()) }).map(|_| ())
    }
}
