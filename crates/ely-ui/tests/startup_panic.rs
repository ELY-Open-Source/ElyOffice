//! Harness-less: GPUI must run on the main thread.
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

use ely_platform::{AppId, logging, startup};

const CHILD: &str = "ELY_STARTUP_CHILD";
const DIR: &str = "ELY_STARTUP_DIR";

fn main() {
    match std::env::var(CHILD).as_deref() {
        Ok("launch") => {
            boot();
            gpui_platform::application().run(|_| panic!("fault: renderer"));
        }
        Ok("late") => {
            boot();
            startup::started();
            panic!("fault: an hour in");
        }
        _ => {
            a_panic_inside_gpuis_launch_callback_is_reported_as_a_failed_start();
            a_panic_after_the_start_is_an_ordinary_panic();
            println!("startup_panic: 2 cases pass");
        }
    }
}

fn boot() {
    logging::init_in(AppId::Doc, &PathBuf::from(std::env::var_os(DIR).unwrap())).unwrap();
    startup::guard(AppId::Doc);
}

fn child(mode: &str) -> (Output, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        // Headless on Linux: the host's display must not decide.
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .env_remove("DISPLAY")
        .env(CHILD, mode)
        .env(DIR, dir.path())
        .env("ELY_NO_DIALOG", "1")
        .output()
        .unwrap();
    (output, dir)
}

fn a_panic_inside_gpuis_launch_callback_is_reported_as_a_failed_start() {
    let (output, dir) = child("launch");

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ElyDoc could not start") && stderr.contains("fault: renderer"),
        "{stderr}"
    );
    let log = fs::read_to_string(dir.path().join("ely.log")).unwrap();
    assert!(
        log.contains("ElyDoc cannot start: the app panicked before its first frame"),
        "{log}"
    );
}

fn a_panic_after_the_start_is_an_ordinary_panic() {
    let (output, dir) = child("late");

    assert_eq!(output.status.code(), Some(101), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("fault: an hour in") && !stderr.contains("could not start"),
        "{stderr}"
    );
    let log = fs::read_to_string(dir.path().join("ely.log")).unwrap();
    assert!(
        log.contains("panic:") && !log.contains("cannot start"),
        "{log}"
    );
}
