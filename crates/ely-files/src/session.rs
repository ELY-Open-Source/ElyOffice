use std::{
    fmt, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

/// Length, mtime and identity when last loaded or saved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fingerprint {
    pub len: u64,
    pub modified: SystemTime,
    /// A replace with the same time is a new file.
    pub id: Option<(u64, u64)>,
}

impl Fingerprint {
    /// Errors name `path`; their kind stays.
    pub fn of(path: &Path) -> io::Result<Self> {
        let read = || -> io::Result<Self> {
            let meta = std::fs::metadata(path)?;
            Ok(Self {
                len: meta.len(),
                modified: meta.modified()?,
                id: file_id(path)?,
            })
        };
        read().map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", path.display())))
    }
}

/// The save states a session shows (FILE-02).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveState {
    Loading,
    LoadFailed,
    Saved,
    Dirty,
    Saving,
    ReadOnly,
    ExternalModified,
    SaveFailed,
}

impl fmt::Display for SaveState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            SaveState::Loading => "loading",
            SaveState::LoadFailed => "load-failed",
            SaveState::Saved => "saved",
            SaveState::Dirty => "dirty",
            SaveState::Saving => "saving",
            SaveState::ReadOnly => "read-only",
            SaveState::ExternalModified => "external-modified",
            SaveState::SaveFailed => "save-failed",
        };
        f.write_str(name)
    }
}

/// A save in flight: its task, generation and revision (FILE-03).
#[derive(Clone, Copy, Debug)]
pub struct SaveTicket {
    task: u64,
    generation: u64,
    revision: u64,
}

/// A ticket from before a reload; its content is obsolete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StaleTicket;

/// One open document and its session data (§6.1).
pub struct Session {
    path: Option<PathBuf>,
    fingerprint: Option<Fingerprint>,
    generation: u64,
    revision: u64,
    saved_revision: u64,
    next_task: u64,
    state: SaveState,
    external_unresolved: bool,
    loading: bool,
    loaded: bool,
    saving: Option<SaveTicket>,
    pub cursor: Option<String>,
    pub zoom: f32,
    recovery: Option<PathBuf>,
}

impl Session {
    /// A brand-new document: no target path until first saved (FILE-01).
    pub fn new_file() -> Self {
        Self {
            path: None,
            fingerprint: None,
            generation: 0,
            revision: 0,
            saved_revision: 0,
            next_task: 0,
            state: SaveState::Saved,
            external_unresolved: false,
            loading: false,
            loaded: true,
            saving: None,
            cursor: None,
            zoom: 1.0,
            recovery: None,
        }
    }

    /// A document the kernel still loads asynchronously.
    pub fn loading(path: PathBuf) -> Self {
        Self {
            state: SaveState::Loading,
            loading: true,
            loaded: false,
            ..Self::new_file()
        }
        .with_path(path)
    }

    fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Loaded from disk; the fingerprint is what was read.
    pub fn load(path: PathBuf) -> io::Result<Self> {
        let fingerprint = Fingerprint::of(&path)?;
        Ok(Self {
            fingerprint: Some(fingerprint),
            ..Self::new_file()
        }
        .with_path(path))
    }

    /// A read-only view: it never saves.
    pub fn read_only(path: PathBuf) -> io::Result<Self> {
        let mut session = Self::load(path)?;
        session.state = SaveState::ReadOnly;
        Ok(session)
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The file moved to `path`; its contents did not change.
    pub fn renamed(&mut self, path: PathBuf) {
        self.path = Some(path);
    }

    pub fn fingerprint(&self) -> Option<Fingerprint> {
        self.fingerprint
    }

    /// Whether the file on disk differs from our fingerprint (FILE-05).
    pub fn external_changed(&self) -> io::Result<bool> {
        let (Some(path), Some(fingerprint)) = (self.path(), self.fingerprint()) else {
            return Ok(false);
        };
        Ok(Fingerprint::of(path)? != fingerprint)
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn saved_revision(&self) -> u64 {
        self.saved_revision
    }

    pub fn state(&self) -> SaveState {
        self.state
    }

    pub fn recovery(&self) -> Option<&Path> {
        self.recovery.as_deref()
    }

    pub fn set_recovery(&mut self, location: PathBuf) {
        self.recovery = Some(location);
    }

    /// Confirms the actually loaded content; a conflict outlives it.
    pub fn finish_load(&mut self, loaded: Fingerprint) -> Result<(), StaleTicket> {
        if !self.loading {
            log::warn!("session: a stale load callback is dropped");
            return Err(StaleTicket);
        }
        self.fingerprint = Some(loaded);
        self.loading = false;
        self.loaded = true;
        self.state = if self.external_unresolved {
            SaveState::ExternalModified
        } else {
            SaveState::Saved
        };
        log::info!("session: loaded at {} bytes", loaded.len);
        Ok(())
    }

    /// The load failed; nothing is editable yet.
    pub fn fail_load(&mut self) -> Result<(), StaleTicket> {
        if !self.loading {
            log::warn!("session: a stale load failure is dropped");
            return Err(StaleTicket);
        }
        let failed_path = self.path().unwrap_or(Path::new("")).display().to_string();
        log::warn!("session: load failed for {failed_path}");
        self.loading = false;
        self.loaded = false;
        self.state = SaveState::LoadFailed;
        Ok(())
    }

    /// An edit bumps the revision; a conflict stays shown (FILE-02).
    pub fn edit(&mut self) {
        self.revision += 1;
        if self.state == SaveState::Saved {
            self.state = SaveState::Dirty;
        }
    }

    /// Undo reached the saved content; other states stay (FILE-02).
    pub fn reverted(&mut self) {
        if self.state == SaveState::Dirty {
            log::info!("session: back to the saved content");
            self.state = SaveState::Saved;
        }
    }

    /// Starts a save; read-only, loading and conflicted states refuse.
    pub fn begin_save(&mut self) -> Result<SaveTicket, SaveState> {
        self.ready_to_save(false)
    }

    /// The user chose keep-local in a conflict; overwrite anyway.
    pub fn begin_save_overwrite(&mut self) -> Result<SaveTicket, SaveState> {
        let ticket = self.ready_to_save(true)?;
        log::info!("session: keep-local overwrites the external change");
        self.external_unresolved = false;
        Ok(ticket)
    }

    /// Shared guards: loading, saving, unloaded, read-only, conflict.
    fn ready_to_save(&mut self, overwrite: bool) -> Result<SaveTicket, SaveState> {
        if self.loading {
            log::warn!("session: a load is still running");
            return Err(SaveState::Loading);
        }
        if self.saving.is_some() {
            log::warn!("session: a save is already running");
            return Err(SaveState::Saving);
        }
        if !self.loaded {
            log::warn!("session: nothing loaded yet to save");
            return Err(SaveState::LoadFailed);
        }
        if self.state == SaveState::ReadOnly {
            log::warn!("session: a read-only view cannot save");
            return Err(self.state);
        }
        if !overwrite && self.external_unresolved {
            log::warn!("session: an unresolved conflict refuses a plain save");
            return Err(SaveState::ExternalModified);
        }
        Ok(self.ticket())
    }

    fn ticket(&mut self) -> SaveTicket {
        self.state = SaveState::Saving;
        let ticket = SaveTicket {
            task: self.next_task,
            generation: self.generation,
            revision: self.revision,
        };
        self.next_task += 1;
        self.saving = Some(ticket);
        ticket
    }

    /// Confirms the written revision; a conflict outlives it (FILE-03).
    pub fn finish_save(
        &mut self,
        ticket: SaveTicket,
        path: PathBuf,
        written: Fingerprint,
    ) -> Result<(), StaleTicket> {
        let Some(running) = self.saving else {
            log::warn!("session: no save is running for {}", path.display());
            return Err(StaleTicket);
        };
        if running.task != ticket.task {
            log::warn!(
                "session: an unknown ticket for {} is dropped",
                path.display()
            );
            return Err(StaleTicket);
        }
        self.saving = None;
        if ticket.generation != self.generation {
            log::warn!("session: a stale ticket for {} is dropped", path.display());
            return Err(StaleTicket);
        }
        log::info!(
            "session: saved revision {} to {}",
            ticket.revision,
            path.display()
        );
        self.path = Some(path);
        self.fingerprint = Some(written);
        self.saved_revision = ticket.revision;
        self.state = if self.external_unresolved {
            SaveState::ExternalModified
        } else if self.saved_revision == self.revision {
            SaveState::Saved
        } else {
            SaveState::Dirty
        };
        Ok(())
    }

    /// A failed save keeps the file and the unsaved content.
    pub fn fail_save(&mut self, ticket: SaveTicket) -> Result<(), StaleTicket> {
        if self.saving.map(|running| running.task) != Some(ticket.task) {
            log::warn!("session: a failure for an unknown save is dropped");
            return Err(StaleTicket);
        }
        self.saving = None;
        if ticket.generation != self.generation {
            log::warn!("session: a stale save failure is dropped");
            return Err(StaleTicket);
        }
        log::warn!("session: save failed at revision {}", self.revision);
        self.state = SaveState::SaveFailed;
        Ok(())
    }

    /// A save that failed before any write (FILE-02).
    pub fn save_failed(&mut self) {
        if self.saving.is_none() && matches!(self.state, SaveState::Saved | SaveState::Dirty) {
            log::warn!("session: a save failed before writing");
            self.state = SaveState::SaveFailed;
        }
    }

    /// The file changed on disk since our fingerprint.
    pub fn external_modified(&mut self) {
        if self.state == SaveState::ReadOnly {
            return;
        }
        self.external_unresolved = true;
        self.state = SaveState::ExternalModified;
    }

    /// Reloaded from disk; the access mode survives, tickets die.
    pub fn reload(&mut self) -> io::Result<()> {
        let Some(path) = self.path.clone() else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "no path to reload"));
        };
        self.reload_at(Fingerprint::of(&path)?)
    }

    /// The disk at `fingerprint` is seen; edits stay unsaved.
    pub fn rebase(&mut self, fingerprint: Fingerprint) {
        self.fingerprint = Some(fingerprint);
        self.external_unresolved = false;
        if self.state != SaveState::ReadOnly {
            self.state = SaveState::Dirty;
        }
        log::info!("session: the disk version is seen; local edits stay");
    }

    /// Reloaded from bytes read at `fingerprint`, not disk now.
    pub fn reload_at(&mut self, fingerprint: Fingerprint) -> io::Result<()> {
        let Some(path) = self.path.clone() else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "no path to reload"));
        };
        self.fingerprint = Some(fingerprint);
        self.generation += 1;
        self.revision = 0;
        self.saved_revision = 0;
        self.external_unresolved = false;
        self.loading = false;
        self.loaded = true;
        if self.state != SaveState::ReadOnly {
            self.state = SaveState::Saved;
        }
        log::info!("session: reloaded {}", path.display());
        Ok(())
    }
}

mod registry;
pub(crate) use registry::file_id;
pub use registry::{Open, Sessions};

#[cfg(test)]
mod tests;
