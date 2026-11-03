//! Where Rscript is: `ELY_RSCRIPT`, PATH, then R's install.

use super::protocol::refused;
use std::{ffi::OsString, io, path::PathBuf};

#[cfg(windows)]
const NAME: &str = "Rscript.exe";
#[cfg(not(windows))]
const NAME: &str = "Rscript";

pub fn locate() -> io::Result<PathBuf> {
    resolve(std::env::var_os("ELY_RSCRIPT"), std::env::var_os("PATH"))
}

/// Absolute now: R later runs in another folder.
fn resolve(script: Option<OsString>, paths: Option<OsString>) -> io::Result<PathBuf> {
    if let Some(path) = script {
        return std::path::absolute(path);
    }
    let paths = paths.unwrap_or_default();
    if let Some(found) = std::env::split_paths(&paths)
        .map(|dir| dir.join(NAME))
        .find(|path| path.is_file())
    {
        return std::path::absolute(found);
    }
    installed()?.ok_or_else(|| refused("table.r.missing"))
}

/// Apps opened from Finder see no Homebrew PATH.
#[cfg(target_os = "macos")]
fn installed() -> io::Result<Option<PathBuf>> {
    Ok([
        "/Library/Frameworks/R.framework/Resources/bin/Rscript",
        "/opt/homebrew/bin/Rscript",
        "/usr/local/bin/Rscript",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|path| path.is_file()))
}

/// Distributions put Rscript on PATH.
#[cfg(all(unix, not(target_os = "macos")))]
fn installed() -> io::Result<Option<PathBuf>> {
    Ok(None)
}

/// R's installer records its folder, not PATH.
#[cfg(windows)]
fn installed() -> io::Result<Option<PathBuf>> {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        if let Some(dir) = registry(root, "SOFTWARE\\R-core\\R", "InstallPath")? {
            let path = PathBuf::from(dir).join("bin").join(NAME);
            if path.is_file() {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
}

/// A string value; None when absent.
#[cfg(windows)]
fn registry(
    root: windows_sys::Win32::System::Registry::HKEY,
    key: &str,
    value: &str,
) -> io::Result<Option<std::ffi::OsString>> {
    use std::os::windows::ffi::OsStringExt as _;
    use windows_sys::Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
        System::Registry::{RRF_RT_REG_SZ, RegGetValueW},
    };
    let wide = |text: &str| text.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let (key, value) = (wide(key), wide(value));
    let mut size = 0u32;
    let mut buffer = Vec::<u16>::new();
    loop {
        // SAFETY: the buffer holds `size` bytes.
        let status = unsafe {
            RegGetValueW(
                root,
                key.as_ptr(),
                value.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                if buffer.is_empty() { std::ptr::null_mut() } else { buffer.as_mut_ptr().cast() },
                &mut size,
            )
        };
        match status {
            ERROR_FILE_NOT_FOUND => return Ok(None),
            ERROR_SUCCESS if !buffer.is_empty() => break,
            ERROR_SUCCESS => buffer = vec![0; (size as usize).div_ceil(2)],
            error => return Err(io::Error::from_raw_os_error(error as i32)),
        }
    }
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Ok(Some(std::ffi::OsString::from_wide(&buffer[..len])))
}

#[cfg(test)]
mod tests {
    use super::{NAME, resolve};

    #[test]
    fn relative_paths_resolve_before_r_moves() {
        let here = std::env::current_dir().expect("the folder");
        let named = resolve(Some("bin/Rscript".into()), None).expect("ELY_RSCRIPT");
        assert_eq!(named, here.join("bin/Rscript"));
        // A PATH entry relative to this folder.
        let dir = tempfile::tempdir_in(&here).expect("directory");
        std::fs::write(dir.path().join(NAME), "").expect("a stand-in");
        let entry = dir.path().file_name().expect("its name").to_owned();
        let found = resolve(None, Some(entry)).expect("found on PATH");
        assert!(found.is_absolute(), "{}", found.display());
        assert_eq!(found, dir.path().join(NAME));
    }

    #[cfg(windows)]
    #[test]
    fn a_registry_string_reads_whole_and_absence_is_none() {
        use windows_sys::Win32::System::Registry::HKEY_LOCAL_MACHINE;
        let root = super::registry(
            HKEY_LOCAL_MACHINE,
            "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",
            "SystemRoot",
        )
        .expect("read");
        assert_eq!(root, std::env::var_os("SystemRoot"));
        let absent = super::registry(HKEY_LOCAL_MACHINE, "SOFTWARE\\Ely\\Absent", "InstallPath");
        assert!(absent.expect("absence is no error").is_none());
    }
}
