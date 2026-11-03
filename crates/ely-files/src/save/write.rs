//! A save: ticket, disk, landing (FILE-03 to FILE-05).

use std::{
    io,
    path::{Path, PathBuf},
};

use super::{SaveError, Saved, atomic_capable, resolved, same_file, save_atomic, save_with_backup, with_path};
use crate::{Fingerprint, SaveState, SaveTicket, Session};

/// What the disk must still hold before the write.
enum Check {
    /// The session's file, as last read or written.
    Seen(Option<Fingerprint>),
    /// Save As: as `Seen` when the target is this file.
    Own(Option<PathBuf>, Option<Fingerprint>),
    /// Keep Mine: whatever is there is replaced.
    Over,
}

/// A save begun on its session; `run` touches the disk.
pub struct Write {
    ticket: SaveTicket,
    path: PathBuf,
    check: Check,
}

/// A finished write, to land on its session.
pub struct Done {
    ticket: SaveTicket,
    /// What reached the disk; the session has not heard yet.
    pub saved: Result<Saved, SaveError>,
}

fn session_path(session: &Session) -> Result<PathBuf, SaveError> {
    session
        .path()
        .map(Path::to_path_buf)
        .ok_or_else(|| SaveError::Io(io::Error::new(io::ErrorKind::NotFound, "no path to save")))
}

impl Write {
    /// A save in place; an outside change is a conflict.
    pub fn save(session: &mut Session) -> Result<Write, SaveError> {
        let path = session_path(session)?;
        let seen = session.fingerprint();
        let ticket = session.begin_save().map_err(|state| match state {
            // A known outside change is a conflict.
            SaveState::ExternalModified => SaveError::Conflict,
            state => SaveError::Refused(state),
        })?;
        Ok(Write { ticket, path, check: Check::Seen(seen) })
    }

    /// Save As to `path`; its own file stays checked.
    pub fn save_as(session: &mut Session, path: &Path) -> Result<Write, SaveError> {
        let (own, seen) = (session.path().map(Path::to_path_buf), session.fingerprint());
        // A new file keeps the local version in a conflict.
        let ticket = match session.state() == SaveState::ExternalModified {
            true => session.begin_save_overwrite(),
            false => session.begin_save(),
        };
        let ticket = ticket.map_err(SaveError::Refused)?;
        session.write_to(path);
        Ok(Write { ticket, path: path.to_path_buf(), check: Check::Own(own, seen) })
    }

    /// Keep Mine in a conflict: over the changed file.
    pub fn overwrite(session: &mut Session) -> Result<Write, SaveError> {
        let path = session_path(session)?;
        let ticket = session.begin_save_overwrite().map_err(SaveError::Refused)?;
        Ok(Write { ticket, path, check: Check::Over })
    }

    /// Checks the disk, then writes `bytes`; off the UI thread.
    pub fn run(self, bytes: &[u8]) -> Done {
        self.run_with(bytes, atomic_capable)
    }

    /// As `run`, the volume probe handed in (§6.1).
    pub fn run_with(self, bytes: &[u8], probe: impl FnOnce(&Path) -> io::Result<bool>) -> Done {
        let saved = self.write(bytes, probe);
        Done { ticket: self.ticket, saved }
    }

    fn write(&self, bytes: &[u8], probe: impl FnOnce(&Path) -> io::Result<bool>) -> Result<Saved, SaveError> {
        let target = resolved(&self.path).map_err(SaveError::Io)?;
        let backup = !probe(&target).map_err(SaveError::Io)?;
        if backup {
            log::warn!("files: {} sits on a non-atomic volume", target.display());
        }
        let held = match &self.check {
            Check::Seen(seen) => seen.map(|seen| (self.path.clone(), seen)),
            Check::Own(Some(own), Some(seen)) if is_own(own, &target)? => Some((own.clone(), *seen)),
            Check::Own(..) => {
                log::info!("files: save as {}", target.display());
                None
            }
            Check::Over => None,
        };
        if let Some((path, seen)) = held
            && Fingerprint::of(&path).map_err(SaveError::Io)? != seen
        {
            log::warn!("files: save to {} meets an external change", target.display());
            return Err(SaveError::Conflict);
        }
        let revision = self.ticket.revision();
        let (written, backup) = match backup {
            true => save_with_backup(&target, bytes, revision),
            false => save_atomic(&target, bytes, revision).map(|written| (written, None)),
        }
        .map_err(SaveError::Io)?;
        // Writes keep `\\?\`; the session shows plain.
        let path = dunce::simplified(&target).to_path_buf();
        Ok(Saved { path, written, backup })
    }
}

/// Whether Save As writes over the session's own file.
fn is_own(own: &Path, target: &Path) -> Result<bool, SaveError> {
    // A file moved or deleted outside is written anew.
    match own.try_exists().map_err(|error| SaveError::Io(with_path(own, error)))? {
        true => same_file(own, target).map_err(SaveError::Io),
        false => Ok(false),
    }
}

impl Done {
    /// The session hears how its save ended.
    pub fn land(self, session: &mut Session) -> Result<Saved, SaveError> {
        match self.saved {
            Ok(saved) => {
                session.finish_save(self.ticket, saved.path.clone(), saved.written).map_err(|_| {
                    SaveError::Io(io::Error::new(io::ErrorKind::InvalidData, "stale save ticket"))
                })?;
                Ok(saved)
            }
            Err(SaveError::Conflict) => {
                if session.conflict(self.ticket).is_err() {
                    log::error!("files: the conflict met an unknown ticket");
                }
                Err(SaveError::Conflict)
            }
            Err(error) => {
                if session.fail_save(self.ticket).is_err() {
                    log::error!("files: the failed save left an unknown ticket");
                }
                Err(error)
            }
        }
    }
}

/// A save begun, written and landed at once.
#[cfg(test)]
pub(crate) fn save(session: &mut Session, bytes: &[u8]) -> Result<Saved, SaveError> {
    Write::save(session)?.run(bytes).land(session)
}

#[cfg(test)]
pub(crate) fn save_as(session: &mut Session, path: &Path, bytes: &[u8]) -> Result<Saved, SaveError> {
    Write::save_as(session, path)?.run(bytes).land(session)
}

#[cfg(test)]
pub(crate) fn save_overwrite(session: &mut Session, bytes: &[u8]) -> Result<Saved, SaveError> {
    Write::overwrite(session)?.run(bytes).land(session)
}
