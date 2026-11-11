//! The launcher: a missing app, an early exit, the app's close.
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
