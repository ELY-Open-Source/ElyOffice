//! CPU time the calling thread has used, and waited for.

use std::{io, time::Duration};

/// User and system CPU time this thread has run.
#[cfg(unix)]
pub fn thread_time() -> io::Result<Duration> {
    let mut at = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    if unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut at) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Duration::new(at.tv_sec as u64, at.tv_nsec as u32))
}

/// User and system CPU time this thread has run.
#[cfg(windows)]
pub fn thread_time() -> io::Result<Duration> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentThread, GetThreadTimes};
    let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut made, mut ended, mut kernel, mut user) = (zero, zero, zero, zero);
    if unsafe { GetThreadTimes(GetCurrentThread(), &mut made, &mut ended, &mut kernel, &mut user) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // FILETIME counts 100 ns ticks.
    let ticks = |at: FILETIME| (u64::from(at.dwHighDateTime) << 32) | u64::from(at.dwLowDateTime);
    Ok(Duration::from_nanos((ticks(kernel) + ticks(user)) * 100))
}

/// Time this thread spent runnable, off a CPU; Linux only.
#[cfg(target_os = "linux")]
pub fn thread_waited() -> io::Result<Option<Duration>> {
    // Fields: ns on a CPU, ns waiting to run, slices.
    let stat = std::fs::read_to_string("/proc/thread-self/schedstat")?;
    let waited = stat.split_whitespace().nth(1).and_then(|field| field.parse::<u64>().ok());
    match waited {
        Some(ns) => Ok(Some(Duration::from_nanos(ns))),
        None => Err(io::Error::new(io::ErrorKind::InvalidData, format!("schedstat reads {stat:?}"))),
    }
}

/// Time this thread spent runnable, off a CPU; Linux only.
#[cfg(not(target_os = "linux"))]
pub fn thread_waited() -> io::Result<Option<Duration>> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::thread_time;

    #[test]
    fn running_counts_and_sleeping_does_not() {
        // Other tests share the CPU: run by this clock.
        let (before, until) = (thread_time().expect("a thread time"), Instant::now() + Duration::from_secs(10));
        while thread_time().expect("a thread time") - before < Duration::from_millis(150) {
            assert!(Instant::now() < until, "a busy loop ran 10 s, its clock stood");
            std::hint::black_box(0u64);
        }
        let before = thread_time().expect("a thread time");
        // Off the CPU: the clock must stand still.
        std::thread::sleep(Duration::from_millis(200));
        let slept = thread_time().expect("a thread time") - before;
        assert!(slept < Duration::from_millis(20), "sleeping ran {slept:?}");
    }
}
