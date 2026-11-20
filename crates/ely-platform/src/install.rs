//! Puts a downloaded release in place of the running app.

mod after;
mod package;
#[cfg(target_os = "linux")]
pub mod units;

use std::{
    io,
    path::{Path, PathBuf},
};

pub use after::after_quit;
pub use package::Package;

/// How this app was installed, and so how it updates.
#[derive(Clone, Debug, PartialEq)]
pub enum Installed {
    /// A macOS `.app`.
    Bundle(PathBuf),
    /// The folder a Windows setup wrote, and the app.
    Folder { dir: PathBuf, exe: PathBuf },
    /// A Linux AppImage file, and the app it ran.
    AppImage { image: PathBuf, slug: String },
    /// A package's manager and name: the manager updates it.
    Package(Package, String),
    /// The Nix store: Nix updates it, Ely only tells.
    Nix,
}

/// The list a Windows setup leaves in its folder.
const LISTED: &str = ".ely-installed";
/// Where Linux packages install; the manager owns them.
const PACKAGED: [&str; 4] = ["/usr", "/opt", "/app", "/snap"];
/// The Nix store: Nix updates what it holds.
const NIX: &str = "/nix/store";

/// How the running app updates; none for a dev build.
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
            _ if exe.starts_with(NIX) => Some(Installed::Nix),
            Some((image, _)) => Some(Installed::AppImage { image, slug: slug.to_owned() }),
            None if PACKAGED.iter().any(|prefix| exe.starts_with(prefix)) => {
                let (manager, name) = Package::of(root, slug, rpm)?;
                Some(Installed::Package(manager, name))
            }
            None => None,
        },
        _ => None,
    })
}

fn bundle_of(exe: &Path) -> Option<&Path> {
    exe.ancestors().find(|dir| dir.extension().is_some_and(|ext| ext == "app"))
}

#[cfg(test)]
mod tests;
