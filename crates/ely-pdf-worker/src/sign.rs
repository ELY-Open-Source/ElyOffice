//! The shim's signature calls (PDF-12).

use std::{
    ffi::{c_char, c_int},
    io,
    path::Path,
};

use ely_pdf_engine::{Signing, proto::{may, tag}};

use crate::edit::{clean, utf8};
use crate::ffi::{Context, Document, Mupdf, Open, checked, page, text};
use crate::secure::refuse;

unsafe extern "C" {
    fn ely_signatures(ctx: *mut Context, doc: *mut Document, data: *mut *mut u8, len: *mut usize) -> c_int;
    fn ely_signed(ctx: *mut Context, doc: *mut Document) -> c_int;
    fn ely_sign(
        ctx: *mut Context,
        doc: *mut Document,
        raw: *const c_char,
        password: *const c_char,
        pfx: *const c_char,
        pfx_password: *const c_char,
        number: c_int,
        object: c_int,
        rect: *const f32,
        name: *const c_char,
        reason: *const c_char,
        location: *const c_char,
    ) -> c_int;
}

impl Mupdf {
    pub fn signatures(&self, doc: &Open) -> io::Result<Vec<u8>> {
        self.owned(|data, len| unsafe { ely_signatures(self.ctx, doc.0, data, len) })
    }

    /// Writes `out` signed; a failure leaves nothing.
    pub fn sign(&self, doc: &Open, out: &Path, password: &str, how: &Signing) -> io::Result<()> {
        let object = match how.object {
            Some(object) => c_int::try_from(object).map_err(io::Error::other)?,
            None => -1,
        };
        let (out_c, password) = (text(utf8(out)?)?, text(password)?);
        let (pfx, pfx_password, name) = (text(&how.pfx)?, text(&how.pfx_password)?, text(&how.name)?);
        let (reason, location) = (text(&how.reason)?, text(&how.location)?);
        let done = checked(unsafe {
            ely_sign(
                self.ctx,
                doc.0,
                out_c.as_ptr(),
                password.as_ptr(),
                pfx.as_ptr(),
                pfx_password.as_ptr(),
                page(how.page)?,
                object,
                how.rect.as_ptr(),
                name.as_ptr(),
                reason.as_ptr(),
                location.as_ptr(),
            )
        });
        if done.is_err() {
            clean(out);
        }
        done.map(|_| ())
    }
}

impl crate::Worker {
    pub(crate) fn signing(&mut self, kind: u8, id: u64, ask: &mut ely_ipc::Unpack) -> io::Result<Vec<u8>> {
        match kind {
            tag::SIGNATURES => {
                // A file's signatures, not the edited copy's.
                let path = ask.str()?;
                let password = self.record(id)?.password.clone();
                let file = self.mupdf.open(&path)?;
                let read = match self.mupdf.locked(&file) {
                    Ok(true) if self.mupdf.unlock(&file, &password)? == 0 => Err(io::Error::other("the file refused the document's password")),
                    Ok(_) => self.mupdf.signatures(&file),
                    Err(error) => Err(error),
                };
                self.mupdf.close(file);
                read
            }
            tag::SIGNED => {
                let count = checked(unsafe { ely_signed(self.mupdf.ctx, self.record(id)?.open.0) })?;
                Ok(ely_ipc::Pack::new().u64(count as u64).bytes())
            }
            tag::SIGN => {
                let out = ask.str()?;
                let (page, object) = (ask.u64()? as usize, ask.i64()?);
                let mut rect = [0f32; 4];
                for value in &mut rect {
                    *value = f32::from_bits(ask.u64()? as u32);
                }
                let (name, pfx, pfx_password) = (ask.str()?, ask.str()?, ask.str()?);
                let (reason, location) = (ask.str()?, ask.str()?);
                let object = u32::try_from(object).ok();
                // Filling a field, or adding one to the page.
                let needed = if object.is_some() { may::FILL } else { may::EDIT | may::ANNOTATE };
                refuse(needed, self.allowed(id)?)?;
                let how = Signing { page, object, rect, name, pfx, pfx_password, reason, location };
                let doc = self.record(id)?;
                self.mupdf.sign(&doc.open, out.as_ref(), &doc.password, &how)?;
                log::info!("pdf-worker: wrote a signed copy of document {id}");
                Ok(Vec::new())
            }
            other => Err(io::Error::other(format!("no request {other}"))),
        }
    }
}
