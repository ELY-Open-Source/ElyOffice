//! The PDF worker: MuPDF behind framed stdin and stdout.

use std::{
    collections::HashMap,
    io::{self, BufReader, BufWriter},
};

use ely_ipc::{Pack, Unpack, read_frame, write_frame};
use ely_pdf_engine::proto::{may, tag};

mod content;
mod edit;
mod ffi;
mod marks;
mod ocr;
mod output;
mod redact;
mod secure;
mod sign;
mod structure;

use ffi::{Mupdf, Open};

/// An open document, its password and undo steps.
struct Doc {
    open: Open,
    /// Kept in memory only, to reopen saved copies.
    password: String,
    /// Opened with the owner password, or unprotected.
    owner: bool,
    /// What MuPDF warned at open; kept as the repair report.
    warnings: String,
    repair: Option<String>,
    /// Damage a read met that no draw has told yet.
    found: bool,
    /// The repaired copy standing in; removed on close.
    copy: Option<std::path::PathBuf>,
    /// MuPDF's steps: an edit's id, or none for a save.
    steps: Vec<Option<u64>>,
    /// Steps done, as MuPDF counts them.
    at: usize,
}

impl Doc {
    /// Equal marks, equal documents.
    fn mark(&self) -> String {
        self.steps[..self.at].iter().rev().flatten().next().map_or_else(|| "0".into(), u64::to_string)
    }

    /// Matches MuPDF's steps; a new last step is `made`.
    fn sync(&mut self, mupdf: &Mupdf, made: Option<u64>) -> io::Result<()> {
        let (at, count) = mupdf.state(&self.open)?;
        if at > self.at {
            self.steps.truncate(self.at);
            self.steps.extend((self.at..at).map(|step| made.filter(|_| step + 1 == at)));
        }
        // Steps MuPDF keeps but no edit made are never redone.
        self.steps.resize(count, None);
        self.at = at;
        Ok(())
    }

    /// Undoes or redoes MuPDF steps until one edit is passed.
    fn step(&mut self, mupdf: &Mupdf, redo: bool) -> io::Result<String> {
        let able = match redo {
            true => self.steps[self.at..].iter().any(Option::is_some),
            false => self.steps[..self.at].iter().any(Option::is_some),
        };
        if !able {
            return Err(io::Error::other(format!("nothing to {}", if redo { "redo" } else { "undo" })));
        }
        loop {
            if let Err(error) = mupdf.undo(&self.open, redo) {
                // MuPDF may have moved part way; follow it.
                self.at = mupdf.state(&self.open)?.0;
                return Err(error);
            }
            let passed = match redo {
                true => {
                    self.at += 1;
                    self.steps[self.at - 1]
                }
                false => {
                    self.at -= 1;
                    self.steps[self.at]
                }
            };
            if passed.is_some() {
                return Ok(self.mark());
            }
        }
    }
}

struct Worker {
    mupdf: Mupdf,
    docs: HashMap<u64, Doc>,
    next: u64,
    /// Edits made so far; each step's id.
    edits: u64,
}

/// Sorted distinct page numbers.
fn pages(ask: &mut Unpack) -> io::Result<Vec<usize>> {
    let count = ask.u64()? as usize;
    (0..count).map(|_| Ok(ask.u64()? as usize)).collect()
}

impl Worker {
    fn doc(&self, id: u64) -> io::Result<&Open> {
        Ok(&self.record(id)?.open)
    }

    fn record(&self, id: u64) -> io::Result<&Doc> {
        self.docs
            .get(&id)
            .ok_or_else(|| io::Error::other(format!("no document {id}")))
    }

    /// Runs one edit on `id`; it becomes the next step.
    fn edited(&mut self, id: u64, edit: impl FnOnce(&Mupdf, &Open) -> io::Result<()>) -> io::Result<String> {
        let doc = self.docs.get_mut(&id).ok_or_else(|| io::Error::other(format!("no document {id}")))?;
        if let Err(error) = edit(&self.mupdf, &doc.open) {
            // A failed edit may have dropped MuPDF's redo steps.
            doc.sync(&self.mupdf, None)?;
            return Err(error);
        }
        self.edits += 1;
        doc.sync(&self.mupdf, Some(self.edits))?;
        Ok(doc.mark())
    }

    /// Writes or previews text; a missing glyph changes nothing.
    fn put_text(&mut self, id: u64, ask: &mut Unpack) -> io::Result<Vec<u8>> {
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
        let writing = content::Writing { block, expected, text, font, size, rgb, bx };
        let doc = self.docs.get_mut(&id).ok_or_else(|| io::Error::other(format!("no document {id}")))?;
        let wrote = self.mupdf.put_text(&doc.open, at, &writing, &doc.password, scale);
        if matches!(wrote, Ok(content::Wrote::Done)) {
            self.edits += 1;
        }
        let made = matches!(wrote, Ok(content::Wrote::Done)).then_some(self.edits);
        doc.sync(&self.mupdf, made)?;
        Ok(match wrote? {
            content::Wrote::Done => Pack::new().u64(0).str(&doc.mark()).bytes(),
            content::Wrote::Missing(c) => Pack::new().u64(2).u64(u64::from(c)).bytes(),
            content::Wrote::Unwritable => Pack::new().u64(3).bytes(),
            content::Wrote::Preview(pixels, (w, h), grew) => {
                Pack::new().u64(1).u64(u64::from(w)).u64(u64::from(h)).u64(u64::from(grew)).blob(&pixels).bytes()
            }
        })
    }

    fn edit(&mut self, kind: u8, ask: &mut Unpack) -> io::Result<Vec<u8>> {
        let id = ask.u64()?;
        self.guard(id, kind)?;
        let mark = match kind {
            tag::HISTORY => self.record(id)?.mark(),
            tag::UNDO => {
                let redo = ask.u64()? != 0;
                let doc = self.docs.get_mut(&id).ok_or_else(|| io::Error::other(format!("no document {id}")))?;
                doc.step(&self.mupdf, redo)?
            }
            tag::DELETE => {
                let pages = pages(ask)?;
                self.edited(id, |mupdf, doc| mupdf.delete_pages(doc, &pages))?
            }
            tag::MOVE => {
                let (pages, to) = (pages(ask)?, ask.u64()? as usize);
                self.edited(id, |mupdf, doc| mupdf.move_pages(doc, &pages, to))?
            }
            tag::ROTATE => {
                let (pages, by) = (pages(ask)?, i32::try_from(ask.i64()?).map_err(io::Error::other)?);
                self.edited(id, |mupdf, doc| mupdf.rotate_pages(doc, &pages, by))?
            }
            tag::CROP => {
                let pages = pages(ask)?;
                let mut margins = [0f32; 4];
                for side in &mut margins {
                    *side = f32::from_bits(ask.u64()? as u32);
                }
                self.edited(id, |mupdf, doc| mupdf.crop_pages(doc, &pages, margins))?
            }
            tag::BLANK => {
                let at = ask.u64()? as usize;
                let size = (f32::from_bits(ask.u64()? as u32), f32::from_bits(ask.u64()? as u32));
                self.edited(id, |mupdf, doc| mupdf.insert_blank(doc, at, size))?
            }
            tag::INSERT => {
                let (at, path, mut count) = (ask.u64()? as usize, ask.str()?, 0);
                let mark = self.edited(id, |mupdf, doc| {
                    count = mupdf.insert_file(doc, at, &path)?;
                    Ok(())
                })?;
                return Ok(Pack::new().u64(count as u64).str(&mark).bytes());
            }
            tag::EXTRACT => {
                let (pages, out) = (pages(ask)?, ask.str()?);
                let doc = self.docs.get_mut(&id).ok_or_else(|| io::Error::other(format!("no document {id}")))?;
                let done = self.mupdf.extract(&doc.open, &pages, out.as_ref(), &doc.password);
                doc.sync(&self.mupdf, None)?;
                done?;
                return Ok(Vec::new());
            }
            tag::TEXT_BLOCKS => return self.mupdf.text_blocks(&self.record(id)?.open, ask.u64()? as usize),
            tag::OBJECTS => return self.mupdf.objects(&self.record(id)?.open, ask.u64()? as usize),
            tag::PUT_TEXT => return self.put_text(id, ask),
            tag::PLACE => {
                let at = ask.u64()? as usize;
                let (index, mut t) = (ask.u64()? as usize, [0f32; 6]);
                for value in &mut t {
                    *value = f32::from_bits(ask.u64()? as u32);
                }
                let (drop, swap) = (ask.u64()? != 0, ask.str()?);
                let swap = Some(swap.as_str()).filter(|path| !path.is_empty());
                self.edited(id, |mupdf, doc| mupdf.place(doc, at, index, t, drop, swap))?
            }
            tag::ADD_IMAGE => {
                let (at, path, mut rect) = (ask.u64()? as usize, ask.str()?, [0f32; 4]);
                for value in &mut rect {
                    *value = f32::from_bits(ask.u64()? as u32);
                }
                self.edited(id, |mupdf, doc| mupdf.add_image(doc, at, &path, rect))?
            }
            tag::ANNOTS => return self.mupdf.annots(&self.record(id)?.open, ask.u64()? as usize),
            tag::OCR_PAGE => {
                let (at, lang, datadir) = (ask.u64()? as usize, ask.str()?, ask.str()?);
                let dpi = f32::from_bits(ask.u64()? as u32);
                let lines = self.mupdf.ocr_page(&self.record(id)?.open, at, &lang, &datadir, dpi)?;
                log::info!("pdf-worker: OCR read page {at} of document {id}");
                return Ok(lines);
            }
            tag::TEXT_LAYER => {
                let (json, replace) = (ask.str()?, ask.u64()? != 0);
                self.edited(id, |mupdf, doc| mupdf.text_layer(doc, &json, replace))?
            }
            tag::OUTLINE_EDIT => {
                let json = ask.str()?;
                self.edited(id, |mupdf, doc| mupdf.outline_edit(doc, &json))?
            }
            tag::LINKS => return self.mupdf.links(&self.record(id)?.open, ask.u64()? as usize),
            tag::LINK_EDIT => {
                let (at, json) = (ask.u64()? as usize, ask.str()?);
                self.edited(id, |mupdf, doc| mupdf.link_edit(doc, at, &json))?
            }
            tag::METADATA => return self.mupdf.metadata(&self.record(id)?.open),
            tag::SET_METADATA => {
                let json = ask.str()?;
                self.edited(id, |mupdf, doc| mupdf.set_metadata(doc, &json))?
            }
            tag::ATTACHMENTS => return self.mupdf.attachments(&self.record(id)?.open),
            tag::ATTACHMENT => return self.mupdf.attachment(&self.record(id)?.open, &ask.str()?),
            tag::ATTACH => {
                let (name, path) = (ask.str()?, ask.str()?);
                self.edited(id, |mupdf, doc| mupdf.attach(doc, &name, &path))?
            }
            tag::DETACH => {
                let name = ask.str()?;
                self.edited(id, |mupdf, doc| mupdf.detach(doc, &name))?
            }
            tag::REDACT => {
                let (json, out) = (ask.str()?, ask.str()?);
                let record = self.record(id)?;
                let report = self.mupdf.redact(&record.open, &json, out.as_ref(), &record.password)?;
                log::info!("pdf-worker: wrote a redacted copy of document {id}");
                return Ok(report);
            }
            tag::CLEAN_SCAN => {
                let scratch = ask.str()?;
                let record = self.record(id)?;
                return self.mupdf.clean_scan(&record.open, scratch.as_ref(), &record.password);
            }
            tag::CLEAN => {
                let (mask, out) = (ask.u64()? as u32, ask.str()?);
                let record = self.record(id)?;
                let left = self.mupdf.clean(&record.open, mask, out.as_ref(), &record.password)?;
                log::info!("pdf-worker: wrote a cleaned copy of document {id}");
                return Ok(left);
            }
            tag::FIELD_NAMES => return self.mupdf.field_names(&self.record(id)?.open),
            tag::ANNOTATE => {
                let json = ask.str()?;
                self.edited(id, |mupdf, doc| mupdf.annotate(doc, &json))?
            }
            tag::CHANGE_ANNOT | tag::CHANGE_FIELD | tag::FILL | tag::DELETE_ANNOT => {
                let (at, object, value) = (ask.u64()? as usize, ask.u64()?, ask.str()?);
                if kind == tag::DELETE_ANNOT {
                    let needed = if value == "field" { may::EDIT | may::ANNOTATE } else { may::ANNOTATE };
                    secure::refuse(needed, self.allowed(id)?)?;
                }
                self.edited(id, |mupdf, doc| match kind {
                    tag::CHANGE_ANNOT => mupdf.change_annot(doc, at, object, &value),
                    tag::CHANGE_FIELD => mupdf.change_field(doc, at, object, &value),
                    tag::FILL => mupdf.fill(doc, at, object, &value),
                    _ => match value.as_str() {
                        "field" | "annot" => mupdf.delete_annot(doc, at, object, value == "field"),
                        other => Err(io::Error::other(format!("no object kind {other}"))),
                    },
                })?
            }
            tag::ADD_FIELD => {
                let (at, json) = (ask.u64()? as usize, ask.str()?);
                self.edited(id, |mupdf, doc| mupdf.add_field(doc, at, &json))?
            }
            tag::SAVE => {
                let out = ask.str()?;
                let doc = self.docs.get_mut(&id).ok_or_else(|| io::Error::other(format!("no document {id}")))?;
                let done = self.mupdf.save(&doc.open, out.as_ref(), &doc.password);
                doc.sync(&self.mupdf, None)?;
                done?;
                log::info!("pdf-worker: saved document {id}");
                return Ok(Vec::new());
            }
            tag::PERMISSIONS | tag::OWNER | tag::PROTECT => return self.secure(kind, id, ask),
            tag::SIGNATURES | tag::SIGN | tag::SIGNED => return self.signing(kind, id, ask),
            tag::CHECK_PICTURES => return self.check_pictures(id),
            tag::EXPORT_IMAGE | tag::EXPORT_TEXT | tag::OPTIMIZE | tag::PRINT_COPY | tag::REPAIR_REPORT => return self.output(kind, id, ask),
            other => return Err(io::Error::other(format!("no request {other}"))),
        };
        Ok(Pack::new().str(&mark).bytes())
    }

    fn answer(&mut self, kind: u8, payload: &[u8]) -> io::Result<Vec<u8>> {
        // A document request names the state it was made for.
        let stamped;
        let payload = match kind {
            tag::OPEN | tag::HELLO | tag::HTML_PDF => payload,
            _ => {
                stamped = self.current(payload)?;
                &stamped
            }
        };
        let mut ask = Unpack::new(payload);
        match kind {
            tag::OPEN => {
                let open = self.mupdf.open(&ask.str()?)?;
                let prepared = (|| {
                    let locked = self.mupdf.locked(&open)?;
                    // Unprotected files answer 1; owners carry bit 4.
                    let owner = !locked && matches!(self.mupdf.unlock(&open, "")?, access if access == 1 || access & 4 != 0);
                    self.mupdf.journal(&open)?;
                    Ok::<_, io::Error>((locked, owner))
                })();
                // A failed start releases what MuPDF opened.
                let (locked, owner) = match prepared {
                    Ok(prepared) => prepared,
                    Err(error) => {
                        self.mupdf.close(open);
                        return Err(error);
                    }
                };
                let id = self.next;
                self.next += 1;
                let warnings = output::open_warnings();
                let doc = Doc { open, password: String::new(), owner, warnings, repair: None, found: false, copy: None, steps: Vec::new(), at: 0 };
                self.docs.insert(id, doc);
                if !locked && let Err(error) = self.repair(id) {
                    self.forget(id)?;
                    return Err(error);
                }
                log::info!("pdf-worker: opened document {id}, locked {locked}");
                Ok(Pack::new().u64(id).u64(u64::from(locked)).bytes())
            }
            tag::UNLOCK => {
                let (id, password) = (ask.u64()?, ask.str()?);
                let access = self.mupdf.unlock(self.doc(id)?, &password)?;
                if access != 0 {
                    let doc = self.docs.get_mut(&id).expect("just read");
                    (doc.password, doc.owner) = (password, access & 4 != 0);
                    self.repair(id)?;
                }
                Ok(Pack::new().u64(u64::from(access != 0)).bytes())
            }
            tag::PAGES => Ok(Pack::new()
                .u64(self.mupdf.pages(self.doc(ask.u64()?)?)? as u64)
                .bytes()),
            tag::BOUNDS => {
                let doc = self.doc(ask.u64()?)?;
                let (first, count) = (ask.u64()? as usize, ask.u64()? as usize);
                let mut out = Pack::new();
                for number in first..first + count {
                    for value in self.mupdf.bounds(doc, number)? {
                        out = out.u64(u64::from(value.to_bits()));
                    }
                }
                Ok(out.bytes())
            }
            tag::RENDER => {
                let id = ask.u64()?;
                let page = ask.u64()? as usize;
                let scale = f32::from_bits(ask.u64()? as u32);
                let rotate = i32::try_from(ask.i64()?).map_err(io::Error::other)?;
                let mut tile = [0i32; 4];
                for value in &mut tile {
                    *value = i32::try_from(ask.i64()?).map_err(io::Error::other)?;
                }
                self.drawn(id, page, (scale, rotate), tile)
            }
            tag::TEXT => {
                let (id, page) = (ask.u64()?, ask.u64()? as usize);
                self.read(id, |mupdf, doc| mupdf.text(doc, page))
            }
            tag::SEARCH => {
                let (id, page, needle) = (ask.u64()?, ask.u64()? as usize, ask.str()?);
                self.read(id, |mupdf, doc| mupdf.search(doc, page, &needle))
            }
            tag::OUTLINE => self.mupdf.outline(self.doc(ask.u64()?)?),
            tag::HELLO => Ok(Pack::new().str(ely_pdf_engine::proto::BUILD).bytes()),
            tag::HTML_PDF => {
                let (dir, page, out) = (ask.str()?, ask.str()?, ask.str()?);
                self.mupdf.html_pdf(&dir, &page, &out).map(|()| Vec::new())
            }
            tag::CLOSE => {
                self.forget(ask.u64()?)?;
                Ok(Vec::new())
            }
            other => self.edit(other, &mut ask),
        }
    }

    /// The request without its revision, if it is still current.
    fn current(&self, payload: &[u8]) -> io::Result<Vec<u8>> {
        let mut ask = Unpack::new(payload);
        let (id, expected) = (ask.u64()?, ask.u64()?);
        let doc = self.record(id)?;
        if expected != ely_pdf_engine::proto::ANY && doc.mark() != expected.to_string() {
            return Err(io::Error::other(format!("{}document {id} is at {}, not {expected}", ely_pdf_engine::proto::STALE, doc.mark())));
        }
        Ok([&id.to_le_bytes()[..], ask.rest()].concat())
    }

    /// Releases document `id` and its repaired copy.
    fn forget(&mut self, id: u64) -> io::Result<()> {
        let doc = self.docs.remove(&id).ok_or_else(|| io::Error::other(format!("no document {id}")))?;
        self.mupdf.close(doc.open);
        if let Some(copy) = doc.copy {
            std::fs::remove_file(copy)?;
        }
        Ok(())
    }
}

fn main() -> io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::new().filter_or("ELY_LOG", "info")).init();
    ffi::script_font();
    // Numbers from another worker process match nothing here.
    let started = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(io::Error::other)?;
    let prefix = (u64::from(std::process::id()) ^ u64::from(started.subsec_nanos())) & 0xffff_ffff;
    let mut worker = Worker {
        mupdf: Mupdf::new()?,
        docs: HashMap::new(),
        next: (prefix << 32) | 1,
        edits: 0,
    };
    let (mut input, mut output) = (
        BufReader::new(io::stdin().lock()),
        BufWriter::new(io::stdout().lock()),
    );
    while let Some((kind, id, payload)) = read_frame(&mut input)? {
        let written = match worker.answer(kind, &payload) {
            Ok(reply) => write_frame(&mut output, tag::OK, id, &reply),
            Err(error) => {
                // Text may quote the document: reply, never log.
                log::warn!("pdf-worker: request {kind} failed ({:?})", error.kind());
                write_frame(&mut output, tag::ERR, id, error.to_string().as_bytes())
            }
        };
        // An oversized answer is refused; the worker lives.
        if let Err(error) = written {
            if error.kind() != io::ErrorKind::InvalidInput {
                return Err(error);
            }
            log::warn!("pdf-worker: request {kind} answer too large");
            write_frame(&mut output, tag::ERR, id, error.to_string().as_bytes())?;
        }
    }
    Ok(())
}
