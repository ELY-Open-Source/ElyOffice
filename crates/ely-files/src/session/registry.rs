use std::{
    io,
    path::{Path, PathBuf},
};

/// A file's device and index.
type FileId = (u64, u64);

/// The app's open sessions (§6.1).
#[derive(Default)]
pub struct Sessions {
    /// Path, file identity when opened, session id.
    open: Vec<(PathBuf, Option<FileId>, usize)>,
    next: usize,
}

/// What opening a path did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Open {
    /// A new session id.
    Fresh(usize),
    /// The path is already open; focus that session.
    Focus(usize),
    /// A new read-only view id beside the existing session.
    View(usize),
}

/// The dedup key: canonical path plus file identity.
#[derive(Clone, Debug)]
pub struct Key {
    canonical: PathBuf,
    id: Option<(u64, u64)>,
}

#[cfg(unix)]
pub(crate) fn file_id(path: &Path) -> io::Result<Option<(u64, u64)>> {
    use std::os::unix::fs::MetadataExt;
    let meta = std::fs::metadata(path)?;
    Ok(Some((meta.dev(), meta.ino())))
}

#[cfg(windows)]
pub(crate) fn file_id(path: &Path) -> io::Result<Option<(u64, u64)>> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE, GetFileInformationByHandle, OPEN_EXISTING,
    };
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetFileInformationByHandle(handle, &mut info) };
    unsafe { CloseHandle(handle) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Some((
        info.dwVolumeSerialNumber as u64,
        ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64,
    )))
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn file_id(_: &Path) -> io::Result<Option<(u64, u64)>> {
    Ok(None)
}

impl Key {
    /// The file's real path.
    pub fn path(&self) -> &Path {
        &self.canonical
    }
}

impl Sessions {
    /// The key of an existing file; missing fails loud.
    pub fn key(path: &Path) -> io::Result<Key> {
        Ok(Key {
            canonical: dunce::canonicalize(path)?,
            id: file_id(path)?,
        })
    }

    /// A match by path, identity, or a moved file's.
    fn matches(have: &Path, had: Option<FileId>, key: &Key) -> io::Result<bool> {
        if *have == key.canonical {
            return Ok(true);
        }
        match file_id(have) {
            Ok(Some(a)) => Ok(key.id == Some(a)),
            Ok(None) => Ok(false),
            // Gone from its path: the same file renamed.
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Ok(had.is_some() && key.id == had)
            }
            Err(error) => Err(error),
        }
    }

    /// The first live match, else the first identity error.
    fn focus(&self, key: &Key) -> io::Result<Option<usize>> {
        let mut error = None;
        for (have, had, id) in &self.open {
            match Self::matches(have, *had, key) {
                Ok(true) => return Ok(Some(*id)),
                Ok(false) => {}
                Err(current) => {
                    if error.is_none() {
                        error = Some(current);
                    }
                }
            }
        }
        match error {
            Some(current) => Err(current),
            None => Ok(None),
        }
    }

    /// A second open focuses, or views read-only.
    pub fn open(&mut self, path: &Path, read_only: bool) -> io::Result<Open> {
        let key = Self::key(path)?;
        let found = self.focus(&key)?;
        if let Some(id) = found
            && !read_only
        {
            log::info!("sessions: {path:?} is open as {id}; focusing it");
            return Ok(Open::Focus(id));
        }
        let id = self.next;
        self.next += 1;
        self.open.push((key.canonical.clone(), key.id, id));
        if found.is_some() {
            log::info!("sessions: {path:?} also opens as read-only view {id}");
            return Ok(Open::View(id));
        }
        Ok(Open::Fresh(id))
    }

    /// Drops a closed session's record.
    pub fn close(&mut self, id: usize) {
        self.open.retain(|(_, _, have)| *have != id);
    }

    /// Session `id` now names the file at `path`.
    pub fn track(&mut self, id: usize, key: Key) -> io::Result<Result<(), usize>> {
        // Another session holding it keeps it alone.
        if let Some(other) = self.holders(&key.canonical, key.id)?.into_iter().find(|other| *other != id) {
            return Ok(Err(other));
        }
        for entry in self.open.iter_mut().filter(|(_, _, have)| *have == id) {
            (entry.0, entry.1) = (key.canonical.clone(), key.id);
        }
        Ok(Ok(()))
    }

    /// The session of what `saved` wrote, else new.
    pub fn adopt(&mut self, saved: &crate::Saved) -> io::Result<Open> {
        if let Some(id) = self.held(&saved.path, saved.written.id)? {
            return Ok(Open::Focus(id));
        }
        let id = self.next;
        self.next += 1;
        self.open.push((saved.path.clone(), saved.written.id, id));
        Ok(Open::Fresh(id))
    }

    /// The session holding `path`; only `path` is read.
    pub fn holding(&self, path: &Path) -> io::Result<Option<usize>> {
        match Self::key(path) {
            Ok(key) => self.held(&key.canonical, key.id),
            // Missing mid-replace: its recorded path still holds it.
            Err(error) if error.kind() == io::ErrorKind::NotFound => self.held(path, None),
            Err(error) => Err(error),
        }
    }

    /// A recorded path or file identity, matched in memory.
    fn held(&self, canonical: &Path, id: Option<FileId>) -> io::Result<Option<usize>> {
        Ok(self.holders(canonical, id)?.first().copied())
    }

    /// Paths match as their folder compares names.
    fn holders(&self, canonical: &Path, id: Option<FileId>) -> io::Result<Vec<usize>> {
        let mut found = Vec::new();
        for (have, had, session) in &self.open {
            if (id.is_some() && *had == id) || crate::same_name(have, canonical)? {
                found.push(*session);
            }
        }
        Ok(found)
    }

    /// Session `id`'s path now holds file `file`, as read.
    pub fn replaced(&mut self, id: usize, file: Option<FileId>) {
        for entry in self.open.iter_mut().filter(|(_, _, have)| *have == id) {
            entry.1 = file;
        }
    }

    /// Session `id` names what `saved` wrote; no disk read.
    pub fn saved(&mut self, id: usize, saved: &crate::Saved) {
        for entry in self.open.iter_mut().filter(|(_, _, have)| *have == id) {
            (entry.0, entry.1) = (saved.path.clone(), saved.written.id);
        }
    }

    /// The file identity session `id` last recorded.
    pub fn identity_of(&self, id: usize) -> Option<FileId> {
        self.open.iter().find(|(_, _, have)| *have == id).and_then(|(_, had, _)| *had)
    }

    /// The entry of `dir` that is file `had`, renamed there.
    pub fn renamed_in(had: FileId, dir: &Path) -> io::Result<Option<PathBuf>> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            // A link to the file is not the file.
            if entry.file_type()?.is_symlink() {
                continue;
            }
            match file_id(&entry.path()) {
                Ok(found) if found == Some(had) => return Ok(Some(entry.path())),
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(None)
    }

    pub fn is_open(&self, path: &Path) -> io::Result<bool> {
        let key = Self::key(path)?;
        Ok(self.focus(&key)?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::{Open, Sessions};

    #[test]
    fn a_renamed_file_stays_one_session() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let (a, b) = (dir.path().join("a.md"), dir.path().join("b.md"));
        std::fs::write(&a, "a").expect("writes");
        let mut sessions = Sessions::default();
        let Open::Fresh(id) = sessions.open(&a, false).expect("opens") else {
            panic!("a first open is fresh");
        };
        std::fs::rename(&a, &b).expect("renames");
        // Opened in the gap: the same file, so focus.
        assert_eq!(sessions.open(&b, false).expect("opens"), Open::Focus(id));
        sessions.track(id, Sessions::key(&b).expect("the key")).expect("compares").expect("moves");
        assert_eq!(sessions.open(&b, false).expect("opens"), Open::Focus(id));
        sessions.close(id);
        assert!(matches!(
            sessions.open(&b, false).expect("opens"),
            Open::Fresh(_)
        ));
    }

    #[test]
    fn a_tracked_save_keeps_a_later_rename_one_session() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let (a, b, temp) = (
            dir.path().join("a.md"),
            dir.path().join("b.md"),
            dir.path().join("t"),
        );
        std::fs::write(&a, "a").expect("writes");
        let mut sessions = Sessions::default();
        let Open::Fresh(id) = sessions.open(&a, false).expect("opens") else {
            panic!("a first open is fresh");
        };
        // An atomic save: a new file takes the name.
        std::fs::write(&temp, "saved").expect("writes");
        std::fs::rename(&temp, &a).expect("replaces");
        sessions.track(id, Sessions::key(&a).expect("the key")).expect("compares").expect("tracks");
        std::fs::rename(&a, &b).expect("renames");
        assert_eq!(sessions.open(&b, false).expect("opens"), Open::Focus(id));
    }
}
