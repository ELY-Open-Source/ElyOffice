use std::{
    io,
    path::{Path, PathBuf},
};

/// What the last session's end looked like (§6.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Marker {
    Clean,
    Crashed,
    Discarded,
}

/// The lock that tells crash from discard.
pub struct Crash {
    file: PathBuf,
}

impl Crash {
    /// Marks the session open under `pid`.
    pub fn open(dir: &Path, pid: u32) -> io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        let file = dir.join("session.lock");
        write_whole(&file, &format!("open {pid}"))?;
        Ok(Self { file })
    }

    /// Reads the lock; a dead pid means a crash (§6.2).
    pub fn check(dir: &Path, alive: impl Fn(u32) -> bool) -> io::Result<Marker> {
        let file = dir.join("session.lock");
        let bytes = match std::fs::read(&file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Marker::Clean),
            Err(error) => return Err(error),
        };
        // Only the two whole forms `open` and `discard` write.
        let text = std::str::from_utf8(&bytes).ok();
        let pid = text
            .and_then(|text| text.strip_prefix("open "))
            .and_then(|pid| pid.parse::<u32>().ok());
        match (text, pid) {
            (Some("discarded 0"), _) => Ok(Marker::Discarded),
            (_, Some(pid)) => Ok(if alive(pid) {
                Marker::Clean
            } else {
                Marker::Crashed
            }),
            // A torn lock means no clean end: offer the points.
            _ => {
                log::warn!("recovery: the session lock is unreadable; treated as a crash");
                Ok(Marker::Crashed)
            }
        }
    }

    /// An explicit discard: never a crash next launch (§6.2).
    pub fn discard(self) -> io::Result<()> {
        write_whole(&self.file, "discarded 0")
    }

    /// A clean close removes the lock.
    pub fn close(self) -> io::Result<()> {
        match std::fs::remove_file(&self.file) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

/// Replaces `file` whole: a power cut leaves old or new.
fn write_whole(file: &Path, text: &str) -> io::Result<()> {
    use std::io::Write as _;
    let mut staging = tempfile::NamedTempFile::new_in(file.parent().unwrap_or(Path::new(".")))?;
    staging.write_all(text.as_bytes())?;
    staging.as_file().sync_all()?;
    staging.persist(file).map(drop).map_err(|error| error.error)
}

/// Whether process `pid` still runs; a crash leaves none.
#[cfg(unix)]
pub fn alive(pid: u32) -> bool {
    let Ok(pid) = i32::try_from(pid) else {
        return false;
    };
    // Another user's live process answers EPERM.
    let answered = unsafe { libc::kill(pid, 0) } == 0;
    answered || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// Whether process `pid` still runs; a crash leaves none.
#[cfg(windows)]
pub fn alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return false;
    }
    let mut code = 0;
    let read = unsafe { GetExitCodeProcess(handle, &mut code) } != 0;
    unsafe { CloseHandle(handle) };
    read && code == STILL_ACTIVE as u32
}

#[cfg(all(test, unix))]
mod alive_tests {
    #[test]
    fn this_process_lives_a_reaped_one_does_not() {
        assert!(super::alive(std::process::id()));
        let mut child = std::process::Command::new("true").spawn().expect("a child");
        let pid = child.id();
        child.wait().expect("the child ends");
        assert!(!super::alive(pid));
    }
}
