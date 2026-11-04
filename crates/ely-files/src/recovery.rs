use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

#[cfg(test)]
mod tests;

/// A point over its cap of `cap` bytes.
#[derive(Debug)]
pub struct TooBig {
    pub cap: u64,
}

impl std::fmt::Display for TooBig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the recovery point exceeds its {} byte cap", self.cap)
    }
}

impl std::error::Error for TooBig {}

/// One recovery point and what it restores (§6.3).
#[derive(Clone, Debug)]
pub struct RecoveryPoint {
    pub at: SystemTime,
    pub content: PathBuf,
    pub cursor: Option<String>,
    pub history: bool,
    /// Sealed at rest; only its key restores it (§18.2).
    pub sealed: bool,
}

/// Per-document recovery copies: 10s apart, 20 points, 7 days (§6.2).
pub struct Recovery {
    dir: PathBuf,
    keep: usize,
    max_age: Duration,
    cap: u64,
    running: bool,
    key: Option<[u8; 32]>,
    next_due: Option<SystemTime>,
    last_ok: Option<SystemTime>,
}

impl Recovery {
    pub const INTERVAL: Duration = Duration::from_secs(10);
    pub const KEEP: usize = 20;
    pub const MAX_AGE: Duration = Duration::from_secs(7 * 24 * 3600);
    pub const CAP: u64 = 64 * 1024 * 1024;

    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            keep: Self::KEEP,
            max_age: Self::MAX_AGE,
            cap: Self::CAP,
            running: false,
            key: None,
            next_due: None,
            last_ok: None,
        }
    }

    /// Prunes expired points; call after every (re)open (§6.2).
    pub fn maintain(&self, now: SystemTime) -> io::Result<()> {
        self.prune(now)
    }

    /// An encrypted document's recovery, sealed with its key (§18.2).
    pub fn sensitive(dir: PathBuf, key: [u8; 32]) -> Self {
        Self {
            key: Some(key),
            ..Self::new(dir)
        }
    }

    /// The user's own limits (§6.2).
    pub fn with_limits(mut self, keep: usize, cap: u64) -> Self {
        self.keep = keep;
        self.cap = cap;
        self
    }

    /// The last successful recovery write; nothing else means persisted.
    pub fn last_ok(&self) -> Option<SystemTime> {
        self.last_ok
    }

    /// Whether a recovery write is due now (§6.2).
    pub fn tick(&mut self, now: SystemTime) -> bool {
        if self.next_due.is_some_and(|due| now < due) {
            return false;
        }
        self.next_due = Some(now + Self::INTERVAL);
        true
    }

    /// Whether the user explicitly discarded the last session (§6.2).
    pub fn discarded(&self) -> io::Result<bool> {
        self.dir.join(".discarded").try_exists()
    }

    /// Whether a point of `bytes` stays under the cap.
    pub fn fits(&self, bytes: u64) -> io::Result<()> {
        let size = if self.key.is_some() { 1 + 4 + bytes + 16 + 12 } else { bytes };
        if size > self.cap {
            return Err(io::Error::new(io::ErrorKind::FileTooLarge, TooBig { cap: self.cap }));
        }
        Ok(())
    }

    /// One point on disk, one task at a time.
    pub fn write_once(
        &mut self,
        content: &[u8],
        cursor: Option<&str>,
        now: SystemTime,
    ) -> io::Result<Option<PathBuf>> {
        if self.running {
            log::info!(
                "recovery: a task is already running for {}",
                self.dir.display()
            );
            return Ok(None);
        }
        let extra = cursor.map(|text| text.len() as u64).unwrap_or(0);
        self.fits(content.len() as u64 + extra)?;
        self.running = true;
        let result = self.write_inner(content, cursor, now);
        self.running = false;
        let path = result?;
        self.prune(now)?;
        self.last_ok = Some(now);
        match std::fs::remove_file(self.dir.join(".discarded")) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        Ok(Some(path))
    }

    /// Writes the point's files, staged and published.
    fn write_inner(
        &self,
        content: &[u8],
        cursor: Option<&str>,
        now: SystemTime,
    ) -> io::Result<PathBuf> {
        let dir = self.dir.join("points");
        std::fs::create_dir_all(&dir)?;
        let stamp = stamp_of(now)?;
        if let Some(key) = self.key {
            let cursor_bytes = cursor.map(|text| text.as_bytes()).unwrap_or(&[]);
            let mut bundle = vec![u8::from(cursor.is_some())];
            bundle.extend_from_slice(&(cursor_bytes.len() as u32).to_le_bytes());
            bundle.extend_from_slice(cursor_bytes);
            bundle.extend_from_slice(content);
            let (sealed, nonce) = seal(&key, &bundle)?;
            let mut point = nonce.to_vec();
            point.extend_from_slice(&sealed);
            let enc = dir.join(format!("{stamp}.enc"));
            crate::save_atomic(&enc, &point, 0)?;
            log::info!("recovery: a sealed point at {}", enc.display());
            return Ok(enc);
        }
        let content_path = dir.join(format!("{stamp}.bin"));
        crate::save_atomic(&content_path, content, 0)?;
        if let Some(cursor) = cursor {
            crate::save_atomic(&dir.join(format!("{stamp}.cursor")), cursor.as_bytes(), 0)?;
        }
        log::info!("recovery: a point at {}", content_path.display());
        Ok(content_path)
    }

    /// Every point on disk, newest first.
    pub fn points(&self) -> io::Result<Vec<RecoveryPoint>> {
        self.stamps()?.into_iter().map(|stamp| self.point(stamp)).collect()
    }

    /// The points' stamps by name alone, newest first.
    fn stamps(&self) -> io::Result<Vec<u128>> {
        let dir = self.dir.join("points");
        let mut stamps = Vec::new();
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };
        for entry in entries {
            let name = entry?.file_name().to_string_lossy().into_owned();
            let Some((stamp, ext)) = name.split_once('.') else {
                continue;
            };
            if matches!(ext, "bin" | "enc")
                && let Ok(stamp) = stamp.parse::<u128>()
            {
                stamps.push(stamp);
            }
        }
        stamps.sort_unstable_by(|a, b| b.cmp(a));
        stamps.dedup();
        Ok(stamps)
    }

    /// The point one stamp names.
    fn point(&self, stamp: u128) -> io::Result<RecoveryPoint> {
        let dir = self.dir.join("points");
        let at = at_of(stamp);
        let enc = dir.join(format!("{stamp}.enc"));
        if enc.try_exists()? {
            let cursor = match self.key {
                Some(key) => open_cursor(&key, &enc)?,
                None => None,
            };
            return Ok(RecoveryPoint {
                at,
                content: enc,
                cursor,
                history: false,
                sealed: true,
            });
        }
        let cursor = match std::fs::read_to_string(dir.join(format!("{stamp}.cursor"))) {
            Ok(cursor) => Some(cursor),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        Ok(RecoveryPoint {
            at,
            content: dir.join(format!("{stamp}.bin")),
            cursor,
            history: dir.join(format!("{stamp}.history")).try_exists()?,
            sealed: false,
        })
    }

    /// Drops the oldest points past age, count and byte limits.
    fn prune(&self, now: SystemTime) -> io::Result<()> {
        // Names only: a sealed point is never opened here.
        let mut stamps = self.stamps()?;
        while stamps.len() > self.keep
            || stamps.last().is_some_and(|stamp| {
                now.duration_since(at_of(*stamp))
                    .map(|age| age > self.max_age)
                    .unwrap_or(false)
            })
            || self.bytes(&stamps)? > self.cap
        {
            let Some(oldest) = stamps.pop() else { break };
            self.drop(oldest)?;
        }
        Ok(())
    }

    /// The total size of every file of every live point.
    fn bytes(&self, stamps: &[u128]) -> io::Result<u64> {
        let mut total = 0;
        for stamp in stamps {
            let dir = self.dir.join("points");
            for ext in ["bin", "cursor", "history", "enc"] {
                let path = dir.join(format!("{stamp}.{ext}"));
                match std::fs::metadata(&path) {
                    Ok(meta) => total += meta.len(),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(total)
    }

    /// Removes a point's files, its metadata first.
    fn drop(&self, stamp: u128) -> io::Result<()> {
        let dir = self.dir.join("points");
        for ext in ["cursor", "history", "bin", "enc"] {
            match std::fs::remove_file(dir.join(format!("{stamp}.{ext}"))) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    /// An explicit discard: points go, the result stays recorded (§6.2).
    pub fn discard(&mut self) -> io::Result<()> {
        for stamp in self.stamps()? {
            self.drop(stamp)?;
        }
        std::fs::write(self.dir.join(".discarded"), "discarded")?;
        self.last_ok = None;
        Ok(())
    }

    /// Restores the point beside, never over; sealed needs its key.
    pub fn restore(&self, point: &RecoveryPoint, dest: &Path) -> io::Result<()> {
        if point.sealed && self.key.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "a sealed point needs its recovery key",
            ));
        }
        let mut file = fresh_dest(dest)?;
        if point.sealed {
            let bundle = open_bundle(&self.key.expect("a sealed recovery holds its key"), point)?;
            let (prefix, rest) = bundle.split_at(5);
            let cursor_len = u32::from_le_bytes(prefix[1..].try_into().expect("a u32 prefix"));
            file.write_all(&rest[cursor_len as usize..])?;
        } else {
            let mut source = std::fs::File::open(&point.content)?;
            io::copy(&mut source, &mut file)?;
        }
        file.sync_all()?;
        log::info!("recovery: restored to {}", dest.display());
        Ok(())
    }
}

/// The time a stamp names.
fn at_of(stamp: u128) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_nanos(stamp as u64)
}

/// The point's timestamp as its file stem.
fn stamp_of(at: SystemTime) -> io::Result<u128> {
    Ok(at
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "time before 1970"))?
        .as_nanos())
}

/// An exclusive fresh restore target; links never followed.
fn fresh_dest(dest: &Path) -> io::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dest)
        .map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", dest.display())))
}

/// AES-256-GCM with a fresh nonce (§18.2).
fn seal(key: &[u8; 32], plaintext: &[u8]) -> io::Result<(Vec<u8>, [u8; 12])> {
    use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
    let mut nonce_bytes = [0u8; 12];
    getrandom::fill(&mut nonce_bytes).map_err(|_| io::Error::other("system randomness failed"))?;
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "recovery key rejected"))?;
    let out = cipher
        .encrypt(nonce(&nonce_bytes)?, plaintext)
        .map_err(|_| io::Error::other("sealing failed"))?;
    Ok((out, nonce_bytes))
}

/// A 96-bit nonce reference from its bytes.
fn nonce<'a>(bytes: &'a [u8]) -> io::Result<&'a aes_gcm::aead::Nonce<aes_gcm::Aes256Gcm>> {
    <&'a aes_gcm::aead::Nonce<aes_gcm::Aes256Gcm>>::try_from(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad nonce"))
}

/// Reverses `seal`; a wrong key fails loud.
fn open_sealed(key: &[u8; 32], ciphertext: &[u8], nonce_bytes: &[u8]) -> io::Result<Vec<u8>> {
    use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "recovery key rejected"))?;
    cipher
        .decrypt(nonce(nonce_bytes)?, ciphertext)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "the sealed point is unreadable"))
}

/// Reads a sealed point's bundle from disk.
fn open_bundle(key: &[u8; 32], point: &RecoveryPoint) -> io::Result<Vec<u8>> {
    let file = std::fs::read(&point.content)?;
    let Some((nonce, ciphertext)) = file.split_at_checked(12) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "a short sealed point",
        ));
    };
    open_sealed(key, ciphertext, nonce)
}

/// The cursor a sealed bundle leads with, when present.
fn open_cursor(key: &[u8; 32], enc: &Path) -> io::Result<Option<String>> {
    let point = RecoveryPoint {
        at: stamp_of_file(enc)?,
        content: enc.to_path_buf(),
        cursor: None,
        history: false,
        sealed: true,
    };
    let bundle = open_bundle(key, &point)?;
    let (flag, rest) = bundle
        .split_at_checked(1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "a short sealed bundle"))?;
    if flag[0] == 0 {
        return Ok(None);
    }
    let (prefix, rest) = rest
        .split_at_checked(4)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "a short sealed bundle"))?;
    let cursor_len = u32::from_le_bytes(prefix.try_into().expect("a u32 prefix")) as usize;
    let cursor = rest
        .get(..cursor_len)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "a short sealed bundle"))?;
    Ok(Some(String::from_utf8(cursor.to_vec()).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidData, "cursor is no text")
    })?))
}

/// The stamp an `.enc` file name carries.
fn stamp_of_file(path: &Path) -> io::Result<SystemTime> {
    let name = path.file_stem().unwrap_or_default().to_string_lossy();
    let stamp: u128 = name
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad point name"))?;
    Ok(SystemTime::UNIX_EPOCH + Duration::from_nanos(stamp as u64))
}

/// A recovery copy lands beside, never over.
pub fn restore_copy(point: &RecoveryPoint, dest: &Path) -> io::Result<()> {
    if point.sealed {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a sealed point restores through its Recovery",
        ));
    }
    let mut file = fresh_dest(dest)?;
    let mut source = std::fs::File::open(&point.content)?;
    io::copy(&mut source, &mut file)?;
    file.sync_all()?;
    log::info!(
        "recovery: restored {} to {}",
        point.content.display(),
        dest.display()
    );
    Ok(())
}
