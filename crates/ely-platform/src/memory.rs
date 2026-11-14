//! The machine's physical memory.

use std::io;

/// Bytes of physical memory.
#[cfg(unix)]
pub fn physical() -> io::Result<u64> {
    let pages = unsafe { libc::sysconf(libc::_SC_PHYS_PAGES) };
    let size = unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) };
    match (u64::try_from(pages), u64::try_from(size)) {
        (Ok(pages), Ok(size)) if pages > 0 && size > 0 => Ok(pages * size),
        _ => Err(io::Error::other(format!("no memory size: {}", io::Error::last_os_error()))),
    }
}

/// Bytes of physical memory.
#[cfg(windows)]
pub fn physical() -> io::Result<u64> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut status = MEMORYSTATUSEX { dwLength: size_of::<MEMORYSTATUSEX>() as u32, ..unsafe { std::mem::zeroed() } };
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(status.ullTotalPhys)
}

#[cfg(test)]
mod tests {
    #[test]
    fn this_machine_has_some_gigabytes() {
        let bytes = super::physical().expect("the memory size");
        assert!(bytes > 1 << 30, "{bytes} bytes");
    }
}
