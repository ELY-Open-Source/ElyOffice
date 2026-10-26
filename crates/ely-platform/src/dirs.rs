use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::BaseDirs;

use crate::AppId;

/// Everything an app stores lives here; uninstall removes only this.
pub fn data_dir(app: AppId) -> Result<PathBuf> {
    Ok(root()?.join(app.slug()))
}

/// The suite's folder: each app's, and the shared settings.
pub(crate) fn root() -> Result<PathBuf> {
    Ok(crate::folder::read(&home()?)?.at)
}

/// The first folder, holding the pointer to the chosen one.
pub(crate) fn home() -> Result<PathBuf> {
    if let Some(root) = chosen_root(crate::env::value("ELY_DATA_ROOT"))? {
        return Ok(root);
    }
    let base = BaseDirs::new().context("no home directory")?;
    Ok(base.data_local_dir().join("ElyOffice"))
}

/// Benchmarks only: a seeded folder, never the user's.
fn chosen_root(over: Option<String>) -> Result<Option<PathBuf>> {
    let Some(root) = over else { return Ok(None) };
    let root = PathBuf::from(root);
    anyhow::ensure!(root.is_absolute(), "ELY_DATA_ROOT is not absolute: {}", root.display());
    Ok(Some(root))
}

/// Omarchy's active theme directory; `ELY_OMARCHY_DIR` overrides it.
pub fn omarchy_current_dir() -> Option<PathBuf> {
    let home = BaseDirs::new()?.home_dir().to_path_buf();
    omarchy_dir(&home, crate::env::value("ELY_OMARCHY_DIR"))
}

/// Omarchy keeps its state under `~/.local/state`, whatever `XDG_STATE_HOME` says.
fn omarchy_dir(home: &Path, over: Option<String>) -> Option<PathBuf> {
    if let Some(dir) = over {
        return Some(PathBuf::from(dir));
    }
    let current = home.join(".local/state/omarchy/current");
    current.is_dir().then_some(current)
}

#[cfg(test)]
mod tests {
    use super::{chosen_root, data_dir, omarchy_dir};
    use crate::AppId;

    #[test]
    fn each_app_has_its_own_directory_under_elyoffice() {
        let doc = data_dir(AppId::Doc).unwrap();
        let mail = data_dir(AppId::Mail).unwrap();

        assert!(doc.ends_with("ElyOffice/ely-doc"), "{}", doc.display());
        assert!(mail.ends_with("ElyOffice/ely-mail"), "{}", mail.display());
        assert_ne!(doc, mail);
    }

    #[test]
    fn a_data_root_override_must_be_absolute_and_wins() {
        let at = std::env::temp_dir().join("ely-bench");
        let chosen = chosen_root(Some(at.display().to_string())).unwrap();
        assert_eq!(chosen, Some(at));
        assert_eq!(chosen_root(None).unwrap(), None);
        let err = chosen_root(Some("relative".into())).unwrap_err();
        assert!(err.to_string().contains("not absolute"), "{err}");
    }

    #[test]
    fn omarchy_is_found_under_home_and_the_override_wins() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(omarchy_dir(home.path(), None), None);

        let current = home.path().join(".local/state/omarchy/current");
        std::fs::create_dir_all(&current).unwrap();
        assert_eq!(omarchy_dir(home.path(), None), Some(current));
        assert_eq!(
            omarchy_dir(home.path(), Some("/elsewhere".into())),
            Some("/elsewhere".into())
        );
    }
}
