//! Page images, text, optimized and repaired copies (PDF-13, PDF-14).

use std::{
    ffi::{CStr, c_char, c_int},
    io,
    path::{Path, PathBuf},
};

use ely_ipc::{Pack, Unpack};
use ely_pdf_engine::proto::{may, tag};

use crate::edit::{clean, raw, utf8};
use crate::ffi::{Context, Document, Mupdf, Open, checked, page, text};
use crate::secure::refuse;

unsafe extern "C" {
    fn ely_export_image(ctx: *mut Context, doc: *mut Document, number: c_int, dpi: f32, path: *const c_char) -> c_int;
    fn ely_export_text(ctx: *mut Context, doc: *mut Document, path: *const c_char) -> c_int;
    fn ely_optimize(
        ctx: *mut Context,
        doc: *mut Document,
        raw: *const c_char,
        out: *const c_char,
        password: *const c_char,
        compress: c_int,
        from: c_int,
        to: c_int,
    ) -> c_int;
    fn ely_repaired(ctx: *mut Context, doc: *mut Document) -> c_int;
    fn ely_write_repaired(ctx: *mut Context, doc: *mut Document, path: *const c_char) -> c_int;
    fn ely_open_warnings() -> *const c_char;
    fn ely_scan(ctx: *mut Context, doc: *mut Document) -> c_int;
    fn ely_watch_begin(ctx: *mut Context);
    fn ely_watch_end(ctx: *mut Context) -> c_int;
    fn ely_check_pictures(ctx: *mut Context, doc: *mut Document) -> c_int;
    fn ely_print_copy(ctx: *mut Context, doc: *mut Document, raw: *const c_char, out: *const c_char, password: *const c_char) -> c_int;
    fn ely_html_pdf(ctx: *mut Context, dir: *const c_char, page: *const c_char, out: *const c_char) -> c_int;
}

/// What MuPDF warned while opening the last file.
pub fn open_warnings() -> String {
    unsafe { CStr::from_ptr(ely_open_warnings()) }.to_string_lossy().into_owned()
}

impl Mupdf {
    fn optimize(&self, doc: &Open, out: &Path, password: &str, compress: bool, (from, to): (u32, u32)) -> io::Result<()> {
        let scratch = raw(out);
        let (raw_c, out_c, password) = (text(utf8(&scratch)?)?, text(utf8(out)?)?, text(password)?);
        let (from, to) = (c_int::try_from(from).map_err(io::Error::other)?, c_int::try_from(to).map_err(io::Error::other)?);
        let done = checked(unsafe {
            ely_optimize(self.ctx, doc.0, raw_c.as_ptr(), out_c.as_ptr(), password.as_ptr(), c_int::from(compress), from, to)
        });
        clean(&scratch);
        if done.is_err() {
            clean(out);
        }
        done.map(|_| ())
    }

    /// `page` in `dir` set as a PDF at `out`.
    pub fn html_pdf(&self, dir: &str, page: &str, out: &str) -> io::Result<()> {
        let (dir_c, page_c, out_c) = (text(dir)?, text(page)?, text(out)?);
        let done = checked(unsafe { ely_html_pdf(self.ctx, dir_c.as_ptr(), page_c.as_ptr(), out_c.as_ptr()) });
        if done.is_err() {
            clean(out.as_ref());
        }
        done.map(|_| log::info!("pdf-worker: set an HTML page as a PDF"))
    }

    pub fn repaired(&self, doc: &Open) -> io::Result<bool> {
        checked(unsafe { ely_repaired(self.ctx, doc.0) }).map(|repaired| repaired != 0)
    }
}

impl crate::Worker {
    /// MuPDF warned on reading: the file is damaged.
    fn note_damage(&mut self, id: u64) -> io::Result<()> {
        let doc = self.docs.get_mut(&id).ok_or_else(|| io::Error::other(format!("no document {id}")))?;
        let report = doc.repair.get_or_insert_with(String::new);
        report.push_str(&open_warnings());
        log::warn!("pdf-worker: document {id} is damaged");
        Ok(())
    }

    /// A text read; damage it meets waits for a draw.
    pub(crate) fn read(&mut self, id: u64, read: impl FnOnce(&Mupdf, &Open) -> io::Result<Vec<u8>>) -> io::Result<Vec<u8>> {
        let doc = self.doc(id)?;
        unsafe { ely_watch_begin(self.mupdf.ctx) };
        let answer = read(&self.mupdf, doc);
        if unsafe { ely_watch_end(self.mupdf.ctx) } != 0 {
            self.note_damage(id)?;
            self.docs.get_mut(&id).expect("just read").found = true;
        }
        answer
    }

    /// Takes damage a read found; the caller tells it.
    fn found(&mut self, id: u64) -> io::Result<bool> {
        let doc = self.docs.get_mut(&id).ok_or_else(|| io::Error::other(format!("no document {id}")))?;
        Ok(std::mem::take(&mut doc.found))
    }

    /// A tile, and whether drawing it met damage.
    pub(crate) fn drawn(&mut self, id: u64, page: usize, view: (f32, i32), tile: [i32; 4]) -> io::Result<Vec<u8>> {
        unsafe { ely_watch_begin(self.mupdf.ctx) };
        let pixels = self.mupdf.render(self.doc(id)?, page, view, tile);
        let warned = unsafe { ely_watch_end(self.mupdf.ctx) } != 0;
        if warned {
            self.note_damage(id)?;
        }
        let pixels = pixels?;
        let damaged = warned | self.found(id)?;
        Ok(Pack::new().u64(u64::from(damaged)).blob(&pixels).bytes())
    }

    /// Every picture decoded before an overwrite; true if any failed.
    pub(crate) fn check_pictures(&mut self, id: u64) -> io::Result<Vec<u8>> {
        let failed = checked(unsafe { ely_check_pictures(self.mupdf.ctx, self.doc(id)?.0) })? != 0;
        if failed {
            self.note_damage(id)?;
        }
        let failed = failed | self.found(id)?;
        Ok(Pack::new().u64(u64::from(failed)).bytes())
    }

    /// A repaired file gives way to a whole copy.
    pub(crate) fn repair(&mut self, id: u64) -> io::Result<()> {
        let doc = self.docs.get_mut(&id).ok_or_else(|| io::Error::other(format!("no document {id}")))?;
        if doc.repair.is_some() {
            return Ok(());
        }
        // Late repairs and broken streams show only on reading.
        let warned = checked(unsafe { ely_scan(self.mupdf.ctx, doc.open.0) })? != 0;
        doc.warnings.push_str(&open_warnings());
        if !warned && !self.mupdf.repaired(&doc.open)? {
            return Ok(());
        }
        let copy: PathBuf = std::env::temp_dir().join(format!("ely-pdf-{}-{id}.pdf", std::process::id()));
        let path = text(utf8(&copy)?)?;
        // Any failure here removes the copy and what was opened.
        let mupdf = &self.mupdf;
        let fresh = (|| {
            checked(unsafe { ely_write_repaired(mupdf.ctx, doc.open.0, path.as_ptr()) })?;
            let fresh = mupdf.open(utf8(&copy)?)?;
            let ready = (|| {
                if mupdf.locked(&fresh)? && mupdf.unlock(&fresh, &doc.password)? == 0 {
                    return Err(io::Error::other("the repaired copy refused the password"));
                }
                mupdf.journal(&fresh)
            })();
            match ready {
                Ok(()) => Ok(fresh),
                Err(error) => {
                    mupdf.close(fresh);
                    Err(error)
                }
            }
        })();
        let fresh = match fresh {
            Ok(fresh) => fresh,
            Err(error) => {
                clean(&copy);
                return Err(error);
            }
        };
        let broken = std::mem::replace(&mut doc.open, fresh);
        self.mupdf.close(broken);
        doc.repair = Some(std::mem::take(&mut doc.warnings));
        doc.copy = Some(copy);
        log::info!("pdf-worker: document {id} was damaged; a repaired copy stands in");
        Ok(())
    }

    pub(crate) fn output(&mut self, kind: u8, id: u64, ask: &mut Unpack) -> io::Result<Vec<u8>> {
        let has = self.allowed(id)?;
        let doc = self.record(id)?;
        // What a read found is no whole export.
        if matches!(kind, tag::EXPORT_IMAGE | tag::EXPORT_TEXT) && doc.found {
            refuse(may::COPY, has)?;
            let report = doc.repair.as_deref().expect("noted with the damage");
            return Err(io::Error::other(format!("the document did not read whole: {report}")));
        }
        match kind {
            tag::EXPORT_IMAGE => {
                refuse(may::COPY, has)?;
                let (at, dpi, path) = (ask.u64()? as usize, f32::from_bits(ask.u64()? as u32), ask.str()?);
                let path_c = text(&path)?;
                checked(unsafe { ely_export_image(self.mupdf.ctx, doc.open.0, page(at)?, dpi, path_c.as_ptr()) })?;
            }
            tag::EXPORT_TEXT => {
                refuse(may::COPY, has)?;
                let path = text(&ask.str()?)?;
                checked(unsafe { ely_export_text(self.mupdf.ctx, doc.open.0, path.as_ptr()) })?;
            }
            tag::OPTIMIZE => {
                refuse(may::EDIT, has)?;
                let (out, compress) = (ask.str()?, ask.u64()? != 0);
                let (from, to) = (ask.u64()? as u32, ask.u64()? as u32);
                self.mupdf.optimize(&doc.open, out.as_ref(), &doc.password, compress, (from, to))?;
            }
            tag::PRINT_COPY => {
                refuse(may::PRINT, has)?;
                let out = ask.str()?;
                let scratch = raw(out.as_ref());
                let (raw_c, out_c, password) = (text(utf8(&scratch)?)?, text(&out)?, text(&doc.password)?);
                let done = checked(unsafe { ely_print_copy(self.mupdf.ctx, doc.open.0, raw_c.as_ptr(), out_c.as_ptr(), password.as_ptr()) });
                clean(&scratch);
                done?;
            }
            tag::REPAIR_REPORT => {
                return Ok(match &doc.repair {
                    Some(report) => Pack::new().u64(1).str(report).bytes(),
                    None => Pack::new().u64(0).bytes(),
                });
            }
            other => return Err(io::Error::other(format!("no request {other}"))),
        }
        log::info!("pdf-worker: request {kind} done for document {id}");
        Ok(Vec::new())
    }
}
