//! A session whose file went away outside Ely.

use super::*;

#[test]
fn a_missing_file_is_named_in_the_error() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("gone 1.docx");
    write(&target, "0");
    let mut session = Session::load(target.clone()).expect("the file loads");
    session.edit();
    std::fs::remove_file(&target).expect("deleted outside");
    let Err(SaveError::Io(error)) = save(&mut session, b"edited") else {
        panic!("a save in place cannot land");
    };
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(error.to_string().contains("gone 1.docx"), "{error}");
}

#[test]
fn saving_as_onto_its_deleted_file_writes_it_again() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("moved.docx");
    write(&target, "0");
    let mut session = Session::load(target.clone()).expect("the file loads");
    session.edit();
    std::fs::rename(&target, dir.path().join("elsewhere.docx")).expect("moved outside");
    let saved = save_as(&mut session, &target, b"edited").expect("written anew");
    assert_eq!(read(&saved.path), "edited");
    assert_eq!(session.state(), crate::SaveState::Saved);
}
