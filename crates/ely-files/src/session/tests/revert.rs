//! Undo back to the saved content (FILE-02).

use super::*;

#[test]
fn undo_to_the_saved_content_is_clean_but_a_conflict_stays() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("r.pdf");
    write(&target, "0");
    let mut session = Session::load(target.clone()).expect("the file loads");
    session.edit();
    session.reverted();
    assert_eq!(session.state(), SaveState::Saved);
    session.edit();
    let _ticket = session.begin_save().expect("the save starts");
    session.reverted();
    assert_eq!(
        session.state(),
        SaveState::Saving,
        "a save in flight decides"
    );
    let mut changed = Session::load(target.clone()).expect("the file loads");
    changed.external_modified();
    changed.reverted();
    assert_eq!(changed.state(), SaveState::ExternalModified);
}

#[test]
fn a_rebase_saves_against_the_seen_disk_only() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("m.md");
    write(&target, "base");
    let mut session = Session::load(target.clone()).expect("the file loads");
    session.edit();
    // The outside write needs a later mtime.
    std::thread::sleep(std::time::Duration::from_millis(20));
    write(&target, "seen change");
    session.external_modified();
    let seen = Fingerprint::of(&target).expect("the fingerprint");
    session.rebase(seen);
    assert_eq!(session.state(), SaveState::Dirty);
    assert!(!session.external_changed().expect("the check"));
    // The later write needs a later mtime.
    std::thread::sleep(std::time::Duration::from_millis(20));
    write(&target, "a later change");
    assert!(session.external_changed().expect("the check"), "a later write still conflicts");
    assert!(matches!(
        crate::save(&mut session, b"local"),
        Err(crate::SaveError::Conflict)
    ));
    assert_eq!(std::fs::read(&target).expect("the file"), b"a later change");
}
