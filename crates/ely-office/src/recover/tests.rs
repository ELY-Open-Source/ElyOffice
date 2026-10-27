use super::{Keeping, Kept, Slot};

#[test]
fn a_failing_point_says_so_once_until_the_reason_changes() {
    let mut slot = Slot { kept: kept("s", false), written: None, writing: false, failed: None };
    let full = std::io::Error::other("disk full");
    assert_eq!(slot.fail(1, &full).as_deref(), Some("disk full"));
    assert_eq!(slot.fail(2, &full), None, "the same reason stays quiet");
    let big = std::io::Error::new(std::io::ErrorKind::InvalidInput, "over the cap");
    assert!(slot.fail(3, &big).is_some(), "a new reason shows");
    assert_eq!(slot.written, Some(3), "too big waits for the next edit");
}

fn kept(slot: &str, held: bool) -> Kept {
    Kept { slot: slot.into(), name: "a.docx".into(), format: "docx".into(), sealed: false, held }
}

#[cfg(unix)]
#[test]
fn a_failed_index_write_keeps_the_last_one() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut keeping = Keeping::start_in(dir.path().to_path_buf()).expect("starts");
    keeping.crashed.push(kept("1-0", false));
    keeping.save_index().expect("the first index");
    let recovery = dir.path().join("recovery");
    std::fs::set_permissions(&recovery, std::fs::Permissions::from_mode(0o555)).expect("locks");
    keeping.crashed.push(kept("1-1", false));
    let failed = keeping.save_index();
    std::fs::set_permissions(&recovery, std::fs::Permissions::from_mode(0o755)).expect("unlocks");
    assert!(failed.is_err(), "the folder refused the write");
    let read = super::read_index(dir.path()).expect("the old index still reads");
    assert_eq!(read.len(), 1);
}

#[test]
fn a_lock_of_bad_bytes_still_offers_the_points() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut keeping = Keeping::start_in(dir.path().to_path_buf()).expect("starts");
    keeping.crashed.push(kept("1-0", false));
    keeping.save_index().expect("the index");
    std::fs::write(dir.path().join("session.lock"), [0xff, 0xfe, 0]).expect("a torn lock");
    let again = Keeping::start_in(dir.path().to_path_buf()).expect("starts despite the lock");
    assert_eq!(again.crashed.len(), 1, "read as a crash");
    let lock = std::fs::read_to_string(dir.path().join("session.lock")).expect("rewritten");
    assert_eq!(lock, format!("open {}", std::process::id()));
}

#[test]
fn a_clean_end_keeps_only_what_never_reopened() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut keeping = Keeping::start_in(dir.path().to_path_buf()).expect("starts");
    for slot in ["1-0", "1-1"] {
        std::fs::create_dir_all(keeping.folder(slot).join("points")).expect("a slot");
    }
    keeping.crashed.push(kept("1-0", false));
    keeping.held.push(kept("1-1", false));
    keeping.save_index().expect("the index");
    keeping.end();
    let again = Keeping::start_in(dir.path().to_path_buf()).expect("starts again");
    let slots: Vec<&str> = again.crashed.iter().map(|kept| kept.slot.as_str()).collect();
    assert_eq!(slots, ["1-1"], "only the held slot comes back");
    assert!(again.folder("1-1").exists() && !again.folder("1-0").exists());
}

#[test]
fn a_quit_index_outlasts_a_background_write_begun_before_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut keeping = Keeping::start_in(dir.path().to_path_buf()).expect("starts");
    keeping.held.push(kept("1-0", false));
    // A background flush takes its index and turn first.
    let (path, bytes) = keeping.index().expect("the index");
    let turn = keeping.turns.take();
    keeping.held.push(kept("1-1", false));
    keeping.save_index().expect("the quit index");
    let late = keeping.turns.write(turn, || super::write_index(&path, &bytes));
    assert!(late.is_none(), "the older write never lands");
    assert_eq!(super::read_index(dir.path()).expect("reads").len(), 2);
}
