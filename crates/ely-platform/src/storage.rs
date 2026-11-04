//! An app's kept data, and clearing it (§18.3).

use std::{
    io,
    path::{Path, PathBuf},
};

use crate::AppId;

/// Kept data the user may see and clear.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Log,
    Recovery,
}

impl Kind {
    pub const ALL: [Kind; 2] = [Kind::Log, Kind::Recovery];

    fn place(self) -> &'static str {
        match self {
            Kind::Log => "ely.log",
            Kind::Recovery => "recovery",
        }
    }

    /// In use while running, so cleared at the next start.
    pub fn at_next_start(self) -> bool {
        self != Kind::Log
    }

    fn mark(self) -> String {
        format!(".clear-{}", self.place())
    }
}

/// Bytes each kind holds now; a missing one holds none.
pub fn usage(app: AppId) -> io::Result<Vec<(Kind, u64)>> {
    let dir = crate::data_dir(app).map_err(io::Error::other)?;
    Kind::ALL.into_iter().map(|kind| Ok((kind, size(&dir.join(kind.place()))?))).collect()
}

fn size(path: &Path) -> io::Result<u64> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };
    if !meta.is_dir() {
        return Ok(meta.len());
    }
    std::fs::read_dir(path)?.map(|entry| size(&entry?.path())).sum()
}

/// Clears `kind` now, or marks it for the next start.
pub fn clear(app: AppId, kind: Kind) -> io::Result<()> {
    let dir = crate::data_dir(app).map_err(io::Error::other)?;
    if kind.at_next_start() {
        std::fs::write(dir.join(kind.mark()), b"")?;
        log::info!("storage: {kind:?} clears at the next start");
        return Ok(());
    }
    // The logger appends; emptying keeps its handle valid.
    std::fs::OpenOptions::new().write(true).open(dir.join(kind.place()))?.set_len(0)?;
    log::info!("storage: {kind:?} cleared");
    Ok(())
}

/// Whether `kind` waits to be cleared at the next start.
pub fn clearing(app: AppId, kind: Kind) -> io::Result<bool> {
    let dir = crate::data_dir(app).map_err(io::Error::other)?;
    dir.join(kind.mark()).try_exists()
}

/// At start, before anything opens them: the clears asked for.
pub fn clear_marked(app: AppId) -> io::Result<()> {
    clear_marked_in(&crate::data_dir(app).map_err(io::Error::other)?)
}

fn clear_marked_in(dir: &Path) -> io::Result<()> {
    for kind in Kind::ALL.into_iter().filter(|kind| kind.at_next_start()) {
        let mark = dir.join(kind.mark());
        if !mark.try_exists()? {
            continue;
        }
        let place: PathBuf = dir.join(kind.place());
        match std::fs::remove_dir_all(&place) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
            _ => {}
        }
        std::fs::remove_file(&mark)?;
        log::info!("storage: {kind:?} cleared as asked");
    }
    Ok(())
}

/// Recovery points kept per document, and space for them (§6.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub keep: usize,
    pub megabytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self { keep: 20, megabytes: 64 }
    }
}

const LIMITS: &str = "recovery";

/// `app`'s recovery limits; a bad entry is an error.
pub fn limits(app: AppId) -> io::Result<Limits> {
    limits_in(&crate::settings::Store::app(app)?)
}

/// The recovery limits `store` holds.
pub fn limits_in(store: &crate::settings::Store) -> io::Result<Limits> {
    limits_of(store.read()?.get(LIMITS))
}

fn limits_of(saved: Option<&serde_json::Value>) -> io::Result<Limits> {
    let Some(saved) = saved else {
        return Ok(Limits::default());
    };
    let number = |key: &str, least: u64, most: u64| {
        saved.get(key).and_then(serde_json::Value::as_u64).filter(|n| (least..=most).contains(n)).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, format!("settings: recovery {key} cannot be {saved}"))
        })
    };
    Ok(Limits { keep: number("keep", 1, 100)? as usize, megabytes: number("megabytes", 1, 4096)? })
}

/// Keeps new limits for `app` if `still` holds once locked.
pub fn set_limits(app: AppId, limits: Limits, still: impl FnOnce() -> bool) -> io::Result<bool> {
    let saved = serde_json::json!({ "keep": limits.keep, "megabytes": limits.megabytes });
    crate::settings::Store::app(app)?.set_if(LIMITS, saved, still)
}

#[cfg(test)]
mod tests {
    use super::{Kind, Limits, clear_marked_in, limits_of, size};

    #[test]
    fn limits_read_strictly() {
        assert_eq!(limits_of(None).unwrap(), Limits::default());
        let set = serde_json::json!({"keep": 10, "megabytes": 32});
        assert_eq!(limits_of(Some(&set)).unwrap(), Limits { keep: 10, megabytes: 32 });
        for bad in [serde_json::json!({"keep": 0, "megabytes": 32}), serde_json::json!({"keep": "ten"}), serde_json::json!(5)] {
            assert!(limits_of(Some(&bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn marked_kinds_go_at_start_and_others_stay() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("recovery/a")).unwrap();
        std::fs::write(dir.path().join("recovery/a/point"), vec![0u8; 4096]).unwrap();
        std::fs::write(dir.path().join("ely.log"), b"line").unwrap();
        assert_eq!(size(&dir.path().join("recovery")).unwrap(), 4096);
        std::fs::write(dir.path().join(Kind::Recovery.mark()), b"").unwrap();
        clear_marked_in(dir.path()).unwrap();
        assert!(!dir.path().join("recovery").exists(), "the copies went");
        assert!(dir.path().join("ely.log").exists(), "the log stays");
        assert!(!dir.path().join(Kind::Recovery.mark()).exists(), "the mark is spent");
    }
}
