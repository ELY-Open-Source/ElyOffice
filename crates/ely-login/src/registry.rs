//! Where the setup put an app.

use std::{
    ffi::{OsStr, OsString},
    io,
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::PathBuf,
    ptr,
};

use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_SUCCESS},
    System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW},
};

/// `InstallDir` of app `name`; None when not installed.
pub(crate) fn install_dir(name: &str) -> io::Result<Option<PathBuf>> {
    let key = format!("Software\\Ely Office\\{name}");
    let wide = |text: &str| {
        OsStr::new(text)
            .encode_wide()
            .chain([0])
            .collect::<Vec<u16>>()
    };
    let (key_w, value_w) = (wide(&key), wide("InstallDir"));
    let mut size = 0u32;
    loop {
        let mut buffer = vec![0u16; size as usize / 2];
        let out = if buffer.is_empty() {
            ptr::null_mut()
        } else {
            buffer.as_mut_ptr()
        };
        let code = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key_w.as_ptr(),
                value_w.as_ptr(),
                RRF_RT_REG_SZ,
                ptr::null_mut(),
                out.cast(),
                &mut size,
            )
        };
        match code {
            ERROR_SUCCESS if !buffer.is_empty() => {
                let end = buffer
                    .iter()
                    .position(|unit| *unit == 0)
                    .unwrap_or(buffer.len());
                return Ok(Some(PathBuf::from(OsString::from_wide(&buffer[..end]))));
            }
            // Sized first; it may grow between reads.
            ERROR_SUCCESS | ERROR_MORE_DATA => {}
            ERROR_FILE_NOT_FOUND => return Ok(None),
            code => {
                return Err(io::Error::other(format!(
                    "HKCU\\{key}\\InstallDir: {}",
                    io::Error::from_raw_os_error(code as i32)
                )));
            }
        }
    }
}
