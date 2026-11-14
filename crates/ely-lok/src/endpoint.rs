//! The per-user endpoint between an app and its worker.

use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::ipc::{self, Pack, Unpack, tag};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as os;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as os;
pub(crate) use os::{Half, Peer, hang_up};

/// The worker's one stdout line once it listens.
const READY: &str = "ely-lok-worker: listening";

/// The profile's file an idle engine locks.
const LOCK: &str = "idle.lock";

/// The profile's file its starter locks, spawn to release.
const STARTING: &str = "starting.lock";

/// How long one wait for a starting engine lasts.
const STARTING_SLICE: Duration = Duration::from_millis(100);

/// The kernel's id, beside its version file.
#[cfg(target_os = "macos")]
const KERNEL_ID: &str = "../Resources/ely-kernel-id";
#[cfg(not(target_os = "macos"))]
const KERNEL_ID: &str = "ely-kernel-id";

/// The id `scripts/kernel-id.sh` wrote for `install`.
fn kernel_id(install: &Path) -> io::Result<String> {
    let file = install.join(KERNEL_ID);
    let text = std::fs::read_to_string(&file).map_err(|error| {
        io::Error::new(error.kind(), format!("{}: {error} (scripts/kernel-id.sh writes it)", file.display()))
    })?;
    let id = text.trim();
    match id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        true => Ok(id.to_string()),
        false => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: {id:?} is no kernel id", file.display()),
        )),
    }
}

/// This build's version and `install`'s kernel id.
fn identity(install: &Path) -> io::Result<(String, String)> {
    Ok((env!("CARGO_PKG_VERSION").to_string(), kernel_id(install)?))
}

/// An endpoint name no other spawn shares.
pub(crate) fn name(session: u64) -> io::Result<String> {
    // A reused pid counts sessions from 1 again.
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(io::Error::other)?;
    Ok(format!("lok-{}-{session}-{:x}", std::process::id(), now.as_nanos()))
}

/// A name's server: a spawned pid, or idle.
#[derive(Clone, Copy)]
#[cfg_attr(unix, expect(dead_code, reason = "the 0700 folder vouches on Unix"))]
pub(crate) enum Server {
    Spawned(u32),
    Idle,
}

/// The fixed name an app's idle engine waits on.
pub(crate) fn idle_name(app: &str) -> String {
    format!("idle-{app}")
}

/// The worker's frames in; an idle engine changes apps once.
pub struct Inbound {
    read: BufReader<Half>,
    idle: Option<Idle>,
}

/// What an idle engine holds until an app takes it.
struct Idle {
    /// Accepted only after the check.
    listener: os::Listener,
    /// Held while idle: one idle engine per app.
    lock: File,
    ours: (String, String),
    name: String,
}

/// The worker's side: listen, say so, take its checker.
pub fn take_app(install: &str, profile: &str, name: &str, idle: Option<&str>) -> io::Result<(Inbound, Half)> {
    let ours = identity(Path::new(install))?;
    let idle = match idle {
        Some(public) => {
            let lock = os::hold(&Path::new(profile).join(LOCK))?;
            // Bound now: apps wait here through the check.
            let listener = os::Listener::bind(public, true)?;
            Some(Idle { listener, lock, ours: ours.clone(), name: public.to_string() })
        }
        None => None,
    };
    // Before the kernel loads: its threads inherit it.
    if idle.is_some() {
        crate::priority::background()?;
    }
    let listener = os::Listener::bind(name, false)?;
    let checked = lifeline()?;
    os::announce(READY)?;
    let (mut read, mut write) = listener.accept()?;
    // Connected: a checker gone now fails the greeting.
    checked.store(true, Ordering::SeqCst);
    greet(&mut read, &mut write, &ours)?;
    log::info!("lok-worker: app {} connected on {name}", ours.0);
    listener.close()?;
    Ok((Inbound { read: BufReader::new(read), idle }, write))
}

/// Exits if stdin closes before the app connects.
fn lifeline() -> io::Result<Arc<AtomicBool>> {
    let checked = Arc::new(AtomicBool::new(false));
    let watched = Arc::clone(&checked);
    std::thread::Builder::new().name("lok-lifeline".into()).spawn(move || {
        let ended = io::copy(&mut io::stdin(), &mut io::sink());
        if !watched.load(Ordering::SeqCst) {
            // No lock is kept by an engine nobody checks.
            log::error!("lok-worker: the app left before connecting ({ended:?})");
            std::process::exit(1);
        }
    })?;
    Ok(checked)
}

/// Whether an engine starts or waits here; takes nothing.
pub(crate) fn idle_waits(profile: &Path) -> io::Result<bool> {
    // Starting first: its holder lets go after the engine locks.
    Ok(os::held(&profile.join(STARTING))? || os::held(&profile.join(LOCK))?)
}

/// A start in one folder, held from spawn to release.
pub struct Start {
    _lock: File,
    dir: PathBuf,
}

impl Start {
    /// Waits out another start in `dir`, then holds it.
    pub fn wait(dir: &Path) -> io::Result<Self> {
        std::fs::create_dir_all(dir).map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", dir.display())))?;
        let path = dir.join(STARTING);
        let mut waited = false;
        loop {
            match os::hold(&path) {
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    if !waited {
                        log::info!("lok: another start holds {}; waiting", path.display());
                        waited = true;
                    }
                    std::thread::sleep(STARTING_SLICE);
                }
                held => return Ok(Self { _lock: held?, dir: dir.to_path_buf() }),
            }
        }
    }

    /// Whether this folder's checked engine waits.
    pub fn ready(&self) -> io::Result<bool> {
        os::held(&self.dir.join(LOCK))
    }

    /// The folder this start holds.
    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }
}

/// Reads a HELLO; a stranger gets ERR and an error.
fn greet(read: &mut Half, write: &mut Half, ours: &(String, String)) -> io::Result<()> {
    let body = match ipc::read_frame(read)? {
        Some((tag::HELLO, _, body)) => body,
        other => return Err(io::Error::new(io::ErrorKind::InvalidData, format!("no hello: {other:?}"))),
    };
    let mut hello = Unpack::new(&body);
    let theirs = (hello.str()?, hello.str()?);
    if theirs != *ours {
        let why = format!(
            "the app is {} with kernel {}; this engine is {} with kernel {}",
            theirs.0, theirs.1, ours.0, ours.1
        );
        ipc::write_frame(write, tag::ERR, ipc::UNASKED, &Pack::new().str(&why).bytes())?;
        return Err(io::Error::new(io::ErrorKind::InvalidData, why));
    }
    let pid = Pack::new().u64(u64::from(std::process::id())).bytes();
    ipc::write_frame(write, tag::OK, ipc::UNASKED, &pid)
}

impl Inbound {
    /// The next frame from the current app.
    pub(crate) fn frame(&mut self) -> io::Result<Option<(u8, u64, Vec<u8>)>> {
        ipc::read_frame(&mut self.read)
    }

    /// After RELEASE: the checker leaves, the next app comes.
    pub(crate) fn hand_over(&mut self) -> io::Result<Half> {
        let Some(Idle { listener, lock, ours, name }) = self.idle.take() else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "a release on a private endpoint"));
        };
        // The checker closes first; nothing of its is lost.
        if let Some(frame) = ipc::read_frame(&mut self.read)? {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("a frame after release: {frame:?}")));
        }
        log::info!("lok-worker: checked, idle on {name}");
        let (mut read, mut write) = listener.accept()?;
        // Name, then lock: a successor binds after both.
        listener.close()?;
        drop(lock);
        // Normal before the app hears OK.
        crate::priority::foreground()?;
        greet(&mut read, &mut write, &ours)?;
        log::info!("lok-worker: taken by app {} on {name}", ours.0);
        self.read = BufReader::new(read);
        Ok(write)
    }
}

/// App side: await the line, then connect.
pub(crate) fn reach(stdout: impl Read, name: &str, pid: u32, install: &Path) -> io::Result<(Half, Half)> {
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line)?;
    if line.trim_end() != READY {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            format!("the worker did not listen; it said {line:?}"),
        ));
    }
    let (read, write, said, _) = join(name, Server::Spawned(pid), &identity(install)?)?;
    if said != pid {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("worker {pid} answered as {said}")));
    }
    Ok((read, write))
}

/// Takes the idle engine; waits while it starts.
pub(crate) fn adopt(name: &str, install: &Path, profile: &Path) -> io::Result<(Half, Half, u32, Peer)> {
    // Read once: a missing id is no start.
    let ours = identity(install)?;
    let mut waited = false;
    loop {
        match join(name, Server::Idle, &ours) {
            Err(error) if starting(&error) && idle_waits(profile)? => {
                if !waited {
                    log::info!("lok: the engine on {name} is starting ({error}); waiting");
                    waited = true;
                }
                std::thread::sleep(STARTING_SLICE);
            }
            joined => return joined,
        }
    }
}

/// Not bound, not yet accepting, or another app waits.
fn starting(error: &io::Error) -> bool {
    use io::ErrorKind::{ConnectionRefused, NotFound, ResourceBusy};
    matches!(error.kind(), NotFound | ConnectionRefused | ResourceBusy)
}

/// Connects and agrees; the answer names the engine's pid.
fn join(name: &str, server: Server, (version, kernel): &(String, String)) -> io::Result<(Half, Half, u32, Peer)> {
    let (mut read, mut write, peer) = os::connect(name, server)?;
    let hello = Pack::new().str(version).str(kernel).bytes();
    ipc::write_frame(&mut write, tag::HELLO, ipc::UNASKED, &hello)?;
    match ipc::read_frame(&mut read)? {
        Some((tag::OK, _, body)) => {
            let said = u32::try_from(Unpack::new(&body).u64()?)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "the engine's pid is out of range"))?;
            // Windows names the pipe's server; it must be the speaker.
            if let Some(server) = peer.server()
                && server != said
            {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("{name} is served by {server:?}, which says it is {said}"),
                ));
            }
            log::info!("lok: connected to worker {said} on {name}");
            Ok((read, write, said, peer))
        }
        // It refused us and exits.
        Some((tag::ERR, _, body)) => Err(io::Error::new(io::ErrorKind::ConnectionRefused, Unpack::new(&body).str()?)),
        // It went to another app first.
        None => Err(io::Error::new(io::ErrorKind::ConnectionReset, format!("{name} closed before answering"))),
        other => Err(io::Error::new(io::ErrorKind::InvalidData, format!("bad hello answer: {other:?}"))),
    }
}
