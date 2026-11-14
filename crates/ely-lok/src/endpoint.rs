//! The per-user endpoint between an app and its worker.

use std::{
    io::{self, BufRead, BufReader, Read},
    path::Path,
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
pub(crate) use os::Half;

/// The worker's one stdout line once it listens.
const READY: &str = "ely-lok-worker: listening";

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

/// An endpoint name unique to one spawn of this app.
pub(crate) fn name(session: u64) -> String {
    format!("lok-{}-{session}", std::process::id())
}

/// The worker's side: listen, say so, take one app.
pub fn take_app(install: &str, name: &str) -> io::Result<(Half, Half)> {
    let ours = identity(Path::new(install))?;
    let listener = os::Listener::bind(name)?;
    os::announce(READY)?;
    let (mut read, mut write) = listener.accept()?;
    let body = match ipc::read_frame(&mut read)? {
        Some((tag::HELLO, _, body)) => body,
        other => return Err(io::Error::new(io::ErrorKind::InvalidData, format!("no hello: {other:?}"))),
    };
    let mut hello = Unpack::new(&body);
    let theirs = (hello.str()?, hello.str()?);
    if theirs != ours {
        let why = format!(
            "the app is {} with kernel {}; this engine is {} with kernel {}",
            theirs.0, theirs.1, ours.0, ours.1
        );
        ipc::write_frame(&mut write, tag::ERR, ipc::UNASKED, &Pack::new().str(&why).bytes())?;
        return Err(io::Error::new(io::ErrorKind::InvalidData, why));
    }
    ipc::write_frame(&mut write, tag::OK, ipc::UNASKED, &[])?;
    log::info!("lok-worker: app {} connected on {name}", ours.0);
    Ok((read, write))
}

/// The app's side: wait for the line, connect, agree.
pub(crate) fn reach(stdout: impl Read, name: &str, pid: u32, install: &Path) -> io::Result<(Half, Half)> {
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line)?;
    if line.trim_end() != READY {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            format!("the worker did not listen; it said {line:?}"),
        ));
    }
    let (version, kernel) = identity(install)?;
    let (mut read, mut write) = os::connect(name, pid)?;
    let hello = Pack::new().str(&version).str(&kernel).bytes();
    ipc::write_frame(&mut write, tag::HELLO, ipc::UNASKED, &hello)?;
    match ipc::read_frame(&mut read)? {
        Some((tag::OK, _, _)) => {
            log::info!("lok: connected to worker {pid} on {name}");
            Ok((read, write))
        }
        Some((tag::ERR, _, body)) => Err(io::Error::new(io::ErrorKind::InvalidData, Unpack::new(&body).str()?)),
        other => Err(io::Error::new(io::ErrorKind::InvalidData, format!("bad hello answer: {other:?}"))),
    }
}
