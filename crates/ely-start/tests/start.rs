//! The launcher's errors, early exits and the app's close.
#![cfg(windows)]

use std::{
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

/// The launcher, an icon and `app` as `probe.exe`, if given.
fn folder(app: Option<&Path>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temp dir");
    std::fs::copy(env!("CARGO_BIN_EXE_ely-start"), dir.path().join("ely-start.exe")).expect("the launcher copies");
    let icon = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/ely-doc.ico");
    std::fs::copy(icon, dir.path().join("probe.ico")).expect("the icon copies");
    if let Some(app) = app {
        std::fs::copy(app, dir.path().join("probe.exe")).expect("the app copies");
    }
    dir
}

fn launch(dir: &Path, args: &[&str]) -> Child {
    Command::new(dir.join("ely-start.exe"))
        .args(["probe", "Probe"])
        .args(args)
        .env("ELY_NO_DIALOG", "1")
        .stderr(Stdio::piped())
        .spawn()
        .expect("the launcher starts")
}

/// Its exit, within a minute.
fn ended(child: &mut Child) -> ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().expect("the launcher is waited on") {
            return status;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("the launcher still runs after a minute");
}

#[test]
fn a_missing_app_is_named_and_fails() {
    let dir = folder(None);
    let mut launcher = launch(dir.path(), &[]);
    assert_eq!(ended(&mut launcher).code(), Some(1));
    let mut said = String::new();
    std::io::Read::read_to_string(&mut launcher.stderr.take().expect("stderr is piped"), &mut said).expect("stderr reads");
    assert!(said.contains("Probe could not start") && said.contains("probe.exe"), "{said}");
}

#[test]
fn an_app_that_exits_ends_the_splash() {
    let dir = folder(Some(Path::new(r"C:\Windows\System32\whoami.exe")));
    let mut launcher = launch(dir.path(), &[]);
    assert_eq!(ended(&mut launcher).code(), Some(0));
}

#[test]
fn the_app_closes_the_splash_and_keeps_running() {
    let me = std::env::current_exe().expect("the test exe");
    let dir = folder(Some(&me));
    let mut launcher = Command::new(dir.path().join("ely-start.exe"))
        .args(["probe", "Probe", "child_closes_the_splash", "--exact", "--nocapture"])
        .env("ELY_START_PROBE", dir.path())
        .spawn()
        .expect("the launcher starts");
    assert_eq!(ended(&mut launcher).code(), Some(0));
    assert!(dir.path().join("closed").exists(), "the app asked the splash to close");
    assert!(!dir.path().join("left").exists(), "the app still runs");
    std::fs::write(dir.path().join("done"), "").expect("the app is told to go");
    let deadline = Instant::now() + Duration::from_secs(60);
    while !dir.path().join("left").exists() {
        assert!(Instant::now() < deadline, "the app never left");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The app's side, run by the launcher.
#[test]
fn child_closes_the_splash() {
    let Some(dir) = std::env::var_os("ELY_START_PROBE") else {
        return;
    };
    let dir = Path::new(&dir);
    assert!(std::env::var_os("ELY_SPLASH").is_some(), "the launcher names its splash");
    ely_platform::startup::close_splash();
    std::fs::write(dir.join("closed"), "").expect("the mark writes");
    let deadline = Instant::now() + Duration::from_secs(60);
    while !dir.join("done").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    std::fs::write(dir.join("left"), "").expect("the mark writes");
}

/// Past the splash's own limit and its fade.
const LATE: u64 = ely_start::scene::LONGEST + 1500;

/// The launcher running `child` after `after` ms; its stderr piped.
fn late_launch(child: &str, after: u64) -> (tempfile::TempDir, Child) {
    let me = std::env::current_exe().expect("the test exe");
    let dir = folder(Some(&me));
    let launcher = Command::new(dir.path().join("ely-start.exe"))
        .args(["probe", "Probe", child, "--exact", "--nocapture"])
        .env("ELY_START_AFTER", after.to_string())
        .env("ELY_NO_DIALOG", "1")
        .stderr(Stdio::piped())
        .spawn()
        .expect("the launcher starts");
    (dir, launcher)
}

fn failed_with_its_reason(mut launcher: Child) {
    assert_eq!(ended(&mut launcher).code(), Some(3), "the app's exit code passes on");
    let mut said = String::new();
    std::io::Read::read_to_string(&mut launcher.stderr.take().expect("stderr is piped"), &mut said).expect("stderr reads");
    assert!(said.contains("the engine did not start"), "the app's own reason: {said}");
    assert!(said.contains("Probe could not start") && said.contains("before its window showed"), "{said}");
}

#[test]
fn an_app_that_fails_before_its_window_fails_the_launcher() {
    failed_with_its_reason(late_launch("child_fails", 0).1);
}

#[test]
fn an_app_that_fails_after_the_splash_hid_still_fails_the_launcher() {
    let began = Instant::now();
    let (_dir, launcher) = late_launch("child_fails", LATE);
    failed_with_its_reason(launcher);
    assert!(began.elapsed() >= Duration::from_millis(LATE), "the launcher waited for the app");
}

#[test]
fn an_app_that_shows_after_the_splash_hid_passes() {
    let began = Instant::now();
    let (_dir, mut launcher) = late_launch("child_shows", LATE);
    assert_eq!(ended(&mut launcher).code(), Some(0));
    assert!(began.elapsed() >= Duration::from_millis(LATE), "the launcher waited for the first frame");
}

/// The app's wait, if the launcher set one.
fn after() -> Option<Duration> {
    let ms = std::env::var("ELY_START_AFTER").ok()?;
    Some(Duration::from_millis(ms.parse().expect("ELY_START_AFTER is ms")))
}

/// The app's side: it fails before its first frame.
#[test]
fn child_fails() {
    let Some(wait) = after() else {
        return;
    };
    // The app's slow start, as the launcher asked.
    std::thread::sleep(wait);
    eprintln!("probe: the engine did not start");
    std::process::exit(3);
}

/// The app's side: its first frame comes late.
#[test]
fn child_shows() {
    let Some(wait) = after() else {
        return;
    };
    // The app's slow start, as the launcher asked.
    std::thread::sleep(wait);
    ely_platform::startup::close_splash();
    // Stays up past its first frame, like a real app.
    std::thread::sleep(Duration::from_secs(2));
}
