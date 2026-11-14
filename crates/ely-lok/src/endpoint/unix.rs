//! Unix: a socket in a private folder.

use std::{
    fs::{self, DirBuilder, Permissions},
    io::{self, Write},
    os::unix::{
        fs::{DirBuilderExt, MetadataExt, PermissionsExt},
        net::{UnixListener, UnixStream},
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

/// A bound socket, waiting for its one app.
pub(crate) struct Listener {
    socket: UnixListener,
    path: PathBuf,
}

impl Listener {
    pub(crate) fn bind(name: &str) -> io::Result<Self> {
        let path = path(name)?;
        let socket = UnixListener::bind(&path).map_err(|error| named(error, &path))?;
        fs::set_permissions(&path, Permissions::from_mode(0o600)).map_err(|error| named(error, &path))?;
        log::info!("lok-worker: listening on {}", path.display());
        Ok(Self { socket, path })
    }

    /// The first app; its name goes, no one else connects.
    pub(crate) fn accept(self) -> io::Result<(Half, Half)> {
        let (stream, _) = self.socket.accept()?;
        fs::remove_file(&self.path).map_err(|error| named(error, &self.path))?;
        Ok((stream.try_clone()?, stream))
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
pub(crate) fn connect(name: &str, _pid: u32) -> io::Result<(Half, Half)> {
    let path = path(name)?;
    let stream = UnixStream::connect(&path).map_err(|error| named(error, &path))?;
    Ok((stream.try_clone()?, stream))
}
