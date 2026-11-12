//! Who holds a file open, by Windows' Restart Manager.

use std::{os::windows::ffi::OsStrExt as _, path::Path};

use windows_sys::Win32::Foundation::ERROR_MORE_DATA;
use windows_sys::Win32::System::RestartManager::{
    CCH_RM_SESSION_KEY, RM_PROCESS_INFO, RmEndSession, RmGetList, RmRegisterResources, RmStartSession,
};

/// The processes holding `path`, named with their ids.
pub(super) fn holders(path: &Path) -> String {
    let mut session = 0u32;
    let mut key = [0u16; CCH_RM_SESSION_KEY as usize + 1];
    let started = unsafe { RmStartSession(&mut session, 0, key.as_mut_ptr()) };
    if started != 0 {
        return format!("holders unknown: Restart Manager error {started}");
    }
    let named = listed(session, path);
    unsafe { RmEndSession(session) };
    match named {
        Ok(names) if names.is_empty() => "no process holds it now".to_string(),
        Ok(names) => format!("held by {}", names.join(", ")),
        Err(code) => format!("holders unknown: Restart Manager error {code}"),
    }
}

/// Every holder of `path`, however many.
fn listed(session: u32, path: &Path) -> Result<Vec<String>, u32> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let files = [wide.as_ptr()];
    let registered = unsafe {
        RmRegisterResources(session, 1, files.as_ptr(), 0, std::ptr::null(), 0, std::ptr::null())
    };
    if registered != 0 {
        return Err(registered);
    }
    // Asked empty first: the answer says how many to hold.
    let mut infos = Vec::new();
    for _ in 0..8 {
        let (mut needed, mut count, mut reasons) = (0u32, infos.len() as u32, 0u32);
        let got = unsafe { RmGetList(session, &mut needed, &mut count, infos.as_mut_ptr(), &mut reasons) };
        if got == 0 {
            infos.truncate(count as usize);
            return Ok(infos.iter().map(named).collect());
        }
        if got != ERROR_MORE_DATA {
            return Err(got);
        }
        // More holders came between asks: ask again for all.
        infos = vec![RM_PROCESS_INFO::default(); needed as usize];
    }
    Err(ERROR_MORE_DATA)
}

/// `name (pid)`; this process is marked.
fn named(info: &RM_PROCESS_INFO) -> String {
    let end = info.strAppName.iter().position(|&unit| unit == 0).unwrap_or(info.strAppName.len());
    let name = String::from_utf16_lossy(&info.strAppName[..end]);
    let pid = info.Process.dwProcessId;
    match pid == std::process::id() {
        true => format!("{name} ({pid}, this worker)"),
        false => format!("{name} ({pid})"),
    }
}

#[cfg(test)]
mod tests {
    use std::os::windows::fs::OpenOptionsExt as _;

    #[test]
    // One holder already overflows the empty first ask.
    fn a_file_held_without_delete_sharing_names_its_holder() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let path = dir.path().join("lu1.tmp");
        std::fs::write(&path, "x").expect("written");
        let held = std::fs::OpenOptions::new().read(true).share_mode(0).open(&path).expect("held");
        assert_eq!(std::fs::remove_file(&path).expect_err("held").raw_os_error(), Some(32));
        let named = super::holders(&path);
        assert!(named.contains(&format!("({}, this worker)", std::process::id())), "{named}");
        drop(held);
        assert_eq!(super::holders(&path), "no process holds it now");
    }
}
