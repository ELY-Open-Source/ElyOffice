//! The shim's page edits, undo and saves.

use std::{
    ffi::{c_char, c_int},
    io,
    path::Path,
};

use crate::ffi::{Context, Document, Mupdf, Open, checked, page, text};

unsafe extern "C" {
    fn ely_journal(ctx: *mut Context, doc: *mut Document) -> c_int;
    fn ely_undo(ctx: *mut Context, doc: *mut Document, redo: c_int) -> c_int;
    fn ely_state(ctx: *mut Context, doc: *mut Document, at: *mut c_int, count: *mut c_int) -> c_int;
    fn ely_delete_pages(ctx: *mut Context, doc: *mut Document, pages: *const c_int, n: c_int) -> c_int;
    fn ely_move_pages(ctx: *mut Context, doc: *mut Document, pages: *const c_int, n: c_int, to: c_int) -> c_int;
    fn ely_rotate_pages(ctx: *mut Context, doc: *mut Document, pages: *const c_int, n: c_int, by: c_int) -> c_int;
    fn ely_crop_pages(
        ctx: *mut Context,
        doc: *mut Document,
        pages: *const c_int,
        n: c_int,
        margins: *const f32,
    ) -> c_int;
    fn ely_insert_blank(ctx: *mut Context, doc: *mut Document, at: c_int, w: f32, h: f32) -> c_int;
    fn ely_insert_file(
        ctx: *mut Context,
        doc: *mut Document,
        at: c_int,
        path: *const c_char,
        count: *mut c_int,
    ) -> c_int;
    fn ely_save(
        ctx: *mut Context,
        doc: *mut Document,
        raw: *const c_char,
        out: *const c_char,
        password: *const c_char,
    ) -> c_int;
    fn ely_extract(
        ctx: *mut Context,
        doc: *mut Document,
        pages: *const c_int,
        n: c_int,
        raw: *const c_char,
        out: *const c_char,
        password: *const c_char,
    ) -> c_int;
}

/// Page numbers as the shim takes them.
fn numbers(pages: &[usize]) -> io::Result<(Vec<c_int>, c_int)> {
    let pages = pages.iter().map(|at| page(*at)).collect::<io::Result<Vec<_>>>()?;
    let n = c_int::try_from(pages.len()).map_err(|_| io::Error::other("too many pages"))?;
    Ok((pages, n))
}

/// The scratch copy a save writes first, removed after.
pub(crate) fn raw(out: &Path) -> std::path::PathBuf {
    let mut name = out.as_os_str().to_owned();
    name.push(".raw");
    name.into()
}

pub(crate) fn utf8(path: &Path) -> io::Result<&str> {
    path.to_str().ok_or_else(|| io::Error::other("a path that is not UTF-8"))
}

impl Mupdf {
    /// Starts the undo history; every edit becomes a step.
    pub fn journal(&self, doc: &Open) -> io::Result<()> {
        checked(unsafe { ely_journal(self.ctx, doc.0) }).map(|_| ())
    }

    /// MuPDF's undo position and step count.
    pub fn state(&self, doc: &Open) -> io::Result<(usize, usize)> {
        let (mut at, mut count) = (0, 0);
        checked(unsafe { ely_state(self.ctx, doc.0, &mut at, &mut count) })?;
        let size = |n: c_int| usize::try_from(n).map_err(io::Error::other);
        Ok((size(at)?, size(count)?))
    }

    pub fn undo(&self, doc: &Open, redo: bool) -> io::Result<()> {
        checked(unsafe { ely_undo(self.ctx, doc.0, c_int::from(redo)) }).map(|_| ())
    }

    pub fn delete_pages(&self, doc: &Open, pages: &[usize]) -> io::Result<()> {
        let (pages, n) = numbers(pages)?;
        checked(unsafe { ely_delete_pages(self.ctx, doc.0, pages.as_ptr(), n) }).map(|_| ())
    }

    pub fn move_pages(&self, doc: &Open, pages: &[usize], to: usize) -> io::Result<()> {
        let (pages, n) = numbers(pages)?;
        checked(unsafe { ely_move_pages(self.ctx, doc.0, pages.as_ptr(), n, page(to)?) }).map(|_| ())
    }

    pub fn rotate_pages(&self, doc: &Open, pages: &[usize], by: i32) -> io::Result<()> {
        let (pages, n) = numbers(pages)?;
        checked(unsafe { ely_rotate_pages(self.ctx, doc.0, pages.as_ptr(), n, by) }).map(|_| ())
    }

    pub fn crop_pages(&self, doc: &Open, pages: &[usize], margins: [f32; 4]) -> io::Result<()> {
        let (pages, n) = numbers(pages)?;
        checked(unsafe { ely_crop_pages(self.ctx, doc.0, pages.as_ptr(), n, margins.as_ptr()) }).map(|_| ())
    }

    pub fn insert_blank(&self, doc: &Open, at: usize, (w, h): (f32, f32)) -> io::Result<()> {
        checked(unsafe { ely_insert_blank(self.ctx, doc.0, page(at)?, w, h) }).map(|_| ())
    }

    /// Inserts the pages of `path`; how many came.
    pub fn insert_file(&self, doc: &Open, at: usize, path: &str) -> io::Result<usize> {
        let (path, mut count) = (text(path)?, 0);
        checked(unsafe { ely_insert_file(self.ctx, doc.0, page(at)?, path.as_ptr(), &mut count) })?;
        usize::try_from(count).map_err(io::Error::other)
    }

    /// Writes a compacted copy to `out`.
    pub fn save(&self, doc: &Open, out: &Path, password: &str) -> io::Result<()> {
        let scratch = raw(out);
        let (raw_c, out_c, password) = (text(utf8(&scratch)?)?, text(utf8(out)?)?, text(password)?);
        let saved = checked(unsafe { ely_save(self.ctx, doc.0, raw_c.as_ptr(), out_c.as_ptr(), password.as_ptr()) });
        clean(&scratch);
        saved.map(|_| ())
    }

    /// Writes only `pages` to `out`; the document stays.
    pub fn extract(&self, doc: &Open, pages: &[usize], out: &Path, password: &str) -> io::Result<()> {
        let (pages, n) = numbers(pages)?;
        let scratch = raw(out);
        let (raw_c, out_c, password) = (text(utf8(&scratch)?)?, text(utf8(out)?)?, text(password)?);
        let done = checked(unsafe {
            ely_extract(self.ctx, doc.0, pages.as_ptr(), n, raw_c.as_ptr(), out_c.as_ptr(), password.as_ptr())
        });
        clean(&scratch);
        done.map(|_| ())
    }
}

/// Removes a scratch copy; one never written is fine.
pub(crate) fn clean(scratch: &Path) {
    match std::fs::remove_file(scratch) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => log::warn!("pdf-worker: the scratch copy stays: {error}"),
    }
}
