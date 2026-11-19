use std::path::{Path, PathBuf};

use anyhow::Result;
use directories::BaseDirs;

use crate::AppId;

/// Everything an app stores lives here; uninstall removes only this.
pub fn data_dir(app: AppId) -> Result<PathBuf> {
    Ok(root()?.join(app.slug()))
}

/// The suite's folder: each app's, and the shared settings.
pub fn root() -> Result<PathBuf> {
    Ok(crate::folder::read(&home()?)?.at)
}

/// The first folder, holding the pointer to the chosen one.
pub(crate) fn home() -> Result<PathBuf> {
    #[cfg(any(test, feature = "test-support"))]
    if let Some(root) = TEST_ROOT.get() {
        return Ok(root.clone());
    }
    if let Some(root) = chosen_root(crate::env::value("ELY_DATA_ROOT"))? {
        return Ok(root);
    }
    real_home()
}

/// The user's own folder; tests never reach it.
fn real_home() -> Result<PathBuf> {
    let base = BaseDirs::new().ok_or_else(|| anyhow::anyhow!("no home directory"))?;
    let home = suite_home(base.data_local_dir());
    // Tests: callers swallow errors, so the test fails here.
    #[cfg(any(test, feature = "test-support"))]
    panic!("a test reached the data folder ({}) without ely_platform::test_root(); it is never used", home.display());
    #[cfg(not(any(test, feature = "test-support")))]
    Ok(home)
}

/// The suite's folder in a platform's local data folder.
fn suite_home(base: &Path) -> PathBuf {
    base.join("ElyOffice")
}

#[cfg(any(test, feature = "test-support"))]
static TEST_ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Tests: this process's data root, a fresh temporary folder.
#[cfg(any(test, feature = "test-support"))]
pub fn test_root() -> &'static Path {
    TEST_ROOT.get_or_init(|| {
        // Lives as long as the process; its tests share it.
        let made = tempfile::Builder::new().prefix("ely-data-").tempdir().expect("a temporary data root");
        made.keep()
    })
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
    use std::path::Path;

    use super::{chosen_root, data_dir, omarchy_dir, suite_home, test_root};
    use crate::AppId;

    #[test]
    fn the_suite_folder_is_elyoffice_in_the_platform_folder() {
        let base = Path::new("/somewhere/local-data");
        assert_eq!(suite_home(base), base.join("ElyOffice"));
    }

    #[test]
    fn each_app_has_its_own_directory_under_the_root() {
        let root = test_root();
        assert_eq!(data_dir(AppId::Doc).unwrap(), root.join("ely-doc"));
        assert_eq!(data_dir(AppId::Mail).unwrap(), root.join("ely-mail"));
    }

    /// Run by the guard test in its own process.
    #[test]
    #[ignore]
    fn a_child_without_a_test_root() {
        if std::env::var_os("ELY_ROOT_CHILD").is_some() {
            println!("found {:?}", data_dir(AppId::Doc));
        }
    }

    #[test]
    fn without_a_test_root_the_data_folder_fails() {
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "dirs::tests::a_child_without_a_test_root", "--ignored", "--nocapture"])
            .env("ELY_ROOT_CHILD", "1")
            .env_remove("ELY_DATA_ROOT")
            .output()
            .unwrap();
        let said = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        assert!(!out.status.success() && !said.contains("found "), "{said}");
        assert!(said.contains("a test reached the data folder (") && said.contains(") without ely_platform::test_root()"), "{said}");
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
