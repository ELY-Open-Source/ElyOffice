//! File names compared as their folder compares them (§6.1).

use std::{ffi::OsStr, io, path::Path};

/// Whether resolved `a` and `b` name one entry.
pub fn same_name(a: &Path, b: &Path) -> io::Result<bool> {
    if a == b {
        return Ok(true);
    }
    let (Some(folder), Some(x), Some(y)) = (a.parent(), a.file_name(), b.file_name()) else {
        return Ok(false);
    };
    if b.parent() != Some(folder) || !folded_equal(x, y) {
        return Ok(false);
    }
    // Only a case variant asks the disk.
    Ok(!case_sensitive(folder)?)
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

/// Equal as APFS and HFS+ match names.
#[cfg(target_os = "macos")]
fn folded_equal(x: &OsStr, y: &OsStr) -> bool {
    use objc2_core_foundation::{CFComparisonResult, CFString, CFStringCompareFlags};
    let (Some(x), Some(y)) = (x.to_str(), y.to_str()) else {
        return false;
    };
    let flags = CFStringCompareFlags::CompareCaseInsensitive | CFStringCompareFlags::CompareNonliteral;
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

/// No API tells; names match exactly.
#[cfg(not(any(windows, target_os = "macos")))]
fn folded_equal(_: &OsStr, _: &OsStr) -> bool {
    false
}

#[cfg(not(any(windows, target_os = "macos")))]
fn case_sensitive(_: &Path) -> io::Result<bool> {
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::same_name;

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
}
