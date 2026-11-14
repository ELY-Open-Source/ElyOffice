//! Unix: a socket in a private folder.

use std::{
    fs::{self, DirBuilder, File, Permissions},
    io::{self, Write},
    os::{
        fd::AsRawFd,
        unix::{
            fs::{DirBuilderExt, MetadataExt, PermissionsExt},
            net::{UnixListener, UnixStream},
        },
    },
    path::{Path, PathBuf},
};

pub(crate) type Half = UnixStream;

/// The user's temp folder; the kernel's TMPDIR differs.
#[cfg(target_os = "macos")]
fn base() -> io::Result<PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    let mut buffer = vec![0u8; 1024];
    let len = unsafe { libc::confstr(libc::_CS_DARWIN_USER_TEMP_DIR, buffer.as_mut_ptr().cast(), buffer.len()) };
    if len == 0 || len > buffer.len() {
        return Err(io::Error::other(format!("no user temp folder: {}", io::Error::last_os_error())));
    }
    buffer.truncate(len - 1);
    Ok(PathBuf::from(std::ffi::OsString::from_vec(buffer)))
}

#[cfg(not(target_os = "macos"))]
fn base() -> io::Result<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|base| !base.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "XDG_RUNTIME_DIR is not set"))
}

fn named(error: io::Error, path: &Path) -> io::Error {
    io::Error::new(error.kind(), format!("{}: {error}", path.display()))
}

/// The endpoints' folder: made 0700, refused if anyone else's.
fn folder() -> io::Result<PathBuf> {
    let dir = base()?.join("ely-office");
    match DirBuilder::new().mode(0o700).create(&dir) {
        Err(error) if error.kind() != io::ErrorKind::AlreadyExists => return Err(named(error, &dir)),
        _ => {}
    }
    let meta = fs::symlink_metadata(&dir).map_err(|error| named(error, &dir))?;
    let me = unsafe { libc::geteuid() };
    if !meta.is_dir() || meta.uid() != me || meta.mode() & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{} is not a folder only user {me} opens", dir.display()),
        ));
    }
    Ok(dir)
}

fn path(name: &str) -> io::Result<PathBuf> {
    Ok(folder()?.join(format!("{name}.sock")))
}

/// A bound socket, waiting for an app.
pub(crate) struct Listener {
    socket: UnixListener,
    path: PathBuf,
}

impl Listener {
    /// The idle lock is ours: old sockets are stale.
    pub(crate) fn bind(name: &str, idle: bool) -> io::Result<Self> {
        let path = path(name)?;
        if idle {
            match fs::remove_file(&path) {
                Ok(()) => log::info!("lok-worker: removed the stale {}", path.display()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(named(error, &path)),
            }
        }
        let socket = UnixListener::bind(&path).map_err(|error| named(error, &path))?;
        fs::set_permissions(&path, Permissions::from_mode(0o600)).map_err(|error| named(error, &path))?;
        log::info!("lok-worker: listening on {}", path.display());
        Ok(Self { socket, path })
    }

    /// The next app in line.
    pub(crate) fn accept(&self) -> io::Result<(Half, Half)> {
        let (stream, _) = self.socket.accept()?;
        Ok((stream.try_clone()?, stream))
    }

    /// Its name goes; no one else connects.
    pub(crate) fn close(self) -> io::Result<()> {
        fs::remove_file(&self.path).map_err(|error| named(error, &self.path))
    }
}

/// Says it listens, then stdout joins stderr.
pub(crate) fn announce(line: &str) -> io::Result<()> {
    let mut out = io::stdout().lock();
    writeln!(out, "{line}")?;
    out.flush()?;
    if unsafe { libc::dup2(2, 1) } != 1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// The worker's socket; the 0700 folder vouches for it.
pub(crate) fn connect(name: &str, _: super::Server) -> io::Result<(Half, Half, Peer)> {
    let path = path(name)?;
    let stream = UnixStream::connect(&path).map_err(|error| named(error, &path))?;
    let peer = Peer(stream.try_clone()?);
    Ok((stream.try_clone()?, stream, peer))
}

/// The engine behind a connection, ended by hanging up.
pub(crate) struct Peer(UnixStream);

impl Peer {
    /// The folder vouches; no pid is checked.
    pub(crate) fn server(&self) -> Option<u32> {
        None
    }

    /// The engine sees EOF and exits within its grace.
    pub(crate) fn end(&self) -> io::Result<()> {
        match self.0.shutdown(std::net::Shutdown::Both) {
            Err(error) if error.kind() != io::ErrorKind::NotConnected => Err(error),
            _ => Ok(()),
        }
    }
}

/// A whole-file write lock for `path`, or AlreadyExists.
pub(crate) fn hold(path: &Path) -> io::Result<File> {
    let file = File::options().write(true).create(true).truncate(false).open(path).map_err(|error| named(error, path))?;
    let mut lock = whole();
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETLK, &mut lock) } == -1 {
        let error = io::Error::last_os_error();
        return match error.raw_os_error() {
            Some(libc::EAGAIN | libc::EACCES) => {
                Err(io::Error::new(io::ErrorKind::AlreadyExists, "another idle engine waits for this app"))
            }
            _ => Err(named(error, path)),
        };
    }
    Ok(file)
}

/// Whether another process holds `path`; takes nothing.
pub(crate) fn held(path: &Path) -> io::Result<bool> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(named(error, path)),
    };
    let mut lock = whole();
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETLK, &mut lock) } == -1 {
        return Err(named(io::Error::last_os_error(), path));
    }
    Ok(lock.l_type != libc::F_UNLCK as libc::c_short)
}

/// A write lock over the whole file.
fn whole() -> libc::flock {
    let mut lock: libc::flock = unsafe { std::mem::zeroed() };
    lock.l_type = libc::F_WRLCK as libc::c_short;
    lock.l_whence = libc::SEEK_SET as libc::c_short;
    lock
}

/// Ends both directions; the read clone sees it too.
pub(crate) fn hang_up(half: Half) -> io::Result<()> {
    half.shutdown(std::net::Shutdown::Both)
}
