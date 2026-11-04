//! File names compared as their folder compares them (§6.1).

use std::{ffi::OsStr, io, path::Path};

/// Whether resolved `a` and `b` name one entry.
pub fn same_name(a: &Path, b: &Path) -> io::Result<bool> {
    same_name_with(a, b, case_sensitive)
}

/// As `same_name`, the volume's case probe handed in.
fn same_name_with(a: &Path, b: &Path, sensitive: impl FnOnce(&Path) -> io::Result<bool>) -> io::Result<bool> {
    if a == b {
        return Ok(true);
    }
    let (Some(folder), Some(x), Some(y)) = (a.parent(), a.file_name(), b.file_name()) else {
        return Ok(false);
    };
    if b.parent() != Some(folder) {
        return Ok(false);
    }
    if equivalent(x, y) {
        return Ok(true);
    }
    if !folded_equal(x, y) {
        return Ok(false);
    }
    // Only a case variant asks the disk.
    Ok(!sensitive(folder)?)
}

/// Equal as NTFS matches names.
#[cfg(windows)]
fn folded_equal(x: &OsStr, y: &OsStr) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Globalization::{CSTR_EQUAL, CompareStringOrdinal};
    let (x, y): (Vec<u16>, Vec<u16>) = (x.encode_wide().collect(), y.encode_wide().collect());
    let (Ok(x_len), Ok(y_len)) = (i32::try_from(x.len()), i32::try_from(y.len())) else {
        return false;
    };
    unsafe { CompareStringOrdinal(x.as_ptr(), x_len, y.as_ptr(), y_len, 1) == CSTR_EQUAL }
}

/// The folder's own flag; NTFS folders fold by default.
#[cfg(windows)]
fn case_sensitive(folder: &Path) -> io::Result<bool> {
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    use windows_sys::Win32::Foundation::{ERROR_INVALID_FUNCTION, ERROR_INVALID_PARAMETER, ERROR_NOT_SUPPORTED};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_CASE_SENSITIVE_INFO, FILE_FLAG_BACKUP_SEMANTICS, FILE_READ_ATTRIBUTES, FileCaseSensitiveInfo,
        GetFileInformationByHandleEx,
    };
    use windows_sys::Win32::System::SystemServices::FILE_CS_FLAG_CASE_SENSITIVE_DIR;
    let dir = std::fs::OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(folder)?;
    let mut info = FILE_CASE_SENSITIVE_INFO { Flags: 0 };
    let size = std::mem::size_of::<FILE_CASE_SENSITIVE_INFO>() as u32;
    let read = unsafe {
        GetFileInformationByHandleEx(dir.as_raw_handle(), FileCaseSensitiveInfo, (&raw mut info).cast(), size)
    };
    if read == 0 {
        let error = io::Error::last_os_error();
        // FAT and the like have no such flag.
        return match error.raw_os_error().map(|code| code as u32) {
            Some(ERROR_INVALID_PARAMETER | ERROR_NOT_SUPPORTED | ERROR_INVALID_FUNCTION) => Ok(false),
            _ => Err(error),
        };
    }
    Ok(info.Flags & FILE_CS_FLAG_CASE_SENSITIVE_DIR != 0)
}

/// One name on APFS and HFS+, whatever the case mode.
#[cfg(target_os = "macos")]
fn equivalent(x: &OsStr, y: &OsStr) -> bool {
    compared(x, y, objc2_core_foundation::CFStringCompareFlags::CompareNonliteral)
}

/// Equal as case-insensitive APFS and HFS+ match names.
#[cfg(target_os = "macos")]
fn folded_equal(x: &OsStr, y: &OsStr) -> bool {
    use objc2_core_foundation::CFStringCompareFlags;
    compared(x, y, CFStringCompareFlags::CompareCaseInsensitive | CFStringCompareFlags::CompareNonliteral)
}

#[cfg(target_os = "macos")]
fn compared(x: &OsStr, y: &OsStr, flags: objc2_core_foundation::CFStringCompareFlags) -> bool {
    use objc2_core_foundation::{CFComparisonResult, CFString};
    let (Some(x), Some(y)) = (x.to_str(), y.to_str()) else {
        return false;
    };
    CFString::from_str(x).compare(Some(&CFString::from_str(y)), flags) == CFComparisonResult::CompareEqualTo
}

#[cfg(target_os = "macos")]
fn case_sensitive(folder: &Path) -> io::Result<bool> {
    use std::os::unix::ffi::OsStrExt;
    let folder = std::ffi::CString::new(folder.as_os_str().as_bytes())?;
    match unsafe { libc::pathconf(folder.as_ptr(), libc::_PC_CASE_SENSITIVE) } {
        -1 => Err(io::Error::last_os_error()),
        sensitive => Ok(sensitive != 0),
    }
}

/// NTFS and Linux keep names as written.
#[cfg(not(target_os = "macos"))]
fn equivalent(_: &OsStr, _: &OsStr) -> bool {
    false
}

/// Equal as casefold folders match: NFD, then case.
#[cfg(not(any(windows, target_os = "macos")))]
fn folded_equal(x: &OsStr, y: &OsStr) -> bool {
    use std::os::unix::ffi::OsStrExt as _;
    use unicode_normalization::UnicodeNormalization as _;
    let (Some(x), Some(y)) = (x.to_str(), y.to_str()) else {
        return x.as_bytes().eq_ignore_ascii_case(y.as_bytes());
    };
    let (x, y): (String, String) = (x.nfd().collect(), y.nfd().collect());
    x.to_lowercase() == y.to_lowercase() || x.to_uppercase() == y.to_uppercase()
}

/// Asks the folder: is an entry's case variant itself?
#[cfg(not(any(windows, target_os = "macos")))]
fn case_sensitive(folder: &Path) -> io::Result<bool> {
    use std::os::unix::ffi::OsStrExt as _;
    for entry in std::fs::read_dir(folder)? {
        let name = entry?.file_name();
        let other: Vec<u8> = name.as_bytes().iter().map(swapped).collect();
        if other == name.as_bytes() {
            continue;
        }
        let Some(had) = identity(&folder.join(&name))? else { continue };
        let found = identity(&folder.join(OsStr::from_bytes(&other)))?;
        // Replaced meanwhile: ask another entry.
        if identity(&folder.join(&name))? != Some(had) {
            continue;
        }
        return Ok(found != Some(had));
    }
    Err(io::Error::other(format!("no name in {} tells whether case folds", folder.display())))
}

#[cfg(not(any(windows, target_os = "macos")))]
fn swapped(byte: &u8) -> u8 {
    match byte.is_ascii_lowercase() {
        true => byte.to_ascii_uppercase(),
        false => byte.to_ascii_lowercase(),
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn identity(path: &Path) -> io::Result<Option<(u64, u64)>> {
    use std::os::unix::fs::MetadataExt as _;
    match std::fs::symlink_metadata(path) {
        Ok(found) => Ok(Some((found.dev(), found.ino()))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::{same_name, same_name_with};

    #[test]
    fn composed_and_decomposed_names_match_where_apple_volumes_do() {
        let folder = std::path::Path::new("/notes");
        let (composed, decomposed) = (folder.join("ely-\u{e9}.md"), folder.join("ely-e\u{301}.md"));
        // A case-sensitive volume still treats them as one name.
        let same = same_name_with(&composed, &decomposed, |_| Ok(true)).expect("compares");
        assert_eq!(same, cfg!(target_os = "macos"));
        let upper = folder.join("ELY-E\u{301}.md");
        assert!(!same_name_with(&composed, &upper, |_| Ok(true)).expect("compares"));
        // Linux casefold normalizes too; NTFS does not.
        let folds = same_name_with(&composed, &upper, |_| Ok(false)).expect("compares");
        assert_eq!(folds, !cfg!(windows));
    }

    #[test]
    fn case_variants_match_as_the_volume_does() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let folder = dunce::canonicalize(dir.path()).expect("the real folder");
        std::fs::write(folder.join("probe.md"), "").expect("a probe");
        let folds = folder.join("PROBE.md").exists();
        let (lower, upper) = (folder.join("b.md"), folder.join("B.md"));
        // Neither exists: only the names are compared.
        assert_eq!(same_name(&lower, &upper).expect("compares"), folds);
        assert!(same_name(&lower, &lower).expect("compares"));
        assert!(!same_name(&lower, &folder.join("c.md")).expect("compares"));
        assert!(!same_name(&lower, &folder.join("x").join("B.md")).expect("compares"));
        // Two Save As targets, neither written yet.
        assert_eq!(crate::same_file(&lower, &upper).expect("compares"), folds);
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    #[test]
    fn linux_folds_as_casefold_folders_do() {
        let folded = |x: &str, y: &str| super::folded_equal(x.as_ref(), y.as_ref());
        assert!(folded("Stra\u{df}e.md", "STRASSE.MD"));
        assert!(folded("\u{c9}t\u{e9}.md", "e\u{301}TE\u{301}.md"));
        assert!(!folded("a.md", "b.md"));
        let dir = tempfile::tempdir().expect("a temp dir");
        let (lower, upper) = (dir.path().join("b.md"), dir.path().join("B.md"));
        // Nothing in the folder tells; no guess.
        assert!(same_name(&lower, &upper).is_err());
        std::fs::write(dir.path().join("1"), "").expect("a caseless name");
        assert!(same_name(&lower, &upper).is_err());
    }
}
