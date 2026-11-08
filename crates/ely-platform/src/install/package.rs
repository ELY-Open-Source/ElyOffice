//! Linux packages: which manager owns the app, and its command.

use std::{io, path::Path, process::Command};

use semver::Version;

/// The Linux package manager that installed the app.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Package {
    Deb,
    Rpm,
    Arch,
}

/// Whether this system's rpm lists `slug`.
pub(super) fn rpm_lists(slug: &str) -> io::Result<bool> {
    let status = Command::new("rpm").args(["-q", "--quiet", slug]).status()?;
    Ok(status.success())
}

impl Package {
    /// Reads the managers' own records under `root`.
    pub(super) fn of(root: &Path, slug: &str, rpm: &dyn Fn(&str) -> io::Result<bool>) -> io::Result<Self> {
        if root.join(format!("var/lib/dpkg/info/{slug}.list")).try_exists()? {
            return Ok(Self::Deb);
        }
        let pacman = root.join("var/lib/pacman/local");
        if pacman.try_exists()? {
            for entry in std::fs::read_dir(&pacman)? {
                let name = entry?.file_name();
                // pacman names it `<name>-<version>-<release>`.
                let version = name.to_str().and_then(|name| name.strip_prefix(slug)?.strip_prefix('-'));
                if version.is_some_and(|version| version.starts_with(|c: char| c.is_ascii_digit())) {
                    return Ok(Self::Arch);
                }
            }
        }
        let database = root.join("usr/lib/sysimage/rpm").try_exists()? || root.join("var/lib/rpm").try_exists()?;
        // A database alone says nothing of this app.
        if database && rpm(slug)? {
            return Ok(Self::Rpm);
        }
        Err(io::Error::new(io::ErrorKind::NotFound, format!("no package manager lists {slug}")))
    }

    /// The CPU name this manager writes in file names.
    fn cpu(self, arch: &str) -> &str {
        match (self, arch) {
            (Self::Deb, "x86_64") => "amd64",
            (Self::Deb, "aarch64") => "arm64",
            (_, arch) => arch,
        }
    }

    /// The release's file for `arch`, such as `amd64`.
    pub fn file_for(self, slug: &str, version: &Version, arch: &str) -> String {
        let cpu = self.cpu(arch);
        match self {
            Self::Deb => format!("{slug}_{version}-1_{cpu}.deb"),
            Self::Rpm => format!("{slug}-{version}-1.{cpu}.rpm"),
            Self::Arch => format!("{slug}-{version}-1-{cpu}.pkg.tar.zst"),
        }
    }

    /// The release's file for this host.
    pub fn file(self, slug: &str, version: &Version) -> String {
        self.file_for(slug, version, std::env::consts::ARCH)
    }

    /// The command that installs the downloaded file.
    pub fn command(self, slug: &str, version: &Version) -> String {
        let file = self.file(slug, version);
        match self {
            Self::Deb => format!("sudo apt install ./{file}"),
            Self::Rpm => format!("sudo dnf install ./{file}"),
            Self::Arch => format!("sudo pacman -U ./{file}"),
        }
    }
}
