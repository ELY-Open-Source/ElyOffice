//! The shim's removal and cleaning calls (PDF-09, PDF-10).

use std::{
    ffi::{c_char, c_int},
    io,
    path::Path,
};

use crate::edit::{clean, raw, utf8};
use crate::ffi::{Context, Document, Mupdf, Open, text};

unsafe extern "C" {
    fn ely_redact(
        ctx: *mut Context,
        doc: *mut Document,
        json: *const c_char,
        raw: *const c_char,
        out: *const c_char,
        password: *const c_char,
        data: *mut *mut u8,
        len: *mut usize,
    ) -> c_int;
    fn ely_clean_scan(
        ctx: *mut Context,
        doc: *mut Document,
        raw: *const c_char,
        password: *const c_char,
        data: *mut *mut u8,
        len: *mut usize,
    ) -> c_int;
    fn ely_clean(
        ctx: *mut Context,
        doc: *mut Document,
        mask: c_int,
        raw: *const c_char,
        out: *const c_char,
        password: *const c_char,
        data: *mut *mut u8,
        len: *mut usize,
    ) -> c_int;
}

impl Mupdf {
    /// Writes `out` with `json`'s regions removed; the check.
    pub fn redact(&self, doc: &Open, json: &str, out: &Path, password: &str) -> io::Result<Vec<u8>> {
        let scratch = raw(out);
        let (json, raw_c, out_c, password) = (text(json)?, text(utf8(&scratch)?)?, text(utf8(out)?)?, text(password)?);
        let done = self.owned(|data, len| unsafe {
            ely_redact(self.ctx, doc.0, json.as_ptr(), raw_c.as_ptr(), out_c.as_ptr(), password.as_ptr(), data, len)
        });
        clean(&scratch);
        // A file that failed its check is not left behind.
        if done.is_err() {
            clean(out);
        }
        done
    }

    /// Counts on a copy written to `scratch`, then removed.
    pub fn clean_scan(&self, doc: &Open, scratch: &Path, password: &str) -> io::Result<Vec<u8>> {
        let (raw_c, password) = (text(utf8(scratch)?)?, text(password)?);
        let done = self.owned(|data, len| unsafe { ely_clean_scan(self.ctx, doc.0, raw_c.as_ptr(), password.as_ptr(), data, len) });
        clean(scratch);
        done
    }

    /// Writes `out` cleaned of `mask`; what is left.
    pub fn clean(&self, doc: &Open, mask: u32, out: &Path, password: &str) -> io::Result<Vec<u8>> {
        let mask = c_int::try_from(mask).map_err(io::Error::other)?;
        let scratch = raw(out);
        let (raw_c, out_c, password) = (text(utf8(&scratch)?)?, text(utf8(out)?)?, text(password)?);
        let done = self.owned(|data, len| unsafe {
            ely_clean(self.ctx, doc.0, mask, raw_c.as_ptr(), out_c.as_ptr(), password.as_ptr(), data, len)
        });
        clean(&scratch);
        if done.is_err() {
            clean(out);
        }
        done
    }
}
