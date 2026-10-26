use std::{
    io::BufRead,
    path::{Path, PathBuf},
};

use super::{ADMIT, MOVING, Moved, NAME, POINTER, State, at_start_in, choose_in, read};
use crate::{AppId, lock::FileLock};

/// A temp folder, spelled as the file system spells it.
fn folder() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dunce::canonicalize(dir.path()).expect("its real path");
    (dir, path)
}

/// A first folder named as the suite names it.
fn first() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let (base, path) = folder();
    let home = path.join(NAME);
    seed(&home);
    (base, path, home)
}

fn seed(root: &Path) {
    std::fs::create_dir_all(root.join("ely-mail/recovery")).expect("a mail folder");
    std::fs::write(root.join("ely-mail/mail.sqlite"), vec![7u8; 4096]).expect("a store");
    std::fs::write(root.join("ely-mail/recovery/draft"), b"ab").expect("a copy");
    std::fs::write(root.join("ely-mail/instance.lock"), b"").expect("a lock");
    std::fs::write(root.join("settings.json"), b"{}").expect("settings");
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("a folder")
        .map(|entry| entry.expect("an entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn start(home: &Path) -> Moved {
    at_start_in(home).expect("started").0
}

#[test]
fn a_chosen_folder_takes_the_data_at_the_next_start() {
    let (_base, _, home) = first();
    let (_away, away) = folder();
    let to = away.join(NAME);
    let asked = choose_in(&home, &away).expect("chosen");
    assert_eq!(asked.next.as_deref(), Some(to.as_path()));
    assert_eq!(read(&home).expect("read").at, home, "nothing moves before the start");
    let moved = start(&home);
    assert!(matches!(moved, Moved::Done { bytes: 4100, .. }), "{moved:?}");
    assert_eq!(std::fs::read(to.join("ely-mail/mail.sqlite")).expect("the store"), vec![7u8; 4096]);
    assert_eq!(std::fs::read(to.join("ely-mail/recovery/draft")).expect("the copy"), b"ab");
    assert!(!to.join("ely-mail/instance.lock").exists(), "a live file stays behind");
    assert!(!to.join(MOVING).exists(), "the mark goes with the copy done");
    assert_eq!(read(&home).expect("read"), State { at: to.clone(), next: None, failed: None });
    assert_eq!(names(&home), [ADMIT, ".settings.lock", POINTER], "only its own files stay");
}

#[test]
fn moving_back_to_the_first_folder_removes_the_other() {
    let (_base, path, home) = first();
    let (_away, away) = folder();
    choose_in(&home, &away).expect("chosen");
    start(&home);
    choose_in(&home, &path).expect("chosen back");
    let moved = start(&home);
    assert!(matches!(moved, Moved::Done { .. }), "{moved:?}");
    assert_eq!(read(&home).expect("read").at, home);
    assert!(home.join("ely-mail/mail.sqlite").exists(), "the data came back");
    assert!(!away.join(NAME).exists(), "the other folder went");
}

#[test]
fn another_spelling_of_the_first_folder_is_the_same_folder() {
    let (_base, path, home) = first();
    let spelled = PathBuf::from(home.to_string_lossy().to_uppercase());
    // Case-sensitive file systems have no other spelling.
    if spelled == home || !spelled.exists() {
        return;
    }
    let inside = choose_in(&spelled, &home.join("ely-mail")).expect_err("inside");
    assert_eq!(inside.kind(), std::io::ErrorKind::InvalidInput, "{inside}");
    let (_away, away) = folder();
    choose_in(&spelled, &away).expect("chosen");
    start(&spelled);
    choose_in(&spelled, &path).expect("chosen back");
    start(&spelled);
    assert_eq!(read(&spelled).expect("read").at, spelled, "back in the first folder");
    choose_in(&spelled, &away).expect("chosen again");
    let moved = start(&spelled);
    assert!(matches!(moved, Moved::Done { .. }), "{moved:?}");
    assert!(home.join(POINTER).exists(), "the pointer stays");
    assert_eq!(read(&spelled).expect("read").at, away.join(NAME));
}

#[test]
fn the_folder_in_use_calls_off_a_move_and_unfit_ones_are_refused() {
    let (_base, path, home) = first();
    let (_away, away) = folder();
    choose_in(&home, &away).expect("chosen");
    assert_eq!(choose_in(&home, &path).expect("called off").next, None);
    let inside = choose_in(&home, &home.join("ely-mail")).expect_err("inside");
    assert_eq!(inside.kind(), std::io::ErrorKind::InvalidInput, "{inside}");
    std::fs::create_dir_all(away.join(NAME)).expect("a folder");
    std::fs::write(away.join(NAME).join("notes.txt"), b"mine").expect("a note");
    let other = choose_in(&home, &away).expect_err("not Ely's");
    assert!(other.to_string().contains("not Ely's"), "{other}");
    assert_eq!(read(&home).expect("read").next, None);
}

#[test]
fn a_folder_with_ely_data_is_used_as_it_is() {
    let (_base, _, home) = first();
    let (_away, away) = folder();
    let to = away.join(NAME);
    std::fs::create_dir_all(&to).expect("a folder");
    std::fs::write(to.join("settings.json"), b"{\"mine\": 1}").expect("its settings");
    choose_in(&home, &away).expect("chosen");
    let moved = start(&home);
    assert!(matches!(moved, Moved::Used(_)), "{moved:?}");
    assert_eq!(std::fs::read(to.join("settings.json")).expect("kept"), b"{\"mine\": 1}");
    assert!(home.join("ely-mail/mail.sqlite").exists(), "the old data stays");
    assert_eq!(read(&home).expect("read").at, to);
}

#[test]
fn a_copy_cut_short_is_done_again() {
    let (_base, _, home) = first();
    let (_away, away) = folder();
    let to = away.join(NAME);
    std::fs::create_dir_all(to.join("ely-mail")).expect("a part");
    std::fs::write(to.join("ely-mail/mail.sqlite"), b"half").expect("half a store");
    std::fs::write(to.join(MOVING), b"").expect("the mark");
    choose_in(&home, &away).expect("chosen");
    let moved = start(&home);
    assert!(matches!(moved, Moved::Done { .. }), "{moved:?}");
    assert_eq!(std::fs::read(to.join("ely-mail/mail.sqlite")).expect("the store"), vec![7u8; 4096]);
    assert!(!to.join(MOVING).exists());
}

#[test]
fn a_failed_copy_keeps_the_data_and_says_why() {
    let (_base, _, home) = first();
    let (_away, away) = folder();
    choose_in(&home, &away.join("gone")).expect_err("no such folder");
    std::fs::create_dir(away.join("gone")).expect("a folder");
    choose_in(&home, &away.join("gone")).expect("chosen");
    std::fs::remove_dir(away.join("gone")).expect("gone");
    // A file where the folder should go.
    std::fs::write(away.join("gone"), b"").expect("a file");
    let moved = start(&home);
    assert!(matches!(moved, Moved::Failed(_)), "{moved:?}");
    let state = read(&home).expect("read");
    assert_eq!((state.at.as_path(), state.next.as_deref()), (home.as_path(), None));
    assert!(state.failed.is_some_and(|why| why.contains("gone")));
    assert!(home.join("ely-mail/mail.sqlite").exists(), "nothing is lost");
}

/// Keeps one source file from being copied while it lives.
#[cfg(windows)]
fn blocked(home: &Path) -> impl Sized {
    use std::os::windows::fs::OpenOptionsExt;
    let path = home.join("ely-mail/mail.sqlite");
    std::fs::OpenOptions::new().read(true).share_mode(0).open(path).expect("held alone")
}

#[cfg(unix)]
fn blocked(home: &Path) -> impl Sized {
    struct Link(PathBuf);
    impl Drop for Link {
        fn drop(&mut self) {
            std::fs::remove_file(&self.0).expect("unlinked");
        }
    }
    let link = home.join("ely-mail/link");
    std::os::unix::fs::symlink(home.join("settings.json"), &link).expect("a link");
    Link(link)
}

#[test]
fn a_failed_copy_into_an_empty_folder_leaves_it_empty() {
    let (_base, _, home) = first();
    let (_away, away) = folder();
    let to = away.join(NAME);
    std::fs::create_dir(&to).expect("an empty folder");
    choose_in(&home, &away).expect("chosen");
    let held = blocked(&home);
    let moved = start(&home);
    assert!(matches!(moved, Moved::Failed(_)), "{moved:?}");
    assert!(names(&to).is_empty(), "{:?}", names(&to));
    drop(held);
    choose_in(&home, &away).expect("chosen again");
    let moved = start(&home);
    assert!(matches!(moved, Moved::Done { .. }), "{moved:?}");
    assert_eq!(std::fs::read(to.join("ely-mail/mail.sqlite")).expect("the store"), vec![7u8; 4096]);
}

#[test]
fn a_start_keeps_others_out_until_it_lets_go() {
    let (_base, _, home) = first();
    let (_, admitted) = at_start_in(&home).expect("admitted");
    let door = home.join(ADMIT);
    assert!(FileLock::try_take(&door).expect("tried").is_none(), "a second start waits");
    drop(admitted);
    assert!(FileLock::try_take(&door).expect("tried").is_some(), "then it goes on");
}

#[test]
fn a_running_app_makes_the_move_wait() {
    let (_base, _, home) = first();
    let (_away, away) = folder();
    choose_in(&home, &away).expect("chosen");
    let exe = std::env::current_exe().expect("this test");
    let mut running = std::process::Command::new(&exe)
        .args(["--exact", "instance::tests::one_copy", "--nocapture"])
        .env("ELY_INSTANCE_DIR", home.join("ely-mail"))
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("a running copy");
    let out = std::io::BufReader::new(running.stdout.take().expect("its output"));
    let mut said = out.lines().map(|line| line.expect("a line"));
    assert!(said.by_ref().any(|line| line.contains("HELD")), "it did not start");
    let moved = start(&home);
    said.for_each(drop);
    assert!(running.wait().expect("ends").success());
    assert!(matches!(moved, Moved::Waits(AppId::Mail)), "{moved:?}");
    assert!(read(&home).expect("read").next.is_some(), "the move still waits");
    assert!(!away.join(NAME).exists(), "nothing moved");
}

/// Keeps `ely-mail` from being deleted while it lives.
#[cfg(windows)]
fn undeletable(home: &Path) -> Option<impl Sized> {
    use std::os::windows::fs::OpenOptionsExt;
    const SHARE_READ: u32 = 1;
    let path = home.join("ely-mail/mail.sqlite");
    Some(std::fs::OpenOptions::new().read(true).share_mode(SHARE_READ).open(path).expect("held"))
}

#[cfg(unix)]
fn undeletable(home: &Path) -> Option<impl Sized> {
    use std::os::unix::fs::PermissionsExt;
    struct Shut(PathBuf);
    impl Drop for Shut {
        fn drop(&mut self) {
            std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755)).expect("opened");
        }
    }
    // Root deletes whatever the permissions say.
    if unsafe { libc::geteuid() } == 0 {
        return None;
    }
    let dir = home.join("ely-mail");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).expect("shut");
    Some(Shut(dir))
}

#[test]
fn a_move_back_after_a_cleanup_cut_short_copies_again() {
    let (_base, path, home) = first();
    std::fs::create_dir_all(home.join("ely-doc/recovery")).expect("a doc folder");
    std::fs::write(home.join("ely-doc/recovery/point"), b"doc").expect("a point");
    let (_away, away) = folder();
    choose_in(&home, &away).expect("chosen");
    let Some(held) = undeletable(&home) else {
        return;
    };
    let moved = start(&home);
    drop(held);
    assert!(matches!(moved, Moved::Failed(_)), "{moved:?}");
    assert_eq!(read(&home).expect("read").at, away.join(NAME));
    choose_in(&home, &path).expect("chosen back");
    let moved = start(&home);
    assert!(matches!(moved, Moved::Done { .. }), "a half-emptied folder is no install: {moved:?}");
    assert_eq!(std::fs::read(home.join("ely-doc/recovery/point")).expect("the point"), b"doc");
    assert_eq!(std::fs::read(home.join("ely-mail/mail.sqlite")).expect("the store"), vec![7u8; 4096]);
}

/// A folder link the way each system makes one.
fn link(target: &Path, at: &Path) {
    #[cfg(windows)]
    {
        let made = std::process::Command::new("cmd").arg("/C").arg("mklink").arg("/J").arg(at).arg(target).output().expect("mklink");
        assert!(made.status.success(), "{}", String::from_utf8_lossy(&made.stderr));
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, at).expect("a link");
}

#[test]
fn a_link_into_the_data_folder_is_refused() {
    let (_base, _, home) = first();
    let (_away, away) = folder();
    // An app's own settings make it look like Ely data.
    std::fs::write(home.join("ely-mail/settings.json"), b"{}").expect("its settings");
    link(&home.join("ely-mail"), &away.join(NAME));
    let inside = choose_in(&home, &away).expect_err("inside");
    assert!(inside.to_string().contains("inside the data folder"), "{inside}");
    assert_eq!(read(&home).expect("read").next, None);
}

#[test]
fn a_chosen_folder_gone_missing_stops_the_start() {
    let (_base, _, home) = first();
    let (_away, away) = folder();
    choose_in(&home, &away).expect("chosen");
    start(&home);
    std::fs::rename(away.join(NAME), away.join("elsewhere")).expect("unplugged");
    let Err(error) = at_start_in(&home) else {
        panic!("it started without its data");
    };
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound, "{error}");
    assert!(error.to_string().contains(POINTER), "{error}");
    assert_eq!(read(&home).expect("read").at, away.join(NAME), "the pointer stays");
}

#[test]
fn a_link_made_after_the_choice_is_refused_at_start() {
    let (_base, _, home) = first();
    let (_away, away) = folder();
    choose_in(&home, &away).expect("chosen");
    std::fs::write(home.join("ely-mail/settings.json"), b"{}").expect("its settings");
    link(&home.join("ely-mail"), &away.join(NAME));
    let moved = start(&home);
    assert!(matches!(&moved, Moved::Failed(why) if why.contains("inside the data folder")), "{moved:?}");
    assert_eq!((read(&home).expect("read").at, read(&home).expect("read").next), (home.clone(), None));
    assert_eq!(std::fs::read(home.join("ely-mail/mail.sqlite")).expect("the store"), vec![7u8; 4096]);
}
