//! A save begun on its session, written elsewhere, landed after.

use super::*;

#[test]
fn the_write_runs_on_another_thread_and_lands() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("away.md");
    write(&target, "0");
    let mut session = Session::load(target.clone()).expect("the file loads");
    session.edit();
    let begun = Write::save(&mut session).expect("the save starts");
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
    let begun = Write::save(&mut session).expect("the save starts");
    write(&target, "outside, longer");
    let done = begun.run(b"local");
    assert!(matches!(done.saved, Err(SaveError::Conflict)));
    assert!(matches!(done.land(&mut session), Err(SaveError::Conflict)));
    assert_eq!(read(&target), "outside, longer", "the outside change stays");
    assert_eq!(session.state(), SaveState::ExternalModified);
    assert!(matches!(Write::save(&mut session), Err(SaveError::Conflict)), "a plain save asks first");
    Write::overwrite(&mut session).expect("keep mine starts").run(b"local").land(&mut session).expect("lands");
    assert_eq!(read(&target), "local");
}

#[test]
fn a_reload_during_the_write_keeps_the_reloaded_file() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("reload.md");
    write(&target, "0");
    let mut session = Session::load(target.clone()).expect("the file loads");
    session.edit();
    let begun = Write::save(&mut session).expect("the save starts");
    let done = begun.run(b"written");
    let seen = Fingerprint::of(&target).expect("the fingerprint");
    session.reload_at(seen).expect("reloads");
    assert!(done.land(&mut session).is_err(), "a stale ticket never lands");
    assert_eq!(session.fingerprint(), Some(seen));
    assert_eq!(session.state(), SaveState::Saved);
}
