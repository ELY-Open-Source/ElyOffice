//! An idle engine waits, checked, and one app takes it.

use super::*;

/// An app name no other test run shares.
fn app(test: &str) -> String {
    format!("test-{test}-{}", std::process::id())
}

#[test]
fn an_app_takes_the_checked_idle_engine_once() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let (profile, app) = (dir.path().join("profile"), app("take"));
    let probe = crate::Probe::default();
    let none = Worker::adopt(&install, &profile, &app, probe).err().expect("no engine waits yet");
    assert_eq!(none.kind(), io::ErrorKind::NotFound, "{none}");
    let log = std::fs::File::create(dir.path().join("engine.log")).expect("the log");
    let mut starter = Worker::spawn_mode(&install, &profile, false, Some((&endpoint::idle_name(&app), log)))
        .expect("the engine starts");
    // Its checker is connected; the check has not run.
    starter.wire().expect("the checker connects");
    assert!(endpoint::idle_waits(&profile).expect("the lock reads"), "the engine holds the idle lock");
    // A missing kernel id fails at once, lock or not.
    let bare = dir.path().join("no-kernel");
    let missing = endpoint::adopt(&endpoint::idle_name(&app), &bare, &profile).err().expect("no kernel id");
    assert_eq!(missing.kind(), io::ErrorKind::NotFound, "{missing}");
    // An app waits on the public name through the check.
    let (kernel, folder, name) = (install.clone(), profile.clone(), app.clone());
    let taker = std::thread::spawn(move || Worker::adopt(&kernel, &folder, &name, probe));
    starter.ready().expect("the checker, not the app, runs the check");
    assert!(endpoint::idle_waits(&profile).expect("the lock reads"), "probing leaves the lock held");
    #[cfg(target_os = "linux")]
    {
        let pid = starter.child.get().expect("the engine process").id();
        let idle = std::fs::read_to_string(format!("/proc/{pid}/autogroup")).expect("the engine's autogroup");
        assert!(idle.ends_with("nice 19\n"), "idle, it waits at background priority: {idle}");
    }
    let pid = starter.release().expect("the engine is released");
    #[cfg(target_os = "linux")]
    let group = || std::fs::read_to_string(format!("/proc/{pid}/autogroup")).expect("the engine's autogroup");
    let mut worker = taker.join().expect("the taker ran").expect("the app takes the engine");
    worker.child.resolve();
    assert!(matches!(worker.child, Process::Adopted(taken, ..) if taken == pid), "the released engine");
    #[cfg(target_os = "linux")]
    assert!(group().ends_with("nice 0\n"), "taken, it works at normal priority: {}", group());
    assert!(!endpoint::idle_waits(&profile).expect("the lock reads"), "the idle lock is free");
    let again = Worker::adopt(&install, &profile, &app, probe).err().expect("no second take");
    // Windows keeps the name while the taken pipes live.
    let held = if cfg!(windows) { io::ErrorKind::ResourceBusy } else { io::ErrorKind::NotFound };
    assert_eq!(again.kind(), held, "{again}");
    let source = dir.path().join("note.fodt");
    write(&source, FODT);
    let doc = worker.load(&crate::file_url(&source)).expect("the document loads");
    worker.close_doc(doc).expect("the document closes");
    worker.shutdown().expect("the engine shuts down");
}

#[test]
fn a_take_adopts_a_waiting_engine_and_else_starts_its_own() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let (profile, app) = (dir.path().join("profile"), app("take-or-start"));
    let probe = crate::Probe::default();
    let log = std::fs::File::create(dir.path().join("engine.log")).expect("the log");
    let start = crate::Start::wait(&profile).expect("the start");
    let pid = Worker::idle(&start, &install, probe, &app, log)
        .and_then(Worker::release)
        .expect("an engine waits");
    drop(start);
    let mut taken = Worker::take(&install, &profile, &app, probe).expect("the take starts");
    taken.ready().expect("the taken engine is ready");
    taken.child.resolve();
    assert!(matches!(taken.child, Process::Adopted(got, ..) if got == pid), "the waiting engine is taken");
    let mut own = Worker::take(&install, &profile, &app, probe).expect("the take starts");
    own.ready().expect("its own engine passes the check");
    own.child.resolve();
    assert!(matches!(own.child, Process::Up(_)), "none waits, so it starts its own");
    for mut worker in [taken, own] {
        let source = dir.path().join("note.fodt");
        write(&source, FODT);
        let doc = worker.load(&crate::file_url(&source)).expect("the document loads");
        worker.close_doc(doc).expect("the document closes");
        worker.shutdown().expect("the engine shuts down");
    }
}

#[test]
fn an_engine_whose_checker_never_connects_exits_and_frees_the_lock() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let (profile, app) = (dir.path().join("profile"), app("orphan"));
    let start = (install, profile.clone(), false);
    let private = endpoint::name(0).expect("a name");
    let (mut command, _) = command::build(&start, &private, Some(&endpoint::idle_name(&app))).expect("the command");
    let ((stdout, stdin), _) = process::pipes(&mut command, None).expect("the pipes");
    let mut child = command.spawn().expect("the engine starts");
    drop(command);
    let mut line = String::new();
    std::io::BufRead::read_line(&mut std::io::BufReader::new(stdout), &mut line).expect("its line");
    assert!(endpoint::idle_waits(&profile).expect("the lock reads"), "it holds the idle lock");
    // The checker dies before it connects.
    drop(stdin);
    let status = child.wait().expect("the engine exits");
    assert!(!status.success(), "{status}");
    assert!(!endpoint::idle_waits(&profile).expect("the lock reads"), "the idle lock is free");
}
