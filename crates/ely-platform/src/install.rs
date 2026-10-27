//! Puts a downloaded release in place of the running app.

mod after;
mod package;

use std::{
    io,
    path::{Path, PathBuf},
};
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};

pub use after::{INSTALL, after_quit};
pub use package::Package;

/// How this app was installed, and so how it updates.
#[derive(Clone, Debug, PartialEq)]
pub enum Installed {
    /// A macOS `.app`.
    Bundle(PathBuf),
    /// The folder a Windows setup wrote, and the app.
    Folder { dir: PathBuf, exe: PathBuf },
    /// A Linux AppImage file.
    AppImage(PathBuf),
    /// A deb, rpm or Arch package: its manager updates it.
    Package(Package),
}

/// The list a Windows setup leaves in its folder.
const LISTED: &str = ".ely-installed";
/// Where Linux packages install; the manager owns them.
const PACKAGED: [&str; 4] = ["/usr", "/opt", "/app", "/snap"];
/// The Nix store: Nix updates what it holds.
const NIX: &str = "/nix/store";

/// How the running app updates; none for dev or Nix.
pub fn running(slug: &str) -> io::Result<Option<Installed>> {
    let exe = std::env::current_exe()?;
    let image = std::env::var_os("APPIMAGE").map(PathBuf::from);
    let mount = std::env::var_os("APPDIR").map(PathBuf::from);
    installed(std::env::consts::OS, &exe, image.zip(mount), Path::new("/"), slug, &package::rpm_lists)
}

/// `image` is an AppImage and its mount; `rpm` asks rpm.
fn installed(
    os: &str,
    exe: &Path,
    image: Option<(PathBuf, PathBuf)>,
    root: &Path,
    slug: &str,
    rpm: &dyn Fn(&str) -> io::Result<bool>,
) -> io::Result<Option<Installed>> {
    Ok(match os {
        "macos" => bundle_of(exe).map(|bundle| Installed::Bundle(bundle.to_path_buf())),
        "windows" => {
            let dir = exe.parent().ok_or_else(|| io::Error::other("the app has no folder"))?;
            dir.join(LISTED)
                .try_exists()?
                .then(|| Installed::Folder { dir: dir.to_path_buf(), exe: exe.to_path_buf() })
        }
        // An AppImage's terminal passes its variables on.
        "linux" => match image.filter(|(_, mount)| exe.starts_with(mount)) {
            // Nix wraps the AppImage; its store is read-only.
            _ if exe.starts_with(NIX) => None,
            Some((image, _)) => Some(Installed::AppImage(image)),
            None if PACKAGED.iter().any(|prefix| exe.starts_with(prefix)) => {
                Some(Installed::Package(Package::of(root, slug, rpm)?))
            }
            None => None,
        },
        _ => None,
    })
}

fn bundle_of(exe: &Path) -> Option<&Path> {
    exe.ancestors().find(|dir| dir.extension().is_some_and(|ext| ext == "app"))
}

/// Replaces `bundle` with the same-named app inside `dmg`.
#[cfg(target_os = "macos")]
pub fn install(dmg: &Path, bundle: &Path) -> io::Result<()> {
    let name = bundle.file_name().ok_or_else(|| io::Error::other("the app has no name"))?;
    let parent = bundle.parent().ok_or_else(|| io::Error::other("the app has no folder"))?;
    let mount = tempfile::tempdir()?;
    run(Command::new("hdiutil").args(["attach", "-nobrowse", "-readonly", "-noautoopen", "-mountpoint"]).arg(mount.path()).arg(dmg))?;
    let copied = stage(&mount.path().join(name), parent, name);
    let detached = run(Command::new("hdiutil").args(["detach", "-force"]).arg(mount.path()));
    let staged = copied?;
    detached?;
    swap(&staged, bundle)
}

/// Copies the new app beside the old, hidden.
#[cfg(target_os = "macos")]
fn stage(from: &Path, parent: &Path, name: &std::ffi::OsStr) -> io::Result<PathBuf> {
    if !from.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("the download holds no {}", name.to_string_lossy())));
    }
    let staged = parent.join(format!(".{}.new", name.to_string_lossy()));
    if staged.exists() {
        std::fs::remove_dir_all(&staged)?;
    }
    run(Command::new("ditto").arg(from).arg(&staged))?;
    Ok(staged)
}

/// Puts `staged` at `bundle`; on failure the old returns.
#[cfg(any(target_os = "macos", test))]
fn swap(staged: &Path, bundle: &Path) -> io::Result<()> {
    let name = bundle.file_name().ok_or_else(|| io::Error::other("the app has no name"))?;
    let old = bundle.with_file_name(format!(".{}.old", name.to_string_lossy()));
    if old.exists() {
        std::fs::remove_dir_all(&old)?;
    }
    std::fs::rename(bundle, &old)?;
    if let Err(error) = std::fs::rename(staged, bundle) {
        std::fs::rename(&old, bundle)?;
        return Err(error);
    }
    log::info!("install: {} replaced", bundle.display());
    // Installed: an old copy left behind is only untidy.
    if let Err(error) = std::fs::remove_dir_all(&old) {
        log::warn!("install: the old copy at {} stays: {error}", old.display());
    }
    Ok(())
}

/// Returns once process `pid` has ended.
#[cfg(target_os = "macos")]
pub fn wait_for_exit(pid: u32) {
    while unsafe { libc::kill(pid as libc::pid_t, 0) } == 0 {
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

/// Opens `bundle` as a fresh launch.
#[cfg(target_os = "macos")]
pub fn reopen(bundle: &Path) -> io::Result<()> {
    run(Command::new("open").arg(bundle))
}

/// Runs `command`; a failure carries what it said.
#[cfg(target_os = "macos")]
fn run(command: &mut Command) -> io::Result<()> {
    let out = command.stdin(Stdio::null()).output()?;
    if out.status.success() {
        return Ok(());
    }
    let said = String::from_utf8_lossy(&out.stderr);
    Err(io::Error::other(format!("{:?} failed ({}): {}", command.get_program(), out.status, said.trim())))
}

#[cfg(test)]
mod tests;
