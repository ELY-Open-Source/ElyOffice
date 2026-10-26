use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

use crate::{Fingerprint, SaveState, Session};

#[cfg(test)]
mod tests;
#[cfg(all(test, target_os = "macos"))]
mod full;

/// The largest name the filesystem takes.
const NAME_MAX: usize = 255;

/// `error` with the path in its message.
fn with_path(path: &Path, error: io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{}: {error}", path.display()))
}

/// The directory a path lives in (bare names: `.`).
fn parent_dir(path: &Path) -> &Path {
    match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    }
}

/// `stem` capped so `suffix` survives the name limit (FILE-09).
fn with_suffix(stem: &str, suffix: &str) -> String {
    take_bytes(stem, NAME_MAX.saturating_sub(suffix.len())) + suffix
}

/// `text` taken head-first within `max` bytes, on char edges.
fn take_bytes(text: &str, max: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let len = ch.len_utf8();
        if used + len > max {
            break;
        }
        out.push(ch);
        used += len;
    }
    out
}

/// A collision name: the counter always survives.
fn numbered_name(stem: &str, counter: u32, ext: &str) -> String {
    let middle = format!("-{counter}");
    let ext = take_bytes(ext, 32);
    let dot = if ext.is_empty() { "" } else { "." };
    let room = NAME_MAX.saturating_sub(middle.len() + dot.len() + ext.len());
    format!("{}{middle}{dot}{ext}", take_bytes(stem, room))
}

/// Writes `bytes` beside `path`, checks the write, then replaces (FILE-04).
pub fn save_atomic(path: &Path, bytes: &[u8], revision: u64) -> io::Result<Fingerprint> {
    let target = resolved(path)?;
    let mut staging =
        tempfile::NamedTempFile::new_in(parent_dir(&target)).map_err(|e| with_path(path, e))?;
    staging.write_all(bytes).map_err(|e| with_path(path, e))?;
    staging
        .as_file()
        .sync_all()
        .map_err(|e| with_path(path, e))?;
    if let Ok(meta) = std::fs::metadata(&target) {
        staging
            .as_file()
            .set_permissions(meta.permissions())
            .map_err(|e| with_path(path, e))?;
        copy_acl(&target, staging.path()).map_err(|e| with_path(path, e))?;
    }
    // tempfile's Windows persist cannot replace a file another reader holds.
    let staged = staging
        .into_temp_path()
        .keep()
        .map_err(|error| with_path(path, error.error))?;
    if let Err(error) = std::fs::rename(&staged, &target) {
        if let Err(left) = std::fs::remove_file(&staged) {
            log::warn!("files: the staged copy {} stays: {left}", staged.display());
        }
        return Err(with_path(path, error));
    }
    log::info!("files: saved revision {revision} to {}", path.display());
    Fingerprint::of(&target)
}

/// The access-control entries of an existing target (macOS).
#[cfg(target_os = "macos")]
fn copy_acl(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    const COPYFILE_ACL: u32 = 1 << 0;
    let cstring = |path: &Path| {
        std::ffi::CString::new(path.as_os_str().as_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "the path holds a NUL"))
    };
    let (from, to) = (cstring(from)?, cstring(to)?);
    if unsafe {
        libc::copyfile(
            from.as_ptr(),
            to.as_ptr(),
            std::ptr::null_mut(),
            COPYFILE_ACL,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn copy_acl(_: &Path, _: &Path) -> io::Result<()> {
    Ok(())
}

/// Non-atomic targets keep the previous file aside (§6.1).
pub fn save_with_backup(
    path: &Path,
    bytes: &[u8],
    revision: u64,
) -> io::Result<(Fingerprint, Option<PathBuf>)> {
    let backup = if path.try_exists().map_err(|e| with_path(path, e))? {
        let (name, mut file) = fresh_backup(path)?;
        let mut source = std::fs::File::open(path).map_err(|e| with_path(path, e))?;
        io::copy(&mut source, &mut file).map_err(|e| with_path(&name, e))?;
        file.sync_all().map_err(|e| with_path(&name, e))?;
        log::info!("files: the previous file stays at {}", name.display());
        Some(name)
    } else {
        None
    };
    let written = save_atomic(path, bytes, revision)?;
    Ok((written, backup))
}

/// An exclusive fresh backup file with the source's access limits.
fn fresh_backup(path: &Path) -> io::Result<(PathBuf, std::fs::File)> {
    let name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} has no file name", path.display()),
        )
    })?;
    for n in 0u32.. {
        let suffix = if n == 0 {
            ".bak".to_string()
        } else {
            format!("-{n}.bak")
        };
        let candidate = parent_dir(path).join(with_suffix(&name.to_string_lossy(), &suffix));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                let meta = std::fs::metadata(path).map_err(|e| with_path(path, e))?;
                file.set_permissions(meta.permissions())
                    .map_err(|e| with_path(&candidate, e))?;
                copy_acl(path, &candidate).map_err(|e| with_path(&candidate, e))?;
                return Ok((candidate, file));
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(with_path(&candidate, error)),
        }
    }
    unreachable!("u32 backup names never run out")
}

/// An export never overwrites the session's file (FILE-06).
pub fn export(
    path: &Path,
    session: Option<&Path>,
    bytes: &[u8],
    revision: u64,
) -> io::Result<Fingerprint> {
    if let Some(open) = session
        && same_file(open, path)?
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("export would overwrite the open file {}", path.display()),
        ));
    }
    log::info!("files: exported revision {revision} to {}", path.display());
    save_atomic(path, bytes, revision)
}

/// The canonical form of a possibly missing path.
fn resolved(path: &Path) -> io::Result<PathBuf> {
    match std::fs::canonicalize(path) {
        Ok(real) => Ok(real),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let name = path.file_name().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{} has no file name", path.display()),
                )
            })?;
            Ok(std::fs::canonicalize(parent_dir(path))?.join(name))
        }
        Err(error) => Err(error),
    }
}

/// Whether two paths name one file, links resolved.
pub fn same_file(a: &Path, b: &Path) -> io::Result<bool> {
    if resolved(a)? == resolved(b)? {
        return Ok(true);
    }
    // A missing side is another file.
    if let (Ok(Some(ia)), Ok(Some(ib))) = (crate::session::file_id(a), crate::session::file_id(b)) {
        return Ok(ia == ib);
    }
    Ok(false)
}

/// Copies an attachment in; the source is never touched (FILE-07).
pub fn import_attachment(source: &Path, dir: &Path) -> io::Result<PathBuf> {
    let name = source.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} has no file name", source.display()),
        )
    })?;
    let mut from = std::fs::File::open(source).map_err(|e| with_path(source, e))?;
    let target = attach(dir, Path::new(name), |to| io::copy(&mut from, to).map(drop))?;
    log::info!("files: imported {} to {}", source.display(), target.display());
    Ok(target)
}

/// Writes `bytes` into `dir` as `name`, or a free variant.
pub fn write_attachment(name: &str, bytes: &[u8], dir: &Path) -> io::Result<PathBuf> {
    let target = attach(dir, Path::new(name), |to| to.write_all(bytes))?;
    log::info!("files: wrote {} bytes to {}", bytes.len(), target.display());
    Ok(target)
}

/// One file name that stays inside its folder.
fn plain_name(name: &Path) -> bool {
    let mut parts = name.components();
    let single = matches!((parts.next(), parts.next()), (Some(std::path::Component::Normal(_)), None));
    single && !(cfg!(windows) && windows_special(&name.to_string_lossy()))
}

/// A stream suffix or a device name on Windows.
fn windows_special(name: &str) -> bool {
    let base = name.split('.').next().unwrap_or_default().trim_end().to_ascii_uppercase();
    let numbered = |prefix: &str| {
        base.strip_prefix(prefix)
            .is_some_and(|n| n.len() == 1 && n.as_bytes()[0].is_ascii_digit() && n != "0")
    };
    name.contains(':') || ["CON", "PRN", "AUX", "NUL"].contains(&base.as_str()) || numbered("COM") || numbered("LPT")
}

/// Claims a free name like `name`, then fills it.
fn attach(
    dir: &Path,
    name: &Path,
    fill: impl FnOnce(&mut std::fs::File) -> io::Result<()>,
) -> io::Result<PathBuf> {
    if !plain_name(name) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a plain file name", name.display()),
        ));
    }
    std::fs::create_dir_all(dir).map_err(|e| with_path(dir, e))?;
    let stem = name.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let ext = name
        .extension()
        .map(|ext| ext.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut target = dir.join(name);
    let mut counter = 1;
    // Claims the name; another writer's file is never replaced.
    let mut to = loop {
        match std::fs::File::create_new(&target) {
            Ok(file) => break file,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                target = dir.join(numbered_name(&stem, counter, &ext));
                counter += 1;
            }
            Err(e) => return Err(with_path(&target, e)),
        }
    };
    let filled = fill(&mut to).and_then(|()| to.flush());
    if let Err(e) = filled {
        // A half copy is not left behind.
        std::fs::remove_file(&target).ok();
        return Err(with_path(&target, e));
    }
    Ok(target)
}

/// Why a save did not land (FILE-05).
#[derive(Debug)]
pub enum SaveError {
    Conflict,
    Refused(SaveState),
    Io(io::Error),
}

/// What a completed save left behind.
pub struct Saved {
    pub path: PathBuf,
    pub written: Fingerprint,
    /// Some on non-atomic volumes; the previous file lives there.
    pub backup: Option<PathBuf>,
}

/// The whole save flow: conflict check, ticket, write, confirm (FILE-04/05).
pub fn save(session: &mut Session, bytes: &[u8]) -> Result<Saved, SaveError> {
    save_with_probe(session, bytes, atomic_capable)
}

/// The save flow with the volume probe handed in (§6.1).
pub fn save_with_probe(
    session: &mut Session,
    bytes: &[u8],
    probe: impl FnOnce(&Path) -> io::Result<bool>,
) -> Result<Saved, SaveError> {
    let Some(path) = session.path().map(Path::to_path_buf) else {
        return Err(SaveError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "no path to save",
        )));
    };
    let target = resolved(&path).map_err(SaveError::Io)?;
    let backup = !probe(&target).map_err(SaveError::Io)?;
    if backup {
        log::warn!("files: {} sits on a non-atomic volume", target.display());
    }
    save_mode(session, &target, bytes, backup)
}

/// The save flow with the volume's capability decided.
fn save_mode(
    session: &mut Session,
    path: &Path,
    bytes: &[u8],
    backup: bool,
) -> Result<Saved, SaveError> {
    if session.external_changed().map_err(SaveError::Io)? {
        session.external_modified();
        log::warn!("files: save to {} meets an external change", path.display());
        return Err(SaveError::Conflict);
    }
    write_ticketed(session, path, bytes, backup, false)
}

/// Save As to a chosen path (FILE-04, FILE-05).
pub fn save_as(session: &mut Session, path: &Path, bytes: &[u8]) -> Result<Saved, SaveError> {
    let target = resolved(path).map_err(SaveError::Io)?;
    let backup = !atomic_capable(&target).map_err(SaveError::Io)?;
    let own = match session.path() {
        Some(current) => same_file(current, &target).map_err(SaveError::Io)?,
        None => false,
    };
    if own {
        // The session's own file keeps its conflict check.
        return save_mode(session, &target, bytes, backup);
    }
    log::info!("files: save as {}", target.display());
    // A new file keeps the local version in a conflict.
    let keep_local = session.state() == SaveState::ExternalModified;
    write_ticketed(session, &target, bytes, backup, keep_local)
}

/// Keep-local in a conflict: overwrite the changed file (FILE-05).
pub fn save_overwrite(session: &mut Session, bytes: &[u8]) -> Result<Saved, SaveError> {
    let Some(path) = session.path().map(Path::to_path_buf) else {
        return Err(SaveError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "no path to save",
        )));
    };
    let target = resolved(&path).map_err(SaveError::Io)?;
    let backup = !atomic_capable(&target).map_err(SaveError::Io)?;
    write_ticketed(session, &target, bytes, backup, true)
}

/// Ticket, write, confirm: the shared tail of every save.
fn write_ticketed(
    session: &mut Session,
    path: &Path,
    bytes: &[u8],
    backup: bool,
    keep_local: bool,
) -> Result<Saved, SaveError> {
    let ticket = if keep_local {
        session.begin_save_overwrite()
    } else {
        session.begin_save()
    }
    .map_err(SaveError::Refused)?;
    let revision = session.revision();
    let outcome = if backup {
        save_with_backup(path, bytes, revision)
    } else {
        save_atomic(path, bytes, revision).map(|written| (written, None))
    };
    let (written, backup) = match outcome {
        Ok(done) => done,
        Err(error) => {
            if session.fail_save(ticket).is_err() {
                log::error!("files: the failed save left an unknown ticket");
            }
            return Err(SaveError::Io(error));
        }
    };
    // Writes keep `\\?\`; the session shows plain.
    let kept = dunce::simplified(path).to_path_buf();
    session
        .finish_save(ticket, kept.clone(), written)
        .map_err(|_| {
            SaveError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "stale save ticket",
            ))
        })?;
    Ok(Saved {
        path: kept,
        written,
        backup,
    })
}

/// Whether the target's volume replaces files atomically (§6.1).
pub fn atomic_capable(path: &Path) -> io::Result<bool> {
    capable(parent_dir(path))
}

#[cfg(unix)]
fn statfs(dir: &Path) -> io::Result<libc::statfs> {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(dir.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "the path holds a NUL"))?;
    let mut stats: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(path.as_ptr(), &mut stats) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(stats)
}

#[cfg(target_os = "macos")]
fn capable(dir: &Path) -> io::Result<bool> {
    const MNT_LOCAL: i32 = 0x0000_1000;
    Ok(statfs(dir)?.f_flags as i32 & MNT_LOCAL != 0)
}

#[cfg(target_os = "linux")]
fn capable(dir: &Path) -> io::Result<bool> {
    const NFS: i64 = 0x6969;
    const SMB: i64 = 0x517B;
    const CIFS: i64 = 0xFF534D42;
    Ok(!matches!(statfs(dir)?.f_type, NFS | SMB | CIFS))
}

#[cfg(windows)]
fn capable(dir: &Path) -> io::Result<bool> {
    use std::path::Component;
    use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;
    const DRIVE_REMOTE: u32 = 4;
    let root = match dir.components().next() {
        Some(Component::Prefix(prefix)) => prefix.as_os_str().to_string_lossy().into_owned() + "\\",
        _ => "\\".to_string(),
    };
    let wide: Vec<u16> = root.encode_utf16().chain(Some(0)).collect();
    Ok(unsafe { GetDriveTypeW(wide.as_ptr()) } != DRIVE_REMOTE)
}

#[cfg(not(any(unix, windows)))]
fn capable(_: &Path) -> io::Result<bool> {
    Ok(true)
}

/// Takes every target or none; a file there already refuses.
pub fn claim_all(paths: &[PathBuf]) -> io::Result<Vec<std::fs::File>> {
    let mut taken = Vec::new();
    for path in paths {
        // A new file only: never a link someone left.
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(file) => taken.push(file),
            Err(error) => {
                paths[..taken.len()]
                    .iter()
                    .try_for_each(std::fs::remove_file)?;
                let name = path.file_name().map_or_else(
                    || path.display().to_string(),
                    |name| name.to_string_lossy().into_owned(),
                );
                return Err(io::Error::new(error.kind(), name));
            }
        }
    }
    Ok(taken)
}
