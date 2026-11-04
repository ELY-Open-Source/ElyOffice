//! A save: ticket, disk, landing (FILE-03 to FILE-05).

use std::{
    io,
    path::{Path, PathBuf},
};

use super::{SaveError, Saved, atomic_capable, resolved, same_file, save_atomic, save_with_backup, with_path};
use crate::{Fingerprint, SaveState, SaveTicket, Session, Stamp};

/// What the disk must still hold before the write.
enum Check {
    /// The session's file, as last read or written.
    Seen(Option<Fingerprint>),
    /// Save As: as `Seen` when the target is this file.
    Own(Option<PathBuf>, Option<Fingerprint>),
    /// Keep Mine over a gone file: only a new one.
    Gone,
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

/// A reload since the export makes its bytes stale.
fn current(session: &Session, stamp: Stamp) -> Result<(), SaveError> {
    if session.stamp().generation != stamp.generation {
        log::warn!("files: the session reloaded during the export; nothing is written");
        return Err(SaveError::Io(io::Error::other("the file reloaded while the save was prepared")));
    }
    Ok(())
}

impl Write {
    /// A save in place of `stamp`; an outside change conflicts.
    pub fn save(stamp: Stamp, session: &mut Session) -> Result<Write, SaveError> {
        current(session, stamp)?;
        let path = session_path(session)?;
        let seen = session.fingerprint();
        let ticket = session.begin_save().map_err(|state| match state {
            // A known outside change is a conflict.
            SaveState::ExternalModified => SaveError::Conflict,
            state => SaveError::Refused(state),
        })?;
        Ok(Write { ticket: ticket.of(stamp), path, check: Check::Seen(seen) })
    }

    /// Save As to `path`; its own file stays checked.
    pub fn save_as(stamp: Stamp, session: &mut Session, path: &Path) -> Result<Write, SaveError> {
        current(session, stamp)?;
        let (own, seen) = (session.path().map(Path::to_path_buf), session.fingerprint());
        // A new file keeps the local version in a conflict.
        let ticket = match session.state() == SaveState::ExternalModified {
            true => session.begin_save_overwrite(),
            false => session.begin_save(),
        };
        let ticket = ticket.map_err(SaveError::Refused)?;
        session.write_to(path);
        Ok(Write { ticket: ticket.of(stamp), path: path.to_path_buf(), check: Check::Own(own, seen) })
    }

    /// Keep Mine over a gone file; one made meanwhile conflicts.
    pub fn anew(stamp: Stamp, session: &mut Session) -> Result<Write, SaveError> {
        current(session, stamp)?;
        let path = session_path(session)?;
        let ticket = session.begin_save_overwrite().map_err(SaveError::Refused)?;
        Ok(Write { ticket: ticket.of(stamp), path, check: Check::Gone })
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
        let Check::Gone = self.check else {
            return self.put(&target, None, bytes, probe);
        };
        let claimed = claim(&target)?;
        let put = self.put(&target, Some(claimed), bytes, probe);
        // A failed write leaves no empty file of ours behind.
        if put.is_err()
            && Fingerprint::of(&target).ok() == Some(claimed)
            && let Err(error) = std::fs::remove_file(&target)
        {
            log::warn!("files: the empty claim {} stays: {error}", target.display());
        }
        put
    }

    /// Writes `bytes` to `target`; `claimed` is a gone file's claim.
    fn put(
        &self,
        target: &Path,
        claimed: Option<Fingerprint>,
        bytes: &[u8],
        probe: impl FnOnce(&Path) -> io::Result<bool>,
    ) -> Result<Saved, SaveError> {
        let target = target.to_path_buf();
        // Asked first: the check stays next to the write.
        let atomic = probe(&target).map_err(SaveError::Io)?;
        let held = match &self.check {
            Check::Gone => claimed.map(|claimed| (target.clone(), claimed)),
            Check::Seen(seen) => seen.map(|seen| (self.path.clone(), seen)),
            Check::Own(Some(own), Some(seen)) if is_own(own, &target)? => Some((own.clone(), *seen)),
            Check::Own(..) => {
                log::info!("files: save as {}", target.display());
                None
            }
        };
        if let Some((path, seen)) = held
            && Fingerprint::of(&path).map_err(SaveError::Io)? != seen
        {
            log::warn!("files: save to {} meets an external change", target.display());
            return Err(SaveError::Conflict);
        }
        // An empty claim of ours needs no copy kept.
        let backup = claimed.is_none() && !atomic;
        if backup {
            log::warn!("files: {} sits on a non-atomic volume", target.display());
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

/// Takes a gone file's name; every volume honours create-new.
fn claim(target: &Path) -> Result<Fingerprint, SaveError> {
    match std::fs::OpenOptions::new().write(true).create_new(true).open(target) {
        Ok(_) => Fingerprint::of(target).map_err(SaveError::Io),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            log::warn!("files: {} appeared before keep mine wrote", target.display());
            Err(SaveError::Conflict)
        }
        Err(error) => Err(SaveError::Io(with_path(target, error))),
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
    Write::save(session.stamp(), session)?.run(bytes).land(session)
}

#[cfg(test)]
pub(crate) fn save_as(session: &mut Session, path: &Path, bytes: &[u8]) -> Result<Saved, SaveError> {
    Write::save_as(session.stamp(), session, path)?.run(bytes).land(session)
}
