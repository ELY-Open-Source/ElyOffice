//! Where the LOK build and the worker binary are.

use std::{
    io,
    path::{Path, PathBuf},
};

/// The kernel's bundled templates of `kind`, such as `presnt`.
pub fn templates(kind: &str) -> io::Result<Vec<PathBuf>> {
    let install = install_dir()?;
    let root = install
        .parent()
        .ok_or_else(|| io::Error::other("the install has no parent"))?;
    let common = match cfg!(target_os = "macos") {
        true => root.join("Resources/template/common"),
        false => root.join("share/template/common"),
    };
    let mut found = std::fs::read_dir(common.join(kind))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<io::Result<Vec<_>>>()?;
    found.sort();
    Ok(found)
}

/// The kernel in this build.
#[cfg(target_os = "macos")]
const BUILT: &str = "instdir/LibreOffice.app/Contents/Frameworks";
#[cfg(not(target_os = "macos"))]
const BUILT: &str = "instdir/program";

/// The `.pkg`'s shared kernel, for an app bundle only.
#[cfg(target_os = "macos")]
fn installed(exe: &Path) -> Option<PathBuf> {
    let bundled = exe.parent().is_some_and(|dir| dir.ends_with("Contents/MacOS"));
    bundled.then(|| PathBuf::from("/Library/Application Support/Ely Office/Kernel/Frameworks"))
}

/// The kernel beside the app.
#[cfg(not(target_os = "macos"))]
fn installed(exe: &Path) -> Option<PathBuf> {
    exe.parent().map(|dir| dir.join("libreoffice/program"))
}

/// The LOK build: `ELY_LOK_DIR`, the installed one, or engines/.
pub fn install_dir() -> io::Result<PathBuf> {
    if let Ok(dir) = std::env::var("ELY_LOK_DIR") {
        let path = PathBuf::from(dir);
        return if path.try_exists()? {
            Ok(path)
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("ELY_LOK_DIR {} does not exist", path.display()),
            ))
        };
    }
    let library = crate::ffi::KERNEL;
    if let Some(installed) = installed(&std::env::current_exe()?)
        && installed.join(library).try_exists()?
    {
        return Ok(installed);
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engines/lo-src").join(BUILT);
    if dev.join(library).try_exists()? {
        return Ok(dev);
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no LOK build (set ELY_LOK_DIR)",
    ))
}

/// A test's LOK build; missing skips unless required.
#[cfg(any(test, feature = "test-support"))]
pub fn test_install() -> Option<PathBuf> {
    let required = std::env::var("ELY_REQUIRE_ENGINES").as_deref() == Ok("1");
    match install_dir() {
        Ok(dir) => Some(dir),
        Err(error) if required => panic!("ELY_REQUIRE_ENGINES=1 and {error}"),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            eprintln!("SKIP: {error}");
            None
        }
        Err(error) => panic!("the LOK build cannot be read: {error}"),
    }
}

/// The worker: env override, beside this exe, or its helper.
pub(super) fn worker_bin() -> io::Result<PathBuf> {
    if let Ok(bin) = std::env::var("ELY_LOK_WORKER") {
        if bin.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "ELY_LOK_WORKER is empty",
            ));
        }
        let path = PathBuf::from(&bin);
        return if path.try_exists()? {
            Ok(path)
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("ELY_LOK_WORKER {bin} does not exist"),
            ))
        };
    }
    let name = if cfg!(windows) { "ely-lok-worker.exe" } else { "ely-lok-worker" };
    let exe = std::env::current_exe()?;
    let dir = exe.parent().unwrap_or(Path::new("."));
    // A Mac app keeps it in its own helper bundle.
    let sibling = match cfg!(target_os = "macos") && dir.ends_with("Contents/MacOS") {
        true => dir.with_file_name("Helpers/Ely Engine.app/Contents/MacOS").join(name),
        false => dir.join(name),
    };
    if sibling.try_exists()? {
        return Ok(sibling);
    }
    // Test binaries live one folder below the worker.
    if let Some(above) = exe.parent().and_then(Path::parent).map(|dir| dir.join(name))
        && above.try_exists()?
    {
        return Ok(above);
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no ely-lok-worker binary (set ELY_LOK_WORKER)",
    ))
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    /// Run by the test below with its own environment.
    #[test]
    #[ignore = "a helper process"]
    fn absent() {
        assert!(super::test_install().is_none(), "no build is there");
    }

    /// The helper with no LOK build, strict or not.
    fn absent_with(strict: bool) -> std::process::Output {
        let mut run = Command::new(std::env::current_exe().expect("this test"));
        run.args(["--exact", "client::install::tests::absent", "--ignored", "--nocapture"]);
        run.env("ELY_LOK_DIR", std::env::temp_dir().join("ely-no-lok-here"));
        match strict {
            true => run.env("ELY_REQUIRE_ENGINES", "1"),
            false => run.env_remove("ELY_REQUIRE_ENGINES"),
        };
        run.output().expect("the helper runs")
    }

    #[test]
    fn a_missing_build_skips_unless_engines_are_required() {
        let loose = absent_with(false);
        let said = String::from_utf8_lossy(&loose.stderr);
        assert!(loose.status.success() && said.contains("SKIP: ELY_LOK_DIR"), "{said}");
        let strict = absent_with(true);
        let said = String::from_utf8_lossy(&strict.stderr);
        assert!(!strict.status.success(), "a required engine that is missing fails");
        assert!(said.contains("ELY_REQUIRE_ENGINES=1 and ELY_LOK_DIR"), "{said}");
    }
}
