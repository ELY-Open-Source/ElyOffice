use std::{
    io::{self, BufReader, BufWriter},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
};

use crate::ipc::{self, Pack, Unpack, tag};

pub use crate::events::{A11y, Cell, Events, Function, MIXED, Mark, Rect, Request};

/// The pinned LibreOffice build the worker must report (LOK-01).
pub const BUILD_ID: &str = "8221e31b3ac356a1623c672912a3d2b492f7e3d1";

mod doc;
mod feed;
mod frames;
mod install;
mod load;
pub use install::{install_dir, templates};
use install::worker_bin;
pub use doc::{Pdf, Sheet, Sheets};
pub use feed::Feed;
#[cfg(test)]
pub(crate) mod tests;

/// The LOK worker process: spawn, talk frames, shut down (LOK-01).
pub struct Worker {
    pub(crate) child: Child,
    stdin: BufWriter<ChildStdin>,
    /// Answers to requests; None once the worker is gone.
    replies: mpsc::Receiver<io::Result<(u8, u64, Vec<u8>)>>,
    feed: Feed,
    down: bool,
    /// The last request's id; each reply must carry it.
    asked: u64,
    /// This worker among all spawned; its documents carry it.
    session: u64,
    stderr: ely_ipc::Relay,
    check: Check,
    /// The name new comments sign with, once set.
    author: Option<String>,
}

/// The start-up check: to run, passed, or why it failed.
enum Check {
    Pending(PathBuf),
    Passed,
    Failed(String),
}

/// One painted tile and the generation it matches (LOK-03).
pub struct Tile {
    pub generation: u64,
    pub pixels: Vec<u8>,
}

/// One LOK mouse event; coordinates in twips.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mouse {
    /// 0 down, 1 up, 2 move.
    pub kind: i32,
    pub x: i32,
    pub y: i32,
    pub count: i32,
    /// 1 left, 2 middle, 4 right.
    pub buttons: i32,
    /// vcl modifier bits.
    pub modifiers: i32,
}

/// One open document: its worker, number and revision read.
#[derive(Clone, Debug, PartialEq)]
pub struct Doc {
    worker: u64,
    id: u64,
    /// The document's revision when this handle was taken.
    pub(crate) rev: u64,
    /// LOK_DOCTYPE: 0 text, 1 spreadsheet, 2 presentation.
    kind: u64,
}

impl Doc {
    /// Whether this is a spreadsheet.
    pub fn is_sheet(&self) -> bool {
        self.kind == 1
    }

    /// Whether this is a presentation.
    pub fn is_deck(&self) -> bool {
        self.kind == 2
    }

    /// Whether this handle saw a later state than `other`.
    pub fn newer_than(&self, other: &Doc) -> bool {
        self.rev > other.rev
    }
}

/// Whether `error` refused a request made for an older state.
pub fn is_stale(error: &io::Error) -> bool {
    error.to_string().starts_with(ipc::STALE)
}

/// Whether `error` refused an undo or redo with no history.
pub fn is_nothing(error: &io::Error) -> bool {
    error.to_string().starts_with(ipc::NOTHING)
}

static SESSIONS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl Worker {
    /// Spawns the worker against `install` with an isolated profile (LOK-01).
    pub fn spawn(install: &Path, profile_dir: &Path) -> io::Result<Self> {
        let mut worker = Self::spawn_mode(install, profile_dir, false)?;
        worker.ready()?;
        Ok(worker)
    }

    /// Starts the process only; the first request checks it.
    pub fn launch(install: &Path, profile_dir: &Path) -> io::Result<Self> {
        Self::spawn_mode(install, profile_dir, false)
    }

    /// Spawns with the dev-only probes on, for tests.
    pub fn spawn_dev(install: &Path, profile_dir: &Path) -> io::Result<Self> {
        let mut worker = Self::spawn_mode(install, profile_dir, true)?;
        worker.ready()?;
        Ok(worker)
    }

    /// Checks the pinned build and tiles, once (LOK-01).
    pub fn ready(&mut self) -> io::Result<()> {
        let profile = match &self.check {
            Check::Passed => return Ok(()),
            Check::Failed(why) => return Err(io::Error::other(format!("the worker failed its check: {why}"))),
            Check::Pending(profile) => profile.clone(),
        };
        // Marked first: the checks' own requests pass through here.
        self.check = Check::Passed;
        let checked = self.version_checked().and_then(|build| self.verify(&build, &profile).map(|()| build));
        match checked {
            Ok(build) => {
                log::info!("lok: worker up, {build}");
                Ok(())
            }
            Err(error) => {
                self.check = Check::Failed(error.to_string());
                Err(error)
            }
        }
    }

    /// Spawns with or without the dev probes.
    fn spawn_mode(install: &Path, profile_dir: &Path, dev: bool) -> io::Result<Self> {
        std::fs::create_dir_all(profile_dir)?;
        let worker_bin = worker_bin()?;
        log::info!("lok: worker from {}", worker_bin.display());
        let mut command = Command::new(worker_bin);
        ely_ipc::windowless(&mut command)
            .arg(install)
            .arg(profile_dir)
            .env("SAL_FORCE_HIDPI_SCALING", "1")
            // A killed worker's profile lock would hang the next.
            .env("SAL_NOLOCK_PROFILE", "1")
            // The kernel's OpenCL test can hang in AMD's driver.
            .env("SAL_DISABLE_OPENCL", "1");
        // Linux tiles via cairo are BGRA; Skia there is RGBA.
        match cfg!(target_os = "linux") {
            true => command.env("SAL_DISABLESKIA", "1"),
            false => command.env("SAL_SKIA", "raster"),
        };
        // Calc reports in document twips; unipoll lends LOK's loop.
        command.env("SAL_LOK_OPTIONS", "unipoll:sc_print_twips_msgs");
        // LOK spells no language off this list.
        command.env("LOK_ALLOWLIST_LANGUAGES", "en");
        if dev {
            command.arg("--dev");
        }
        if let Ok(sal_log) = std::env::var("ELY_LOK_SAL_LOG") {
            command.env("SAL_LOG", sal_log);
        }
        let temp = profile_dir.join("tmp");
        std::fs::create_dir_all(&temp)?;
        command.env("TMPDIR", &temp);
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| io::Error::new(error.kind(), format!("lok-worker spawn: {error}")))?;
        let stderr = ely_ipc::relay(child.stderr.take().expect("piped stderr"), "lok-worker");
        let (stdin, stdout) = (
            child.stdin.take().expect("piped stdin"),
            child.stdout.take().expect("piped stdout"),
        );
        let (feed, (sender, replies)) = (Feed::default(), mpsc::channel());
        let reading = feed.clone();
        std::thread::spawn(move || frames::read_frames(BufReader::new(stdout), &reading, &sender));
        let worker = Self {
            child,
            stdin: BufWriter::new(stdin),
            replies,
            feed,
            down: false,
            asked: 0,
            session: SESSIONS.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            stderr,
            check: Check::Pending(profile_dir.to_path_buf()),
            author: None,
        };
        Ok(worker)
    }

    /// The hooked library's BuildId.
    pub fn build_id(&mut self) -> io::Result<String> {
        let payload = self.request(tag::BUILD_ID, &[])?;
        Unpack::new(&payload).str()
    }

    /// The name new comments and tracked changes carry.
    pub fn set_author(&mut self, name: &str) -> io::Result<()> {
        log::info!("lok: the author is set");
        // Wanted even if this worker fails; a restart sends it.
        self.author = Some(name.to_string());
        self.request(tag::SET_AUTHOR, &Pack::new().str(name).bytes())?;
        Ok(())
    }

    /// The name last asked for; a restart signs with it.
    pub fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    /// Runs a macro by its URL (LOK-11 probe).
    pub fn run_macro(&mut self, url: &str) -> io::Result<i32> {
        let payload = self.request(tag::RUN_MACRO, &Pack::new().str(url).bytes())?;
        Ok(Unpack::new(&payload).u64()? as i32)
    }

    /// Posts a key press and release (LOK-06).
    pub fn post_key(&mut self, doc: &Doc, char_code: u32, key_code: i32) -> io::Result<()> {
        self.request(
            tag::POST_KEY,
            &Pack::new()
                .u64(self.doc_id(doc)?)
                .i64(char_code as i64)
                .i64(key_code as i64)
                .bytes(),
        )?;
        Ok(())
    }

    /// Turns `step` parts, or posts `key` if something is picked.
    pub fn turn(&mut self, doc: &Doc, step: i32, key: Option<(u32, i32)>) -> io::Result<Option<usize>> {
        let (char_code, key_code) = key.unwrap_or_default();
        let doc_id = self.doc_id(doc)?;
        let payload = Pack::new().u64(doc_id).i64(step.into()).i64(char_code.into()).i64(key_code.into()).bytes();
        let shown = Unpack::new(&self.request(tag::TURN, &payload)?).i64()?;
        Ok(usize::try_from(shown).ok())
    }

    /// Drags a selected object's handle, start then end.
    pub fn drag_graphic(&mut self, doc: &Doc, from: (i32, i32), to: (i32, i32)) -> io::Result<()> {
        for (kind, (x, y)) in [(0, from), (1, to)] {
            let pack = Pack::new()
                .u64(self.doc_id(doc)?)
                .i64(kind)
                .i64(x.into())
                .i64(y.into());
            self.request(tag::GRAPHIC, &pack.bytes())?;
        }
        Ok(())
    }

    /// Posts one mouse event in twips (LOK-04).
    pub fn post_mouse(&mut self, doc: &Doc, event: Mouse) -> io::Result<()> {
        let pack = [
            event.kind as i64,
            event.x as i64,
            event.y as i64,
            event.count as i64,
            event.buttons as i64,
            event.modifiers as i64,
        ]
        .into_iter()
        .fold(Pack::new().u64(self.doc_id(doc)?), Pack::i64);
        self.request(tag::POST_MOUSE, &pack.bytes())?;
        Ok(())
    }

    /// Updates the IME composition, or commits it (LOK-07).
    pub fn post_text(&mut self, doc: &Doc, commit: bool, text: &str) -> io::Result<()> {
        let kind = if commit { 2 } else { 0 };
        self.request(
            tag::POST_TEXT,
            &Pack::new().u64(self.doc_id(doc)?).i64(kind).str(text).bytes(),
        )?;
        Ok(())
    }

    /// Copies the selection; one (mime, bytes) per flavor (LOK-06).
    pub fn copy(&mut self, doc: &Doc) -> io::Result<Vec<(String, Vec<u8>)>> {
        let payload = self.request(tag::COPY, &Pack::new().u64(self.doc_id(doc)?).bytes())?;
        let mut unpack = Unpack::new(&payload);
        let count = unpack.u64()?;
        (0..count)
            .map(|_| Ok((unpack.str()?, unpack.blob()?)))
            .collect()
    }

    /// Cuts the selection after a copy went out (LOK-06).
    pub fn cut(&mut self, doc: &Doc) -> io::Result<()> {
        self.request(tag::CUT, &Pack::new().u64(self.doc_id(doc)?).bytes())?;
        Ok(())
    }

    /// Pastes one flavor into the document (LOK-06).
    pub fn paste(&mut self, doc: &Doc, mime: &str, data: &[u8]) -> io::Result<()> {
        self.request(
            tag::PASTE,
            &Pack::new().u64(self.doc_id(doc)?).str(mime).blob(data).bytes(),
        )?;
        Ok(())
    }

    /// Runs a table command; the worker confirms it took (LOK-02).
    pub fn run(&mut self, doc: &Doc, id: &str, values: &[&str]) -> io::Result<()> {
        self.run_all(doc, &[(id, values)])
    }

    /// Commands made from one read; the state is checked once.
    pub fn run_all(&mut self, doc: &Doc, commands: &[(&str, &[&str])]) -> io::Result<()> {
        let mut addressed = false;
        for (id, values) in commands {
            let command = crate::commands::find(id)?;
            command.args_json(values)?;
            addressed |= command.addressed(values);
        }
        // Numbers hold only in the state they were read.
        if addressed {
            self.request(tag::EXPECT, &Pack::new().u64(self.doc_id(doc)?).u64(doc.rev).bytes())?;
        }
        for (id, values) in commands {
            let pack = values.iter().fold(
                Pack::new().u64(self.doc_id(doc)?).str(id).u64(values.len() as u64),
                |pack, value| pack.str(value),
            );
            self.request(tag::POST_UNO, &pack.bytes())?;
        }
        Ok(())
    }

    /// `doc` as of now: commands by number use this revision.
    pub fn now(&mut self, doc: &Doc) -> io::Result<Doc> {
        let payload = self.request(tag::REVISION, &Pack::new().u64(self.doc_id(doc)?).bytes())?;
        Ok(Doc { rev: Unpack::new(&payload).u64()?, ..doc.clone() })
    }

    /// `doc`'s number here; another worker's document is refused.
    pub(crate) fn doc_id(&self, doc: &Doc) -> io::Result<u64> {
        match doc.worker == self.session {
            true => Ok(doc.id),
            false => Err(io::Error::new(io::ErrorKind::NotFound, "a document of another lok worker")),
        }
    }

    /// The kernel's JSON list for `command`: fonts or styles (LOK-05).
    pub fn values(&mut self, doc: &Doc, command: &str) -> io::Result<String> {
        let payload = self.request(tag::VALUES, &Pack::new().u64(self.doc_id(doc)?).str(command).bytes())?;
        Unpack::new(&payload).str()
    }

    /// Events past seq `known`, waiting up to `timeout_ms`.
    pub fn poll_events(&self, doc: &Doc, known: u64, timeout_ms: u64) -> io::Result<Events> {
        self.feed.wait(doc, known, timeout_ms)
    }

    /// The pushed events, waited on without this worker's lock.
    pub fn feed(&self) -> Feed {
        self.feed.clone()
    }

    /// A worker whose output failed: stopped, reaped, and why.
    fn broken(&mut self, request: u8, error: io::Error) -> io::Error {
        let _ = self.child.kill();
        let status = self.child.wait().map(|status| status.to_string()).unwrap_or_else(|wait| wait.to_string());
        log::error!("lok: the worker failed on request {request} ({status})");
        let said = self.stderr.last_error().map_or_else(String::new, |said| format!(": {said}"));
        io::Error::new(error.kind(), format!("the lok worker failed: {error} ({status}){said}"))
    }

    /// One round trip per request.
    pub(crate) fn request(&mut self, request: u8, payload: &[u8]) -> io::Result<Vec<u8>> {
        // Nothing reaches a worker that has not passed its checks.
        self.ready()?;
        self.asked += 1;
        if let Err(error) = ipc::write_frame(&mut self.stdin, request, self.asked, payload) {
            return Err(self.broken(request, error));
        }
        let (reply, id, body) = match self.replies.recv() {
            Ok(Ok(frame)) => frame,
            Ok(Err(error)) => return Err(self.broken(request, error)),
            Err(_) => return Err(self.broken(request, io::Error::other("the reply reader stopped"))),
        };
        if let Err(error) = ipc::check_reply(self.asked, id) {
            return Err(self.broken(request, error));
        }
        match reply {
            tag::OK => Ok(body),
            tag::ERR => Err(io::Error::other(Unpack::new(&body).str()?)),
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown reply {other}"),
            )),
        }
    }

    /// Why the worker process ended, if it has.
    pub fn ended(&mut self) -> Option<String> {
        match self.child.try_wait() {
            Ok(status) => status.map(|status| status.to_string()),
            Err(error) => Some(error.to_string()),
        }
    }

    /// Closes the document in the worker.
    pub fn close_doc(&mut self, doc: Doc) -> io::Result<()> {
        self.request(tag::CLOSE_DOC, &Pack::new().u64(self.doc_id(&doc)?).bytes())?;
        self.feed.closed(doc.id);
        Ok(())
    }

    /// Shuts the worker down and reaps it.
    pub fn shutdown(&mut self) -> io::Result<()> {
        if self.down {
            return Ok(());
        }
        self.down = true;
        let mut outcome = self.answer_shutdown();
        if outcome.is_err() {
            let _ = self.child.kill();
        }
        let status = match self.child.wait() {
            Ok(status) => status,
            Err(error) => {
                let _ = self.child.kill();
                return Err(io::Error::new(
                    error.kind(),
                    format!("the lok worker could not be reaped: {error}"),
                ));
            }
        };
        if outcome.is_ok() && !status.success() {
            outcome = Err(io::Error::other(format!(
                "the lok worker exited as {status}"
            )));
        }
        if outcome.is_ok() {
            log::info!("lok: worker down ({status})");
        }
        outcome
    }

    /// The shutdown round trip alone.
    fn answer_shutdown(&mut self) -> io::Result<()> {
        self.asked += 1;
        ipc::write_frame(&mut self.stdin, tag::SHUTDOWN, self.asked, &[])?;
        let reply = match self.replies.recv() {
            Ok(Ok(frame)) => frame,
            Ok(Err(error)) => return Err(self.broken(tag::SHUTDOWN, error)),
            Err(_) => return Err(self.broken(tag::SHUTDOWN, io::Error::other("the reply reader stopped"))),
        };
        ipc::check_reply(self.asked, reply.1)?;
        match reply {
            (tag::OK, _, _) => Ok(()),
            (tag::ERR, _, body) => Err(io::Error::other(Unpack::new(&body).str()?)),
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("bad shutdown answer: {other:?}"),
            )),
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if let Err(error) = self.shutdown() {
            log::warn!("lok: shutdown failed: {error}");
        }
    }
}
