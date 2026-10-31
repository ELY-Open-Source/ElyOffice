use super::*;
use std::sync::{Arc, Barrier, Mutex};

struct FakeBackend {
    stored: Mutex<Option<Vec<u8>>>,
    get_delay: Option<Duration>,
    fail_set: Mutex<bool>,
}

impl FakeBackend {
    /// Makes the next `set` fail once.
    fn fail_next_set(&self) {
        *self.fail_set.lock().expect("the flag") = true;
    }
}

impl Backend for FakeBackend {
    fn get(&self) -> io::Result<Option<Vec<u8>>> {
        if let Some(delay) = self.get_delay {
            std::thread::sleep(delay);
        }
        Ok(self.stored.lock().expect("the store").clone())
    }

    fn set(&self, key: &[u8]) -> io::Result<()> {
        let mut fail = self.fail_set.lock().expect("the flag");
        if *fail {
            *fail = false;
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "the store is locked",
            ));
        }
        *self.stored.lock().expect("the store") = Some(key.to_vec());
        Ok(())
    }
}

struct RefusingBackend;

impl Backend for RefusingBackend {
    fn get(&self) -> io::Result<Option<Vec<u8>>> {
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "locked"))
    }

    fn set(&self, _: &[u8]) -> io::Result<()> {
        unreachable!("a locked store never takes a key")
    }
}

#[test]
fn two_loaders_share_one_key() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let backend = Arc::new(FakeBackend {
        stored: Mutex::new(None),
        get_delay: Some(Duration::from_millis(30)),
        fail_set: Mutex::new(false),
    });
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let (backend, barrier, dir) = (
            Arc::clone(&backend),
            Arc::clone(&barrier),
            dir.path().to_path_buf(),
        );
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            load_with(backend.as_ref() as &dyn Backend, &dir).expect("the load")
        }));
    }
    let a = handles.pop().expect("one handle").join().expect("a joins");
    let b = handles
        .pop()
        .expect("another handle")
        .join()
        .expect("b joins");
    assert_eq!(a, b, "both processes hold the one persisted key");
    assert_eq!(a.len(), 32);
}

#[test]
fn a_locked_store_fails_loud() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let refused = load_with(&RefusingBackend as &dyn Backend, dir.path());
    assert!(refused.is_err());
}

#[test]
fn the_lock_holder_rechecks_before_generating() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let backend = Arc::new(FakeBackend {
        stored: Mutex::new(None),
        get_delay: Some(Duration::from_millis(50)),
        fail_set: Mutex::new(false),
    });
    let (a_backend, b_backend) = (Arc::clone(&backend), backend);
    let dir_a = dir.path().to_path_buf();
    let dir_b = dir.path().to_path_buf();
    let a = std::thread::spawn(move || {
        load_with(a_backend.as_ref() as &dyn Backend, &dir_a).expect("the load")
    });
    std::thread::sleep(Duration::from_millis(10));
    let b = load_with(b_backend.as_ref() as &dyn Backend, &dir_b).expect("the load");
    let a = a.join().expect("a joins");
    assert_eq!(a, b, "the second loader returns the first key");
}

#[test]
fn a_slow_init_times_out_the_waiter_but_keeps_its_key() {
    let dir = tempfile::tempdir().expect("a temp dir");
    struct SlowBackend(Mutex<Option<Vec<u8>>>);
    impl Backend for SlowBackend {
        fn get(&self) -> io::Result<Option<Vec<u8>>> {
            Ok(self.0.lock().expect("the store").clone())
        }
        fn set(&self, key: &[u8]) -> io::Result<()> {
            std::thread::sleep(Duration::from_millis(1500));
            *self.0.lock().expect("the store") = Some(key.to_vec());
            Ok(())
        }
    }
    let backend = Arc::new(SlowBackend(Mutex::new(None)));
    let (a_backend, dir_a) = (Arc::clone(&backend), dir.path().to_path_buf());
    let a = std::thread::spawn(move || {
        load_with(a_backend.as_ref() as &dyn Backend, &dir_a).expect("the load")
    });
    std::thread::sleep(Duration::from_millis(100));
    let timed_out = load_with(backend.as_ref() as &dyn Backend, dir.path());
    let a_key = a.join().expect("a joins");
    assert!(timed_out.is_err(), "a live holder is never overthrown");
    assert_eq!(
        backend
            .0
            .lock()
            .expect("the store")
            .as_ref()
            .expect("a key")
            .len(),
        32
    );
    let again = load_with(backend.as_ref() as &dyn Backend, dir.path()).expect("the reload");
    assert_eq!(again, a_key, "the slow initializer's key survives");
}

#[test]
fn a_failed_init_releases_the_lock_for_a_retry() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let backend = Arc::new(FakeBackend {
        stored: Mutex::new(None),
        get_delay: None,
        fail_set: Mutex::new(false),
    });
    backend.fail_next_set();
    let first = load_with(backend.as_ref() as &dyn Backend, dir.path());
    assert!(first.is_err(), "the failing set propagates");
    let second = load_with(backend.as_ref() as &dyn Backend, dir.path()).expect("the retry");
    assert_eq!(second.len(), 32);
}

#[test]
fn the_lock_lives_at_a_stable_home_path() {
    let home = Path::new("/home/u");
    let dir = shared_dir(home);
    #[cfg(target_os = "macos")]
    assert_eq!(dir, home.join("Library/Application Support/ElyOffice"));
    #[cfg(all(unix, not(target_os = "macos")))]
    assert_eq!(dir, home.join(".local/share/ElyOffice"));
    #[cfg(windows)]
    assert_eq!(dir, home.join("AppData/Local/ElyOffice"));
}

#[test]
fn a_mid_init_key_is_never_visible_to_another_loader() {
    struct StagedBackend {
        visible: Mutex<Option<Vec<u8>>>,
        persist_delay: Duration,
        armed: Mutex<bool>,
    }
    impl Backend for StagedBackend {
        fn get(&self) -> io::Result<Option<Vec<u8>>> {
            Ok(self.visible.lock().expect("the store").clone())
        }
        fn set(&self, key: &[u8]) -> io::Result<()> {
            *self.visible.lock().expect("the store") = Some(key.to_vec());
            std::thread::sleep(self.persist_delay);
            let mut armed = self.armed.lock().expect("the flag");
            if *armed {
                *armed = false;
                *self.visible.lock().expect("the store") = None;
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "rolled back",
                ));
            }
            Ok(())
        }
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let backend = Arc::new(StagedBackend {
        visible: Mutex::new(None),
        persist_delay: Duration::from_millis(120),
        armed: Mutex::new(true),
    });
    let (a_backend, dir_a) = (Arc::clone(&backend), dir.path().to_path_buf());
    let a = std::thread::spawn(move || load_with(a_backend.as_ref() as &dyn Backend, &dir_a));
    // B starts once a holds the lock, mid-write.
    let until = std::time::Instant::now() + Duration::from_secs(10);
    while backend.visible.lock().expect("the store").is_none() {
        assert!(std::time::Instant::now() < until, "a never began its write");
        std::thread::sleep(Duration::from_millis(1));
    }
    let b = load_with(backend.as_ref() as &dyn Backend, dir.path()).expect("b loads");
    assert!(a.join().expect("a joins").is_err(), "a's init rolls back");
    assert_eq!(b.len(), 32, "b initialized its own committed key");
    let persisted = backend
        .visible
        .lock()
        .expect("the store")
        .clone()
        .expect("a key");
    assert_eq!(
        b.as_slice(),
        persisted.as_slice(),
        "b's key is the persisted one"
    );
}

#[cfg(unix)]
#[test]
fn the_account_home_holds_under_concurrent_queries() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let want = super::account_home().expect("the account home");
    let stop = Arc::new(AtomicBool::new(false));
    let other = {
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let _ = unsafe { libc::getpwuid(0) };
            }
        })
    };
    for _ in 0..2000 {
        assert_eq!(super::account_home().expect("the account home"), want);
    }
    stop.store(true, Ordering::Relaxed);
    other.join().expect("the query thread joins");
}

#[cfg(unix)]
#[test]
fn the_account_home_ignores_the_env() {
    let here = super::account_home().expect("the account home");
    let out = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--exact",
            "key::tests::print_account_home",
            "--nocapture",
            "--include-ignored",
        ])
        .env("HOME", "/nonexistent-home")
        .output()
        .expect("the child runs");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let child = stdout
        .lines()
        .find(|line| line.starts_with('/'))
        .expect("the child prints its account home")
        .to_string();
    assert_eq!(child, here.display().to_string());
}

#[cfg(unix)]
#[test]
#[ignore]
fn print_account_home() {
    println!(
        "{}",
        super::account_home().expect("the account home").display()
    );
}

#[test]
fn without_a_store_the_key_lasts_one_session_only() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let first = RecoveryKey::from(load_with(&RefusingBackend, dir.path()));
    let again = RecoveryKey::from(load_with(&RefusingBackend, dir.path()));
    assert!(!first.lasting, "the store refused, so nothing lasts");
    assert_eq!(first.key, again.key, "one key for the whole run");
    let names: Vec<_> = std::fs::read_dir(dir.path()).expect("the dir").flatten().map(|e| e.file_name()).collect();
    assert!(names.iter().all(|name| name.to_string_lossy().ends_with(".lock")), "no key on disk: {names:?}");
    let stored = RecoveryKey::from(Ok([7; 32]));
    assert!(stored.lasting);
}
