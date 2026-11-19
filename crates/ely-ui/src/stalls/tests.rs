use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use ely_platform::cpu::thread_time;

use super::{Beat, Late};

/// Runs `span` of this thread's CPU; other tests share it.
fn busy(span: Duration) {
    let before = thread_time().expect("a thread time");
    while thread_time().expect("a thread time") - before < span {
        std::hint::black_box(0u64);
    }
}

#[test]
fn a_busy_loop_is_busy_and_an_on_time_wait_is_nothing() {
    let beat = Beat::start().expect("a beat");
    busy(Duration::from_millis(300));
    let late = beat.end(Duration::from_millis(20)).expect("a thread time").expect("late");
    assert!(matches!(late, Late::Busy { ran, .. } if ran >= Duration::from_millis(300)), "{late:?}");
    assert!(late.to_string().starts_with("the main thread was busy "), "{late}");
    let beat = Beat::start().expect("a beat");
    // The wait itself, on time.
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(beat.end(Duration::from_millis(20)).expect("a thread time"), None);
}

#[test]
fn a_thread_blocked_on_a_lock_stalls() {
    let lock = Arc::new(Mutex::new(()));
    let (held, told) = mpsc::channel();
    let holder = {
        let lock = lock.clone();
        std::thread::spawn(move || {
            let _guard = lock.lock().expect("the lock");
            held.send(()).expect("told");
            // Holds the lock while the watched thread waits.
            std::thread::sleep(Duration::from_millis(600));
        })
    };
    told.recv().expect("the lock is held");
    let beat = Beat::start().expect("a beat");
    drop(lock.lock().expect("the lock"));
    let late = beat.end(Duration::ZERO).expect("a thread time").expect("late");
    holder.join().expect("the holder ends");
    assert!(!matches!(late, Late::Starved { .. }), "a block is a stall: {late}");
    #[cfg(target_os = "linux")]
    assert!(matches!(late, Late::Blocked { .. }) && late.to_string().starts_with("the main thread was blocked "), "{late}");
}

#[cfg(target_os = "linux")]
#[test]
fn a_thread_kept_off_its_cpu_is_starved_not_a_stall() {
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Pins the calling thread to `cpu`.
    fn pin(cpu: usize) {
        let mut set: libc::cpu_set_t = unsafe { std::mem::zeroed() };
        unsafe { libc::CPU_SET(cpu, &mut set) };
        assert_eq!(unsafe { libc::sched_setaffinity(0, size_of::<libc::cpu_set_t>(), &set) }, 0);
    }
    let mut set: libc::cpu_set_t = unsafe { std::mem::zeroed() };
    assert_eq!(unsafe { libc::sched_getaffinity(0, size_of::<libc::cpu_set_t>(), &mut set) }, 0);
    let cpu = (0..libc::CPU_SETSIZE as usize).find(|&at| unsafe { libc::CPU_ISSET(at, &set) }).expect("a CPU");
    pin(cpu);
    // Three busy threads share the watched thread's CPU.
    let spinning = Arc::new(AtomicBool::new(true));
    let hogs: Vec<_> = (0..3)
        .map(|_| {
            let spinning = spinning.clone();
            std::thread::spawn(move || {
                pin(cpu);
                while spinning.load(Ordering::Relaxed) {
                    std::hint::black_box(0u64);
                }
            })
        })
        .collect();
    let beat = Beat::start().expect("a beat");
    busy(Duration::from_millis(80));
    let late = beat.end(Duration::from_millis(20)).expect("a thread time");
    spinning.store(false, Ordering::Relaxed);
    hogs.into_iter().for_each(|hog| hog.join().expect("a busy thread ends"));
    let late = late.expect("late behind three busy threads");
    assert!(matches!(late, Late::Starved { .. }), "{late}");
    assert!(late.to_string().ends_with(": starved"), "{late}");
}
