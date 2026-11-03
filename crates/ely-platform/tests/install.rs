//! The step after quitting: installs, then reopens the app.
#![cfg(any(windows, target_os = "linux"))]

use std::{
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use ely_platform::install::{Installed, after_quit};

/// A process that has already ended.
fn ended() -> u32 {
    let mut child = Command::new(if cfg!(windows) { "cmd" } else { "true" })
        .args(if cfg!(windows) { &["/c", "exit"][..] } else { &[][..] })
        .spawn()
        .expect("starts");
    child.wait().expect("ends");
    child.id()
}

/// Waits for the reopened app's mark.
fn reopened(mark: &Path) -> bool {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(60) {
        if mark.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

/// Programs under %TEMP% are blocked on some Windows hosts.
fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch dir")
}

#[cfg(windows)]
fn windows_install(setup: &str, note_blocked: bool) -> (tempfile::TempDir, std::path::PathBuf, bool) {
    let root = scratch();
    let dir = root.path().join("Ely Office").join("ElyTest");
    std::fs::create_dir_all(&dir).expect("the install folder");
    let setup_file = root.path().join("setup.cmd");
    std::fs::write(&setup_file, setup).expect("a fake setup");
    let exe = dir.join("reopen.cmd");
    std::fs::write(&exe, "@>\"%~dp0opened\" echo yes\r\n").expect("a fake app");
    let note = root.path().join("notes").join("failed.txt");
    if note_blocked {
        std::fs::write(root.path().join("notes"), "a file, not a folder").expect("blocks the note");
    }
    let installed = Installed::Folder { dir: dir.clone(), exe };
    let mut child = after_quit(&setup_file, &installed, ended(), &note, Stdio::null()).expect("starts");
    assert!(child.wait().expect("ends").success());
    let opened = reopened(&dir.join("opened"));
    (root, note, opened)
}

#[cfg(windows)]
#[test]
fn a_windows_setup_runs_silent_into_the_folder_then_the_app_reopens() {
    // Busy once, as when a worker still holds a file.
    let setup = "@if not exist \"%~dp0busy\" (echo.>\"%~dp0busy\" & exit /b 2)\r\n@>\"%~dp0args.txt\" echo %*\r\n@exit /b 0\r\n";
    let (root, note, opened) = windows_install(setup, false);
    assert!(opened, "the app reopens");
    let args = std::fs::read_to_string(root.path().join("args.txt")).expect("the setup ran twice");
    let dir = root.path().join("Ely Office").join("ElyTest");
    assert_eq!(args.trim(), format!("/S /D={}", dir.display()), "unquoted, as NSIS wants");
    assert!(!note.exists(), "no failure");
}

#[cfg(windows)]
#[test]
fn a_failed_windows_setup_leaves_a_note_and_still_reopens() {
    let (_root, note, opened) = windows_install("@exit /b 3\r\n", false);
    assert!(opened, "the old version reopens");
    let said = std::fs::read_to_string(&note).expect("a note");
    assert_eq!(said, "the setup exited with 3 (2: a file is in use)");
}

#[cfg(windows)]
#[test]
fn a_note_that_cannot_be_written_still_reopens() {
    let (_root, note, opened) = windows_install("@exit /b 3\r\n", true);
    assert!(opened, "the old version reopens");
    assert!(!note.exists());
}

#[cfg(target_os = "linux")]
#[test]
fn an_appimage_is_swapped_then_reopened() {
    use std::os::unix::fs::PermissionsExt as _;
    let root = scratch();
    let image = root.path().join("ElyDoc.AppImage");
    std::fs::write(&image, "old").expect("the old image");
    let new = root.path().join("new.AppImage");
    let mark = root.path().join("opened");
    std::fs::write(&new, format!("#!/bin/sh\necho yes > '{}'\n", mark.display())).expect("the new image");
    let note = root.path().join("notes/failed.txt");
    let installed = Installed::AppImage(image.clone());
    let mut child = after_quit(&new, &installed, ended(), &note, Stdio::null()).expect("starts");
    assert!(child.wait().expect("ends").success());
    assert!(reopened(&mark), "the new image runs");
    assert_eq!(std::fs::read(&image).expect("reads"), std::fs::read(&new).expect("reads"));
    let mode = std::fs::metadata(&image).expect("its mode").permissions().mode();
    assert_eq!(mode & 0o777, 0o755);
    assert!(!note.exists() && !root.path().join(".ElyDoc.AppImage.new").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn an_appimage_that_cannot_be_replaced_leaves_a_note() {
    let root = scratch();
    let image = root.path().join("missing-folder/ElyDoc.AppImage");
    let note = root.path().join("notes/failed.txt");
    let new = root.path().join("new.AppImage");
    std::fs::write(&new, "#!/bin/sh\n").expect("the new image");
    let installed = Installed::AppImage(image.clone());
    let mut child = after_quit(&new, &installed, ended(), &note, Stdio::null()).expect("starts");
    assert!(child.wait().expect("ends").success());
    let said = std::fs::read_to_string(&note).expect("a note");
    assert!(said.starts_with(&format!("{} was not replaced: ", image.display())), "{said}");
}

/// A real setup, from tests/scripts/windows-installer.sh.
#[cfg(windows)]
#[test]
fn a_real_setup_installs_over_its_folder() {
    let (Some(setup), Some(dir)) = (std::env::var_os("ELY_TEST_SETUP"), std::env::var_os("ELY_TEST_DIR")) else {
        eprintln!("SKIP: run from tests/scripts/windows-installer.sh");
        return;
    };
    let root = scratch();
    let exe = root.path().join("reopen.cmd");
    std::fs::write(&exe, "@>\"%~dp0opened\" echo yes\r\n").expect("a fake app");
    let note = root.path().join("failed.txt");
    let installed = Installed::Folder { dir: dir.into(), exe };
    let mut child = after_quit(Path::new(&setup), &installed, ended(), &note, Stdio::inherit()).expect("starts");
    assert!(child.wait().expect("ends").success());
    assert!(reopened(&root.path().join("opened")), "the app reopens");
    assert!(!note.exists(), "{:?}", std::fs::read_to_string(&note));
}
