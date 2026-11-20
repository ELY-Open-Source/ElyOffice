//! Each start stage's time and the OS's read counters (R20.74).

use std::{
    io,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

/// The OS's counters for this process, since it began.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Usage {
    /// Faults that read the disk; on Windows, every fault.
    faults: u64,
    /// Reads from storage: blocks on Unix, bytes on Windows.
    input: u64,
}

#[cfg(unix)]
const SAID: (&str, &str) = ("page-ins", "block reads");
#[cfg(windows)]
const SAID: (&str, &str) = ("page faults, soft and hard", "bytes read");

/// A moment and the counters then, if readable.
pub(super) struct Mark {
    at: Instant,
    usage: Option<Usage>,
}

impl Mark {
    pub(super) fn now() -> Self {
        let usage = usage().map_err(|error| log::warn!("lok-worker: start counters unread: {error}")).ok();
        Self { at: Instant::now(), usage }
    }
}

/// The start's last stage; none once the self-check painted.
static LAST: Mutex<Option<Mark>> = Mutex::new(None);
static FIRST_DOCUMENT: AtomicBool = AtomicBool::new(true);

/// The worker began; stages count from here.
pub(super) fn begin() {
    match LAST.lock() {
        Ok(mut held) => *held = Some(Mark::now()),
        Err(_) => log::warn!("lok-worker: start stages not logged: a panic held them"),
    }
}

/// Logs `name`'s share of the start; `last` ends it.
pub(super) fn step(name: &str, last: bool) {
    let Ok(mut held) = LAST.lock() else {
        return log::warn!("lok-worker: start {name} not logged: a panic held the stages");
    };
    let Some(before) = held.take() else {
        return;
    };
    let after = Mark::now();
    log::info!("{}", line(name, &before, &after));
    if !last {
        *held = Some(after);
    }
}

/// Logs the first document's load, from `began`.
pub(super) fn first_document(began: &Mark) {
    if FIRST_DOCUMENT.swap(false, Ordering::Relaxed) {
        log::info!("{}", line("first document loaded", began, &Mark::now()));
    }
}

fn line(name: &str, before: &Mark, after: &Mark) -> String {
    let ms = after.at.duration_since(before.at).as_millis();
    let counted = match (before.usage, after.usage) {
        (Some(b), Some(a)) => match (a.faults.checked_sub(b.faults), a.input.checked_sub(b.input)) {
            (Some(faults), Some(input)) => format!("{faults} {}, {input} {}", SAID.0, SAID.1),
            _ => "counters went back".to_string(),
        },
        _ => "counters unread".to_string(),
    };
    format!("lok-worker: start {name} in {ms} ms, {counted}")
}

#[cfg(unix)]
fn usage() -> io::Result<Usage> {
    let mut ru = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    // SAFETY: a zeroed rusage the call fills.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, ru.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: getrusage succeeded and filled it.
    let ru = unsafe { ru.assume_init() };
    Ok(Usage { faults: ru.ru_majflt as u64, input: ru.ru_inblock as u64 })
}

#[cfg(windows)]
fn usage() -> io::Result<Usage> {
    use windows_sys::Win32::System::{
        ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Threading::{GetCurrentProcess, GetProcessIoCounters, IO_COUNTERS},
    };
    let mut memory = PROCESS_MEMORY_COUNTERS { cb: size_of::<PROCESS_MEMORY_COUNTERS>() as u32, ..unsafe { std::mem::zeroed() } };
    let mut io_counters: IO_COUNTERS = unsafe { std::mem::zeroed() };
    // SAFETY: this process's pseudo-handle, structs of the sizes given.
    unsafe {
        if K32GetProcessMemoryInfo(GetCurrentProcess(), &mut memory, memory.cb) == 0 {
            return Err(io::Error::last_os_error());
        }
        if GetProcessIoCounters(GetCurrentProcess(), &mut io_counters) == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(Usage { faults: u64::from(memory.PageFaultCount), input: io_counters.ReadTransferCount })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(faults: u64, input: u64) -> Mark {
        Mark { at: Instant::now(), usage: Some(Usage { faults, input }) }
    }

    #[test]
    fn a_stage_says_what_it_added() {
        let said = line("kernel hooked", &at(10, 100), &at(15, 160));
        assert!(said.starts_with("lok-worker: start kernel hooked in "), "{said}");
        assert!(said.ends_with(&format!("ms, 5 {}, 60 {}", SAID.0, SAID.1)), "{said}");
    }

    #[test]
    fn unread_or_falling_counters_say_so() {
        let unread = Mark { at: Instant::now(), usage: None };
        assert!(line("x", &unread, &at(1, 1)).ends_with("counters unread"));
        assert!(line("x", &at(5, 5), &at(4, 6)).ends_with("counters went back"));
    }

    #[test]
    fn this_process_s_counters_read() {
        let first = usage().expect("the counters");
        let second = usage().expect("the counters");
        assert!(second.faults >= first.faults && second.input >= first.input, "{first:?} then {second:?}");
    }
}
