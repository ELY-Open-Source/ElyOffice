//! Versioned settings files, shared and per app (§18.1).

use std::{
    io::{self, Write as _},
    path::{Path, PathBuf},
};

use serde_json::{Map, Value};

use crate::{AppId, lock::FileLock};

mod shared;
pub use shared::{
    APPEARANCE, AUTHOR, Appearance, CHECK_UPDATES, LANGUAGE, Language, MOTION, Motion, OMARCHY_SYNC, Shared, UI_FONT, UI_TEXT_SIZE, UI_TEXT_SIZES,
    DOC_ZOOM, DOC_ZOOMS, DocZoom, Sides, TWO_SIDED, word,
};

/// The layout this build reads and writes.
pub const VERSION: u64 = 1;

/// One settings file; writes keep unknown fields.
#[derive(Clone, Debug)]
pub struct Store {
    path: PathBuf,
}

impl Store {
    /// Theme, language and motion, read by every app.
    pub fn shared() -> io::Result<Self> {
        let root = crate::dirs::root().map_err(io::Error::other)?;
        Ok(Self::at(root.join("settings.json")))
    }

    /// One app's own preferences.
    pub fn app(app: AppId) -> io::Result<Self> {
        Ok(Self::at(crate::data_dir(app).map_err(io::Error::other)?.join("settings.json")))
    }

    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The stored fields; none when the file is absent.
    pub fn read(&self) -> io::Result<Map<String, Value>> {
        let bytes = match std::fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Map::new()),
            Err(error) => return Err(error),
        };
        let invalid = |why: String| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {why}", self.path.display()));
        let Value::Object(fields) = serde_json::from_slice(&bytes).map_err(|error| invalid(error.to_string()))? else {
            return Err(invalid("not a JSON object".into()));
        };
        match fields.get("version") {
            None => Ok(fields),
            Some(Value::Number(n)) if n.as_u64().is_some_and(|n| n <= VERSION) => Ok(fields),
            Some(Value::Number(n)) => Err(invalid(format!("version {n} is newer than this Ely reads"))),
            Some(_) => Err(invalid("the version is not a number".into())),
        }
    }

    /// Sets `key` under the lock; other fields stay.
    pub fn set(&self, key: &str, value: Value) -> io::Result<()> {
        self.set_if(key, value, || true).map(|_| ())
    }

    /// Sets `key` if `still` holds once locked.
    pub fn set_if(&self, key: &str, value: Value, still: impl FnOnce() -> bool) -> io::Result<bool> {
        let dir = self.path.parent().ok_or_else(|| io::Error::other("settings have no folder"))?;
        std::fs::create_dir_all(dir)?;
        let _held = FileLock::take(&dir.join(".settings.lock"))?;
        // A newer choice made while we waited wins.
        if !still() {
            log::info!("settings: {key} overtaken while waiting");
            return Ok(false);
        }
        // Read under the lock: another app may just have written.
        let mut fields = self.read()?;
        fields.insert(key.into(), value);
        fields.insert("version".into(), VERSION.into());
        let mut temp = tempfile::NamedTempFile::new_in(dir)?;
        temp.write_all(&serde_json::to_vec_pretty(&Value::Object(fields))?)?;
        temp.as_file().sync_all()?;
        temp.persist(&self.path).map_err(|error| error.error)?;
        log::info!("settings: {key} saved in {}", self.path.display());
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
