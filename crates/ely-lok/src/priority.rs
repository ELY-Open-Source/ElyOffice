//! An idle engine starts at background priority, works at normal.

use std::io;

/// Linux: the engine's own autogroup, low CPU and idle I/O.
#[cfg(target_os = "linux")]
mod os {
    use std::io;

    const GROUP: &str = "/proc/self/autogroup";
    /// ioprio_set: every thread of our process group.
    const WHO_PGRP: libc::c_int = 2;
    const CLASS_IDLE: libc::c_int = 3 << 13;
    const CLASS_NONE: libc::c_int = 0;

    pub(super) fn background() -> io::Result<()> {
        // Alone in a session, alone in an autogroup.
        if unsafe { libc::setsid() } < 0 {
            return Err(io::Error::last_os_error());
        }
        group("19")?;
        io_class(CLASS_IDLE)
    }

    pub(super) fn foreground() -> io::Result<()> {
        // Raising an autogroup back to 0 needs no privilege.
        group("0")?;
        io_class(CLASS_NONE)
    }

    /// The kernel takes one write per 100 ms, system-wide.
    fn group(nice: &str) -> io::Result<()> {
        let mut tries = 0;
        loop {
            match std::fs::write(GROUP, nice) {
                Err(error) if error.raw_os_error() == Some(libc::EAGAIN) && tries < 20 => {
                    tries += 1;
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                done => return done.map_err(|error| io::Error::new(error.kind(), format!("{GROUP}: {error}"))),
            }
        }
    }

    fn io_class(class: libc::c_int) -> io::Result<()> {
        if unsafe { libc::syscall(libc::SYS_ioprio_set, WHO_PGRP, 0, class) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

/// macOS: the process's darwin background state.
#[cfg(target_os = "macos")]
mod os {
    use std::io;

    pub(super) fn background() -> io::Result<()> {
        set(libc::PRIO_DARWIN_BG)
    }

    pub(super) fn foreground() -> io::Result<()> {
        set(0)
    }

    fn set(priority: libc::c_int) -> io::Result<()> {
        if unsafe { libc::setpriority(libc::PRIO_DARWIN_PROCESS, 0, priority) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

/// Windows: background mode, which lowers disk reads too.
#[cfg(windows)]
mod os {
    use std::io;

    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, PROCESS_MODE_BACKGROUND_BEGIN, PROCESS_MODE_BACKGROUND_END, SetPriorityClass,
    };

    pub(super) fn background() -> io::Result<()> {
        set(PROCESS_MODE_BACKGROUND_BEGIN)
    }

    pub(super) fn foreground() -> io::Result<()> {
        set(PROCESS_MODE_BACKGROUND_END)
    }

    fn set(mode: u32) -> io::Result<()> {
        if unsafe { SetPriorityClass(GetCurrentProcess(), mode) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

/// Lowers this process until an app takes it.
pub(crate) fn background() -> io::Result<()> {
    os::background().map_err(|error| io::Error::new(error.kind(), format!("background priority: {error}")))?;
    log::info!("lok-worker: background priority");
    Ok(())
}

/// Back to normal once an app works with it.
pub(crate) fn foreground() -> io::Result<()> {
    os::foreground().map_err(|error| io::Error::new(error.kind(), format!("normal priority: {error}")))?;
    log::info!("lok-worker: normal priority");
    Ok(())
}
