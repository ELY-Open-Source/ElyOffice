//! The app's side: spawn the worker, ask, read the answer.

use std::{
    io::{self, BufReader, BufWriter},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use ely_ipc::{Pack, Unpack, check_reply, read_frame, write_frame};

use crate::proto::{self, Bookmark, Char, tag};


/// The PDF worker process.
pub struct Worker {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    /// The last request's id; each reply must carry it.
    asked: u64,
    /// This worker among all spawned; its documents carry it.
    session: u64,
}

static SESSIONS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// An open document: its worker, number and history mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Doc {
    worker: u64,
    id: u64,
    /// The history mark this handle was read at.
    mark: u64,
}

impl Doc {
    /// This document as of history `mark`, which an edit answered.
    pub fn at(self, mark: &str) -> io::Result<Doc> {
        let mark = mark.parse().map_err(|_| io::Error::other(format!("a history mark that is not a number: {mark}")))?;
        Ok(Doc { mark, ..self })
    }
}

/// Whether `error` refused a request made for an older state.
pub fn is_stale(error: &io::Error) -> bool {
    error.to_string().starts_with(proto::STALE)
}

/// Drawn pixels, BGRA, and whether drawing them met damage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tile {
    pub bgra: Vec<u8>,
    pub damaged: bool,
}

/// A document just opened: locked until its password is given.
#[derive(Clone, Copy, Debug)]
pub struct Opened {
    pub doc: Doc,
    pub locked: bool,
}

const WORKER: &str = if cfg!(windows) {
    "ely-pdf-worker.exe"
} else {
    "ely-pdf-worker"
};

/// The worker binary: `ELY_PDF_WORKER`, beside the app, or this build's.
fn worker_bin() -> io::Result<PathBuf> {
    if let Some(bin) = std::env::var_os("ELY_PDF_WORKER") {
        let bin = PathBuf::from(bin);
        return match bin.is_file() {
            true => Ok(bin),
            false => Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("ELY_PDF_WORKER {} is not there", bin.display()),
            )),
        };
    }
    let exe = std::env::current_exe()?;
    let beside = exe.parent().unwrap_or(Path::new(".")).join(WORKER);
    if beside.is_file() {
        return Ok(beside);
    }
    // Test binaries live one folder below the worker.
    let above = exe
        .parent()
        .and_then(Path::parent)
        .map(|dir| dir.join(WORKER));
    above.filter(|bin| bin.is_file()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "no ely-pdf-worker binary (set ELY_PDF_WORKER)",
        )
    })
}

impl Worker {
    pub fn spawn() -> io::Result<Self> {
        Self::spawn_from(&worker_bin()?)
    }

    /// Spawns `bin`; one from other sources is refused.
    pub fn spawn_from(bin: &Path) -> io::Result<Self> {
        let mut child = ely_ipc::windowless(&mut Command::new(bin))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| io::Error::new(error.kind(), format!("pdf-worker spawn: {error}")))?;
        // Its records reach the app log; nothing reads them back.
        drop(ely_ipc::relay(child.stderr.take().expect("piped stderr"), "pdf-worker"));
        let stdin = BufWriter::new(child.stdin.take().expect("piped stdin"));
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
        let mut worker = Worker {
            child,
            stdin,
            stdout,
            asked: 0,
            session: SESSIONS.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        };
        log::info!("pdf: worker from {}", bin.display());
        let reply = worker.request(tag::HELLO, &[])?;
        if Unpack::new(&reply).str()? != proto::BUILD {
            return Err(io::Error::other(format!(
                "the pdf worker {} is from other sources; run cargo build -p ely-pdf-worker",
                bin.display()
            )));
        }
        log::info!("pdf: worker up");
        Ok(worker)
    }

    /// `doc` as it is now, for tests that chain edits.
    #[cfg(any(test, feature = "test-support"))]
    pub fn now(&mut self, doc: Doc) -> io::Result<Doc> {
        let mark = self.history(doc)?;
        doc.at(&mark)
    }

    /// Why the worker process ended, if it has.
    pub fn ended(&mut self) -> Option<String> {
        match self.child.try_wait() {
            Ok(status) => status.map(|status| status.to_string()),
            Err(error) => Some(error.to_string()),
        }
    }

    /// The worker's process id, for resource tests.
    #[cfg(any(test, feature = "test-support"))]
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Ends the worker process, for crash tests.
    #[cfg(any(test, feature = "test-support"))]
    pub fn kill(&mut self) -> io::Result<()> {
        self.child.kill()?;
        self.child.wait().map(|_| ())
    }

    /// A request aimed at pages or objects `doc` saw.
    pub(crate) fn on(&self, doc: &Doc) -> io::Result<Pack> {
        Ok(Pack::new().u64(self.doc_id(doc)?).u64(doc.mark))
    }

    /// A request on `doc` whatever its state: the whole document.
    pub(crate) fn whole(&self, doc: &Doc) -> io::Result<Pack> {
        Ok(Pack::new().u64(self.doc_id(doc)?).u64(proto::ANY))
    }

    /// `doc`'s number here; another worker's document is refused.
    fn doc_id(&self, doc: &Doc) -> io::Result<u64> {
        match doc.worker == self.session {
            true => Ok(doc.id),
            false => Err(io::Error::new(io::ErrorKind::NotFound, "a document of another pdf worker")),
        }
    }

    pub(crate) fn request(&mut self, kind: u8, payload: &[u8]) -> io::Result<Vec<u8>> {
        self.asked += 1;
        write_frame(&mut self.stdin, kind, self.asked, payload)?;
        let frame = read_frame(&mut self.stdout)?;
        if let Some((_, id, _)) = &frame {
            check_reply(self.asked, *id)?;
        }
        match frame {
            Some((tag::OK, _, reply)) => Ok(reply),
            Some((tag::ERR, _, reply)) => Err(refusal(String::from_utf8_lossy(&reply).into_owned())),
            Some((other, _, _)) => Err(io::Error::other(format!("pdf-worker answered tag {other}"))),
            None => Err(io::Error::other(match self.child.try_wait() {
                Ok(Some(status)) => format!("the pdf worker exited: {status}"),
                _ => "the pdf worker closed its pipe".to_string(),
            })),
        }
    }

    /// Opens `path`; a locked file waits for `unlock`.
    pub fn open(&mut self, path: &Path) -> io::Result<Opened> {
        let path = path
            .to_str()
            .ok_or_else(|| io::Error::other("a path that is not UTF-8"))?;
        let reply = self.request(tag::OPEN, &Pack::new().str(path).bytes())?;
        let mut reply = Unpack::new(&reply);
        Ok(Opened {
            doc: Doc { worker: self.session, id: reply.u64()?, mark: 0 },
            locked: reply.u64()? != 0,
        })
    }

    /// Tries `password`; false when it is wrong.
    pub fn unlock(&mut self, doc: Doc, password: &str) -> io::Result<bool> {
        let reply = self.request(tag::UNLOCK, &self.whole(&doc)?.str(password).bytes())?;
        Ok(Unpack::new(&reply).u64()? != 0)
    }

    pub fn pages(&mut self, doc: Doc) -> io::Result<usize> {
        let reply = self.request(tag::PAGES, &self.whole(&doc)?.bytes())?;
        Ok(Unpack::new(&reply).u64()? as usize)
    }

    /// Page boxes in points, `count` from `first`.
    pub fn bounds(&mut self, doc: Doc, first: usize, count: usize) -> io::Result<Vec<[f32; 4]>> {
        let ask = self.on(&doc)?
            .u64(first as u64)
            .u64(count as u64)
            .bytes();
        proto::boxes(&self.request(tag::BOUNDS, &ask)?, count)
    }

    /// A page's device pixels `tile` (x, y, w, h), BGRA.
    pub fn render(
        &mut self,
        doc: Doc,
        page: usize,
        (scale, rotate): (f32, i32),
        tile: [i32; 4],
    ) -> io::Result<Tile> {
        let mut ask = self.on(&doc)?
            .u64(page as u64)
            .u64(u64::from(scale.to_bits()))
            .i64(i64::from(rotate));
        for value in tile {
            ask = ask.i64(i64::from(value));
        }
        let reply = self.request(tag::RENDER, &ask.bytes())?;
        let mut reply = Unpack::new(&reply);
        let (damaged, bgra) = (reply.u64()? != 0, reply.blob()?);
        match bgra.len() == tile[2] as usize * tile[3] as usize * 4 {
            true => Ok(Tile { bgra, damaged }),
            false => Err(io::Error::other("a tile of the wrong size")),
        }
    }

    pub fn text(&mut self, doc: Doc, page: usize) -> io::Result<Vec<Char>> {
        proto::chars(&self.request(tag::TEXT, &self.on(&doc)?.u64(page as u64).bytes())?)
    }

    /// Where `needle` is on a page: each hit's quads.
    pub fn search(&mut self, doc: Doc, page: usize, needle: &str) -> io::Result<Vec<Vec<[f32; 8]>>> {
        let ask = self.on(&doc)?.u64(page as u64).str(needle).bytes();
        proto::hits(&self.request(tag::SEARCH, &ask)?)
    }

    pub fn outline(&mut self, doc: Doc) -> io::Result<Vec<Bookmark>> {
        let reply = self.request(tag::OUTLINE, &self.whole(&doc)?.bytes())?;
        proto::outline(&String::from_utf8(reply).map_err(io::Error::other)?)
    }

    pub fn close(&mut self, doc: Doc) -> io::Result<()> {
        self.request(tag::CLOSE, &self.whole(&doc)?.bytes())
            .map(|_| ())
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        // The worker holds nothing that needs saving.
        if let Err(error) = self.child.kill() {
            log::info!("pdf: the worker was already gone: {error}");
        }
        if let Err(error) = self.child.wait() {
            log::warn!("pdf: the worker was not reaped: {error}");
        }
    }
}

/// A request the document's permissions refused, by what it needs.
#[derive(Debug)]
pub struct Refused(pub &'static str);

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", crate::proto::may::REFUSED, self.0)
    }
}

impl std::error::Error for Refused {}

/// What a permission refusal needs; none for other errors.
pub fn refused(error: &io::Error) -> Option<&'static str> {
    error.get_ref()?.downcast_ref::<Refused>().map(|refused| refused.0)
}

/// A worker error; a refusal names a permission it knows.
fn refusal(text: String) -> io::Error {
    let named = text
        .strip_prefix(crate::proto::may::REFUSED)
        .and_then(|needed| crate::proto::may::NAMES.iter().find(|(_, name)| *name == needed));
    match named {
        Some((_, name)) => io::Error::other(Refused(name)),
        None => io::Error::other(text),
    }
}
