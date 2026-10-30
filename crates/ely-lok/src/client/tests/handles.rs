//! Closed documents leave no handles behind (PERF-08).

use std::os::windows::io::AsRawHandle as _;

use super::*;

/// Load and close cycles after the first ones fill caches.
const CYCLES: usize = 20;

/// Opened once on first use, plus the thread pool's churn.
const ONCE: u32 = 8;

fn handles(worker: &Worker) -> u32 {
    let mut count = 0;
    let done = unsafe {
        windows_sys::Win32::System::Threading::GetProcessHandleCount(worker.child.as_raw_handle(), &mut count)
    };
    assert_ne!(done, 0, "{}", std::io::Error::last_os_error());
    count
}

#[test]
fn closed_documents_leave_no_handles() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker spawns");
    let source = dir.path().join("note.fodt");
    write(&source, FODT);
    let url = crate::file_url(&source);
    let cycle = |worker: &mut Worker| {
        let doc = worker.load(&url).expect("loads");
        worker.close_doc(doc).expect("closes");
    };
    for _ in 0..3 {
        cycle(&mut worker);
    }
    let before = handles(&worker);
    let counts: Vec<u32> = (0..CYCLES)
        .map(|_| {
            cycle(&mut worker);
            handles(&worker)
        })
        .collect();
    if let Err(why) = settled(before, &counts) {
        panic!("{why}");
    }
}

/// Settled as the bench judges: no climb at the end.
fn settled(before: u32, counts: &[u32]) -> Result<(), String> {
    let quarter = (counts.len() / 4).max(1);
    let (prior, late) = (&counts[counts.len() - 2 * quarter..counts.len() - quarter], &counts[counts.len() - quarter..]);
    let median = |part: &[u32]| {
        let mut sorted = part.to_vec();
        sorted.sort_unstable();
        f64::from(sorted[sorted.len() / 2] + sorted[(sorted.len() - 1) / 2]) / 2.0
    };
    let (low, high) = (late.iter().min().expect("a quarter"), prior.iter().max().expect("a quarter"));
    if low > high && median(late) > median(prior) {
        return Err(format!("the handles climb: {counts:?}"));
    }
    if *low > before + ONCE {
        return Err(format!("{before} handles became {counts:?}"));
    }
    Ok(())
}

#[test]
fn a_climb_after_an_early_peak_fails() {
    let mut climb = vec![108; 5];
    climb.extend([100; 10]);
    climb.extend([101, 102, 103, 104, 105]);
    assert!(settled(100, &climb).is_err());
    let mut jitter = vec![108; 5];
    jitter.extend([100, 101, 100, 102, 100, 101, 100, 102, 100, 101, 100, 101, 100, 102, 100]);
    assert_eq!(settled(100, &jitter), Ok(()));
    assert!(settled(100, &[109; 20]).is_err(), "kept past ONCE");
}
