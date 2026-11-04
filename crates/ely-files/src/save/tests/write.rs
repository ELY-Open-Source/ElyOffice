//! A save begun on its session, written elsewhere, landed after.

use super::*;

#[test]
fn the_write_runs_on_another_thread_and_lands() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("away.md");
    write(&target, "0");
    let mut session = Session::load(target.clone()).expect("the file loads");
    session.edit();
    let begun = Write::save(session.stamp(), &mut session).expect("the save starts");
    assert_eq!(session.state(), SaveState::Saving);
    let done = std::thread::spawn(move || begun.run(b"edited")).join().expect("the writer ends");
    assert_eq!(read(&target), "edited", "on disk before the session hears");
    assert_eq!(session.state(), SaveState::Saving);
    let saved = done.land(&mut session).expect("the save lands");
    assert_eq!(session.state(), SaveState::Saved);
    assert_eq!(session.fingerprint(), Some(saved.written));
}

#[test]
fn a_change_after_the_ticket_conflicts_at_the_write() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("late.md");
    write(&target, "0");
    let mut session = Session::load(target.clone()).expect("the file loads");
    session.edit();
    let begun = Write::save(session.stamp(), &mut session).expect("the save starts");
    write(&target, "outside, longer");
    let done = begun.run(b"local");
    assert!(matches!(done.saved, Err(SaveError::Conflict)));
    assert!(matches!(done.land(&mut session), Err(SaveError::Conflict)));
    assert_eq!(read(&target), "outside, longer", "the outside change stays");
    assert_eq!(session.state(), SaveState::ExternalModified);
    assert!(matches!(Write::save(session.stamp(), &mut session), Err(SaveError::Conflict)), "a plain save asks first");
    // Keep Mine: the disk is seen, then saved over.
    session.rebase(Fingerprint::of(&target).expect("the fingerprint"));
    Write::save(session.stamp(), &mut session).expect("keep mine starts").run(b"local").land(&mut session).expect("lands");
    assert_eq!(read(&target), "local");
}

#[test]
fn a_reload_during_the_write_keeps_the_reloaded_file() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("reload.md");
    write(&target, "0");
    let mut session = Session::load(target.clone()).expect("the file loads");
    session.edit();
    let begun = Write::save(session.stamp(), &mut session).expect("the save starts");
    let done = begun.run(b"written");
    let seen = Fingerprint::of(&target).expect("the fingerprint");
    session.reload_at(seen).expect("reloads");
    assert!(done.land(&mut session).is_err(), "a stale ticket never lands");
    assert_eq!(session.fingerprint(), Some(seen));
    assert_eq!(session.state(), SaveState::Saved);
}

#[test]
fn an_edit_during_the_export_stays_unsaved() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("typed.md");
    write(&target, "0");
    let mut session = Session::load(target.clone()).expect("the file loads");
    session.edit();
    let exported = session.stamp();
    // Typed after the export took its bytes.
    session.edit();
    let begun = Write::save(exported, &mut session).expect("the save starts");
    begun.run(b"one edit").land(&mut session).expect("the save lands");
    assert_eq!(session.saved_revision(), 1, "the ticket claims what was exported");
    assert_eq!(session.state(), SaveState::Dirty, "the later edit still needs a save");
}

#[test]
fn a_reload_during_the_export_writes_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("reloaded.md");
    write(&target, "0");
    let mut session = Session::load(target.clone()).expect("the file loads");
    session.edit();
    let exported = session.stamp();
    write(&target, "outside, longer");
    session.reload().expect("reloads");
    assert!(Write::save(exported, &mut session).is_err(), "old bytes never land");
    assert_eq!(read(&target), "outside, longer");
    assert_eq!(session.state(), SaveState::Saved);
}

#[test]
fn keep_mine_over_a_gone_file_never_replaces_a_new_one() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("gone.md");
    write(&target, "0");
    let mut session = Session::load(target.clone()).expect("the file loads");
    session.edit();
    std::fs::remove_file(&target).expect("deleted outside");
    let begun = Write::anew(session.stamp(), &mut session).expect("keep mine starts");
    // Made outside between the look and the write.
    write(&target, "theirs");
    let done = begun.run(b"mine");
    assert!(matches!(done.saved, Err(SaveError::Conflict)), "the new file conflicts");
    assert!(matches!(done.land(&mut session), Err(SaveError::Conflict)));
    assert_eq!(read(&target), "theirs", "the new file stays");
    assert_eq!(session.state(), SaveState::ExternalModified);
    std::fs::remove_file(&target).expect("deleted again");
    Write::anew(session.stamp(), &mut session).expect("keep mine starts").run(b"mine").land(&mut session).expect("lands");
    assert_eq!(read(&target), "mine", "still gone: written anew");
}

#[test]
fn a_failed_keep_mine_leaves_no_empty_file() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("gone.md");
    write(&target, "0");
    let mut session = Session::load(target.clone()).expect("the file loads");
    session.edit();
    std::fs::remove_file(&target).expect("deleted outside");
    let begun = Write::anew(session.stamp(), &mut session).expect("keep mine starts");
    let done = begun.run_with(b"mine", |_| Err(std::io::Error::other("the volume failed")));
    assert!(matches!(done.land(&mut session), Err(SaveError::Io(_))));
    assert!(!target.exists(), "the claim is taken back");
}

/// Keep Mine begun on `name`, deleted outside first.
fn gone_keep_mine(dir: &std::path::Path, name: &str) -> (std::path::PathBuf, Session, Write) {
    let target = dir.join(name);
    write(&target, "0");
    let mut session = Session::load(target.clone()).expect("the file loads");
    session.edit();
    std::fs::remove_file(&target).expect("deleted outside");
    let begun = Write::anew(session.stamp(), &mut session).expect("keep mine starts");
    (target, session, begun)
}

#[test]
fn a_file_written_over_the_claim_is_never_replaced() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (target, mut session, begun) = gone_keep_mine(dir.path(), "synced.md");
    let done = begun.run_with(b"mine", |path| {
        write(path, "theirs");
        Ok(true)
    });
    assert!(matches!(done.land(&mut session), Err(SaveError::Conflict)));
    assert_eq!(read(&target), "theirs", "the outside file stays");
}

#[test]
fn a_failed_keep_mine_never_removes_a_file_written_over_the_claim() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (target, mut session, begun) = gone_keep_mine(dir.path(), "synced.md");
    let done = begun.run_with(b"mine", |path| {
        write(path, "theirs");
        Err(std::io::Error::other("the volume failed"))
    });
    assert!(matches!(done.land(&mut session), Err(SaveError::Io(_))));
    assert_eq!(read(&target), "theirs", "the outside file stays");
}
