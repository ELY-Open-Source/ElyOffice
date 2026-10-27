use super::*;
use crate::crash::{Crash, Marker};

fn at(secs: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
}

fn key() -> [u8; 32] {
    [7u8; 32]
}

#[test]
fn a_point_lands_with_cursor_and_last_ok() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::new(dir.path().to_path_buf());
    assert_eq!(recovery.last_ok(), None);
    let path = recovery
        .write_once(b"body", Some("12:4"), at(100))
        .expect("the write lands")
        .expect("the task runs");
    assert_eq!(
        std::fs::read_to_string(&path).expect("the point reads"),
        "body"
    );
    assert_eq!(recovery.last_ok(), Some(at(100)));
    let points = recovery.points().expect("the points list");
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].cursor.as_deref(), Some("12:4"));
    assert!(!points[0].history, "undo history is not captured");
    assert!(!points[0].sealed);
    assert_eq!(points[0].at, at(100));
}

#[test]
fn a_running_task_skips_the_tick() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::new(dir.path().to_path_buf());
    recovery.running = true;
    let skipped = recovery
        .write_once(b"body", None, at(100))
        .expect("the skip");
    assert_eq!(skipped, None);
    assert_eq!(recovery.last_ok(), None);
}

#[test]
fn the_cadence_drives_ten_second_writes() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::new(dir.path().to_path_buf());
    assert!(recovery.tick(at(100)), "the first tick fires");
    assert!(!recovery.tick(at(109)), "nine seconds is too soon");
    assert!(recovery.tick(at(110)), "ten seconds is due");
    recovery
        .write_once(b"0", None, at(110))
        .expect("the write lands");
    assert!(recovery.tick(at(120)), "the cadence continues");
}

#[test]
fn only_twenty_points_stay() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::new(dir.path().to_path_buf());
    for n in 0..25u64 {
        recovery
            .write_once(b"x", None, at(100 + n))
            .expect("the write lands");
    }
    let points = recovery.points().expect("the points list");
    assert_eq!(points.len(), 20);
    assert_eq!(points.last().expect("an oldest").at, at(105));
}

#[test]
fn points_older_than_seven_days_go() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::new(dir.path().to_path_buf());
    recovery
        .write_once(b"old", None, at(100))
        .expect("the write lands");
    let week = 8 * 24 * 3600;
    recovery
        .write_once(b"new", None, at(100 + week))
        .expect("the write lands");
    let points = recovery.points().expect("the points list");
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].at, at(100 + week));
}

#[test]
fn reopening_prunes_without_a_new_write() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::new(dir.path().to_path_buf());
    recovery
        .write_once(b"old", None, at(100))
        .expect("the write lands");
    drop(recovery);
    let week = 8 * 24 * 3600;
    let recovery = Recovery::new(dir.path().to_path_buf());
    recovery
        .maintain(at(100 + week))
        .expect("the reopen maintain");
    assert!(recovery.points().expect("the points list").is_empty());
}

#[test]
fn the_byte_cap_drops_oldest() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::new(dir.path().to_path_buf()).with_limits(20, 12);
    for n in 0..5u64 {
        recovery
            .write_once(b"12345", None, at(100 + n))
            .expect("the write lands");
    }
    let points = recovery.points().expect("the points list");
    assert_eq!(points.len(), 2);
    assert_eq!(points.last().expect("an oldest").at, at(103));
}

#[test]
fn an_oversize_point_fails_and_keeps_the_previous() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::new(dir.path().to_path_buf()).with_limits(20, 4);
    recovery
        .write_once(b"old!", None, at(100))
        .expect("the write lands");
    let refused = recovery.write_once(b"large", None, at(200));
    assert!(refused.is_err());
    assert_eq!(recovery.last_ok(), Some(at(100)));
    let points = recovery.points().expect("the points list");
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].at, at(100));
}

#[test]
fn the_cap_counts_the_cursor_too() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::new(dir.path().to_path_buf()).with_limits(20, 8);
    let refused = recovery.write_once(b"a", Some("12345678"), at(100));
    assert!(refused.is_err());
    assert_eq!(recovery.last_ok(), None);
}

#[cfg(unix)]
#[test]
fn a_failed_write_keeps_last_ok_honest() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::new(dir.path().to_path_buf());
    recovery
        .write_once(b"0", None, at(100))
        .expect("the write lands");
    let points = dir.path().join("points");
    // A file in the folder's place stops root too.
    let aside = dir.path().join("aside");
    std::fs::rename(&points, &aside).expect("the points move aside");
    std::fs::write(&points, "in the way").expect("the file");
    let failed = recovery.write_once(b"1", None, at(200));
    std::fs::remove_file(&points).expect("the file goes");
    std::fs::rename(&aside, &points).expect("the points return");
    assert!(failed.is_err());
    assert_eq!(recovery.last_ok(), Some(at(100)));
    let listed = recovery.points().expect("the points list");
    assert_eq!(listed.len(), 1, "a partial write is never listed");
}

#[test]
fn dangling_metadata_is_no_point() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let points = dir.path().join("points");
    std::fs::create_dir_all(&points).expect("the points dir");
    std::fs::write(points.join("100.cursor"), "12:4").expect("the dangling cursor");
    std::fs::write(points.join("200.nonce"), [0u8; 12]).expect("the dangling nonce");
    let recovery = Recovery::new(dir.path().to_path_buf());
    assert!(recovery.points().expect("the points list").is_empty());
}

#[test]
fn a_recovery_copy_lands_beside_never_over() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::new(dir.path().to_path_buf());
    recovery
        .write_once(b"body", None, at(100))
        .expect("the write lands");
    let points = recovery.points().expect("the points list");
    let dest = dir.path().join("restored.md");
    restore_copy(&points[0], &dest).expect("the copy lands");
    assert_eq!(
        std::fs::read_to_string(&dest).expect("the copy reads"),
        "body"
    );
    assert!(points[0].content.try_exists().expect("the point stays"));
    let refused = restore_copy(&points[0], &dest);
    assert!(refused.is_err(), "an existing target is never overwritten");
}

#[cfg(unix)]
#[test]
fn a_restore_never_follows_links() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::new(dir.path().to_path_buf());
    recovery
        .write_once(b"body", None, at(100))
        .expect("the write lands");
    let points = recovery.points().expect("the points list");
    let other = dir.path().join("other.md");
    write(&other, "other");
    let linked = dir.path().join("linked.md");
    std::os::unix::fs::symlink(&other, &linked).expect("the symlink");
    assert!(restore_copy(&points[0], &linked).is_err());
    assert_eq!(
        std::fs::read_to_string(&other).expect("the original reads"),
        "other"
    );
}

#[cfg(unix)]
fn write(path: &Path, body: &str) {
    std::fs::write(path, body).expect("the fixture file writes");
}

#[test]
fn a_sealed_point_hides_everything_but_restores_with_the_key() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::sensitive(dir.path().to_path_buf(), key());
    recovery
        .write_once(b"secret content", Some("12:4"), at(100))
        .expect("the write lands");
    let points = recovery.points().expect("the points list");
    assert_eq!(points.len(), 1);
    assert!(points[0].sealed);
    assert_eq!(points[0].cursor.as_deref(), Some("12:4"));
    let on_disk = std::fs::read(&points[0].content).expect("the sealed bytes");
    let text = String::from_utf8_lossy(&on_disk);
    assert!(!text.contains("secret"), "no plaintext on disk");
    assert!(!text.contains("12:4"), "no plaintext cursor on disk");
    let dest = dir.path().join("restored.md");
    recovery
        .restore(&points[0], &dest)
        .expect("the restore lands");
    assert_eq!(
        std::fs::read_to_string(&dest).expect("the restore reads"),
        "secret content"
    );
    assert!(restore_copy(&points[0], &dir.path().join("wrong.md")).is_err());
}

#[test]
fn a_sealed_point_needs_its_key_to_restore() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::sensitive(dir.path().to_path_buf(), key());
    recovery
        .write_once(b"secret", None, at(100))
        .expect("the write lands");
    let points = recovery.points().expect("the points list");
    let keyless = Recovery::new(dir.path().to_path_buf());
    let dest = dir.path().join("restored.md");
    assert!(keyless.restore(&points[0], &dest).is_err());
    assert!(!dest.try_exists().expect("no unprotected copy"));
}

#[test]
fn a_discard_drops_points_and_records_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::new(dir.path().to_path_buf());
    recovery
        .write_once(b"0", None, at(100))
        .expect("the write lands");
    recovery.discard().expect("the discard lands");
    assert!(recovery.discarded().expect("the marker"));
    assert!(recovery.points().expect("the points list").is_empty());
    assert_eq!(recovery.last_ok(), None);
    recovery
        .write_once(b"1", None, at(200))
        .expect("the write lands");
    assert!(!recovery.discarded().expect("the marker clears"));
}

#[test]
fn a_torn_lock_counts_as_a_crash() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let torn: [&[u8]; 9] = [
        b"",
        b"op",
        b"open x",
        b"open 7\xff",
        b"open 7 \xff",
        b"open 7 8",
        b"discarded \xff",
        b"discarded",
        b"\xff\xfe\x00",
    ];
    for torn in torn {
        std::fs::write(dir.path().join("session.lock"), torn).expect("writes");
        assert_eq!(Crash::check(dir.path(), |_| true).expect("reads"), Marker::Crashed, "{torn:?}");
    }
    // Opening again rewrites it whole.
    Crash::open(dir.path(), 7).expect("opens");
    assert_eq!(Crash::check(dir.path(), |pid| pid == 7).expect("reads"), Marker::Clean);
}

#[test]
fn the_lock_tells_crash_from_discard() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let lock = Crash::open(dir.path(), 4242).expect("the lock opens");
    assert_eq!(
        Crash::check(dir.path(), |pid| pid == 4242).expect("the check"),
        Marker::Clean
    );
    assert_eq!(
        Crash::check(dir.path(), |pid| pid != 4242).expect("the check"),
        Marker::Crashed
    );
    lock.discard().expect("the discard lands");
    assert_eq!(
        Crash::check(dir.path(), |_| false).expect("the check"),
        Marker::Discarded
    );
    let lock = Crash::open(dir.path(), 4242).expect("the lock reopens");
    lock.close().expect("the clean close");
    assert_eq!(
        Crash::check(dir.path(), |_| false).expect("the check"),
        Marker::Clean
    );
}

#[test]
fn the_scheduler_writes_every_interval_and_stops() {
    use std::sync::atomic::{AtomicU64, Ordering};
    let dir = tempfile::tempdir().expect("a temp dir");
    let ticks = std::sync::Arc::new(AtomicU64::new(0));
    let snapshot = {
        let ticks = std::sync::Arc::clone(&ticks);
        move || {
            let n = ticks.fetch_add(1, Ordering::Relaxed);
            (format!("tick {n}").into_bytes(), None)
        }
    };
    let task = crate::RecoveryTask::start(
        Recovery::new(dir.path().to_path_buf()),
        Duration::from_millis(10),
        snapshot,
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while ticks.load(Ordering::Relaxed) < 3 && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(task.last_ok().is_some(), "the scheduler wrote a point");
    task.stop();
    let points = Recovery::new(dir.path().to_path_buf())
        .points()
        .expect("the points list");
    assert!(points.len() >= 2, "several ticks landed: {}", points.len());
    let content = std::fs::read_to_string(&points[0].content).expect("the newest reads");
    assert!(content.starts_with("tick "));
}

#[test]
fn a_dropped_task_never_writes_after_a_discard() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let slow = || {
        std::thread::sleep(Duration::from_millis(100));
        (b"tick".to_vec(), None)
    };
    let task = crate::RecoveryTask::start(
        Recovery::new(dir.path().to_path_buf()),
        Duration::from_millis(10),
        slow,
    );
    std::thread::sleep(Duration::from_millis(150));
    drop(task);
    let mut recovery = Recovery::new(dir.path().to_path_buf());
    recovery.discard().expect("the discard lands");
    assert!(recovery.discarded().expect("the marker stays"));
    assert_eq!(recovery.points().expect("the points list").len(), 0);
    std::thread::sleep(Duration::from_millis(150));
    assert!(recovery.discarded().expect("the marker survives"));
    assert_eq!(recovery.points().expect("the points list").len(), 0);
}

#[test]
fn last_ok_reads_free_during_a_slow_write() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let slow = || {
        std::thread::sleep(Duration::from_millis(200));
        (b"tick".to_vec(), None)
    };
    let task = crate::RecoveryTask::start(
        Recovery::new(dir.path().to_path_buf()),
        Duration::from_millis(10),
        slow,
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while task.last_ok().is_none() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(task.last_ok().is_some(), "the first point landed");
    std::thread::sleep(Duration::from_millis(30));
    let began = std::time::Instant::now();
    let read = task.last_ok();
    let took = began.elapsed();
    assert!(read.is_some());
    assert!(
        took < Duration::from_millis(150),
        "the read never waits for disk: {took:?}"
    );
    task.stop();
}

#[test]
fn the_task_status_starts_from_the_recovery() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut recovery = Recovery::new(dir.path().to_path_buf());
    recovery
        .write_once(b"0", None, at(100))
        .expect("the write lands");
    let task = crate::RecoveryTask::start(recovery, Duration::from_secs(60), || {
        (b"tick".to_vec(), None)
    });
    assert_eq!(task.last_ok(), Some(at(100)));
    task.stop();
}

#[test]
fn keeping_sealed_points_opens_none_of_them() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut old = Recovery::sensitive(dir.path().to_path_buf(), key()).with_limits(2, u64::MAX);
    old.write_once(b"one", Some("1:1"), at(100)).expect("lands");
    // Another run's key cannot open the older points.
    let mut now = Recovery::sensitive(dir.path().to_path_buf(), [9u8; 32]).with_limits(2, u64::MAX);
    now.write_once(b"two", Some("2:2"), at(200)).expect("pruning reads names only");
    now.write_once(b"three", None, at(300)).expect("and drops the oldest unread");
    assert_eq!(now.stamps().expect("names").len(), 2);
}
