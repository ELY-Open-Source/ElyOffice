//! Permissions, owner access and protected copies (PDF-11).

use std::{
    ffi::{c_char, c_int},
    io,
    path::Path,
};

use ely_pdf_engine::proto::{may, tag};

use crate::edit::{clean, raw, utf8};
use crate::ffi::{Context, Document, Mupdf, Open, checked, text};

unsafe extern "C" {
    fn ely_permissions(ctx: *mut Context, doc: *mut Document) -> c_int;
    fn ely_protect(
        ctx: *mut Context,
        doc: *mut Document,
        raw: *const c_char,
        out: *const c_char,
        password: *const c_char,
        protect: c_int,
        user: *const c_char,
        owner: *const c_char,
        may: c_int,
    ) -> c_int;
}

/// What each request needs the document to allow.
pub fn needs(kind: u8) -> u32 {
    match kind {
        tag::DELETE | tag::MOVE | tag::ROTATE | tag::BLANK | tag::INSERT | tag::OUTLINE_EDIT => may::ASSEMBLE,
        tag::CROP | tag::PUT_TEXT | tag::PLACE | tag::ADD_IMAGE | tag::TEXT_LAYER | tag::LINK_EDIT => may::EDIT,
        tag::SET_METADATA | tag::ATTACH | tag::DETACH | tag::REDACT | tag::CLEAN => may::EDIT,
        tag::ANNOTATE | tag::CHANGE_ANNOT => may::ANNOTATE,
        tag::ADD_FIELD | tag::CHANGE_FIELD => may::EDIT | may::ANNOTATE,
        tag::FILL => may::FILL,
        tag::EXTRACT => may::COPY,
        tag::PROTECT => may::OWNER,
        _ => 0,
    }
}

/// The bits `needed` lacks in `has`, as an error.
pub fn refuse(needed: u32, has: u32) -> io::Result<()> {
    let missing = needed & !has;
    match may::NAMES.iter().find(|(bit, _)| missing & bit != 0) {
        Some((_, name)) => Err(io::Error::other(format!("{}{name}", may::REFUSED))),
        None => Ok(()),
    }
}

impl Mupdf {
    /// Ely's permission bits; `owner` adds full access.
    pub fn permissions(&self, doc: &Open, owner: bool) -> io::Result<u32> {
        let bits = checked(unsafe { ely_permissions(self.ctx, doc.0) })? as u32;
        Ok(if owner { bits | may::OWNER } else { bits })
    }

    /// Writes `out` protected, or unprotected without `lock`.
    pub fn protect(&self, doc: &Open, out: &Path, password: &str, lock: Option<(&str, &str, u32)>) -> io::Result<()> {
        let (user, owner, bits) = lock.unwrap_or(("", "", 0));
        let scratch = raw(out);
        let (raw_c, out_c, password) = (text(utf8(&scratch)?)?, text(utf8(out)?)?, text(password)?);
        let (user, owner) = (text(user)?, text(owner)?);
        let bits = c_int::try_from(bits).map_err(io::Error::other)?;
        let done = checked(unsafe {
            ely_protect(
                self.ctx,
                doc.0,
                raw_c.as_ptr(),
                out_c.as_ptr(),
                password.as_ptr(),
                c_int::from(lock.is_some()),
                user.as_ptr(),
                owner.as_ptr(),
                bits,
            )
        });
        clean(&scratch);
        if done.is_err() {
            clean(out);
        }
        done.map(|_| ())
    }
}

impl crate::Worker {
    /// What document `id` allows now.
    pub(crate) fn allowed(&self, id: u64) -> io::Result<u32> {
        let doc = self.record(id)?;
        self.mupdf.permissions(&doc.open, doc.owner)
    }

    /// Refuses `kind` on `id` when the document forbids it.
    pub(crate) fn guard(&self, id: u64, kind: u8) -> io::Result<()> {
        match needs(kind) {
            0 => Ok(()),
            needed => refuse(needed, self.allowed(id)?),
        }
    }

    pub(crate) fn secure(&mut self, kind: u8, id: u64, ask: &mut ely_ipc::Unpack) -> io::Result<Vec<u8>> {
        match kind {
            tag::PERMISSIONS => Ok(ely_ipc::Pack::new().u64(u64::from(self.allowed(id)?)).bytes()),
            tag::OWNER => {
                let password = ask.str()?;
                let doc = self.docs.get_mut(&id).ok_or_else(|| io::Error::other(format!("no document {id}")))?;
                let access = self.mupdf.unlock(&doc.open, &password)?;
                let owner = access & 4 != 0;
                match owner {
                    true => (doc.password, doc.owner) = (password, true),
                    // A refused try drops MuPDF's keys; the old ones return.
                    false => _ = self.mupdf.unlock(&doc.open, &doc.password)?,
                }
                log::info!("pdf-worker: owner password for document {id}: {}", if owner { "accepted" } else { "refused" });
                Ok(ely_ipc::Pack::new().u64(u64::from(owner)).bytes())
            }
            tag::PROTECT => {
                let (out, lock) = (ask.str()?, ask.u64()? != 0);
                let (user, owner, bits) = (ask.str()?, ask.str()?, ask.u64()? as u32);
                let doc = self.record(id)?;
                let lock = lock.then_some((user.as_str(), owner.as_str(), bits));
                self.mupdf.protect(&doc.open, out.as_ref(), &doc.password, lock)?;
                log::info!("pdf-worker: wrote a {} copy of document {id}", if lock.is_some() { "protected" } else { "unprotected" });
                Ok(Vec::new())
            }
            other => Err(io::Error::other(format!("no request {other}"))),
        }
    }
}
