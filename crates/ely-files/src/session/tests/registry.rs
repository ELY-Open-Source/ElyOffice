use super::*;

#[cfg(unix)]
#[test]
fn a_replace_rekeys_links_and_forks_old_ones() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("doc.md");
    let old = dir.path().join("old.md");
    write(&target, "0");
    std::fs::hard_link(&target, &old).expect("the old link");
    let mut sessions = Sessions::default();
    assert!(matches!(
        sessions.open(&target, false).expect("the first open"),
        Open::Fresh(_)
    ));
    let staging = dir.path().join("doc.md.tmp");
    write(&staging, "1");
    std::fs::rename(&staging, &target).expect("the atomic replace");
    let alias = dir.path().join("alias.md");
    std::fs::hard_link(&target, &alias).expect("the new link");
    assert!(sessions.is_open(&target).expect("the check"));
    assert_eq!(
        sessions.open(&alias, false).expect("the alias open"),
        Open::Focus(0)
    );
    assert!(matches!(
        sessions
            .open(&old, false)
            .expect("the old link opens alone"),
        Open::Fresh(_)
    ));
}

#[test]
fn opening_twice_focuses_or_marks_a_view() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("doc.txt");
    write(&target, "0");
    let mut sessions = Sessions::default();
    assert!(matches!(
        sessions.open(&target, false).expect("the first open"),
        Open::Fresh(_)
    ));
    assert_eq!(
        sessions.open(&target, false).expect("the second open"),
        Open::Focus(0)
    );
    assert_eq!(
        sessions.open(&target, true).expect("the view open"),
        Open::View(1)
    );
    assert!(sessions.is_open(&target).expect("the check"));
    sessions.close(0);
    sessions.close(1);
    assert!(!sessions.is_open(&target).expect("the check"));
}

#[test]
fn hard_links_share_one_session() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("doc.txt");
    let alias = dir.path().join("alias.txt");
    write(&target, "0");
    std::fs::hard_link(&target, &alias).expect("the hard link");
    let mut sessions = Sessions::default();
    assert!(matches!(
        sessions.open(&target, false).expect("the first open"),
        Open::Fresh(_)
    ));
    assert_eq!(
        sessions.open(&alias, false).expect("the alias open"),
        Open::Focus(0)
    );
}

#[cfg(unix)]
#[test]
fn a_dangling_symlink_fails_loud_and_registers_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("gone.txt");
    let link = dir.path().join("link.txt");
    write(&target, "0");
    std::os::unix::fs::symlink(&target, &link).expect("the symlink");
    std::fs::remove_file(&target).expect("the target goes");
    let mut sessions = Sessions::default();
    assert!(sessions.open(&link, false).is_err());
    assert!(sessions.is_open(&link).is_err());
    let real = dir.path().join("real.txt");
    write(&real, "0");
    assert_eq!(
        sessions.open(&real, false).expect("the open"),
        Open::Fresh(0)
    );
}

#[test]
fn an_atomic_replace_keeps_the_session() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("doc.md");
    write(&target, "0");
    let mut sessions = Sessions::default();
    assert!(matches!(
        sessions.open(&target, false).expect("the first open"),
        Open::Fresh(_)
    ));
    let staging = dir.path().join("doc.md.tmp");
    write(&staging, "1");
    std::fs::rename(&staging, &target).expect("the atomic replace");
    assert!(sessions.is_open(&target).expect("the check"));
    assert_eq!(
        sessions.open(&target, false).expect("the second open"),
        Open::Focus(0)
    );
}

#[test]
fn a_missing_file_fails_open_loud() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let missing = dir.path().join("nope.txt");
    let mut sessions = Sessions::default();
    assert!(sessions.open(&missing, false).is_err());
    assert!(sessions.is_open(&missing).is_err());
    let real = dir.path().join("real.txt");
    write(&real, "0");
    assert_eq!(
        sessions.open(&real, false).expect("the open"),
        Open::Fresh(0)
    );
    std::fs::remove_file(&real).expect("the file goes");
    assert!(sessions.is_open(&real).is_err());
    assert!(sessions.open(&real, false).is_err());
}

#[test]
fn a_missing_file_fails_loud() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let missing = dir.path().join("nope.txt");
    assert!(Session::load(missing.clone()).is_err());
    assert!(Fingerprint::of(&missing).is_err());
}

#[cfg(unix)]
#[test]
fn a_revoked_entry_path_fails_loud() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let restricted = dir.path().join("restricted");
    std::fs::create_dir(&restricted).expect("the dir");
    let target = restricted.join("doc.md");
    let alias = dir.path().join("alias.md");
    write(&target, "0");
    std::fs::hard_link(&target, &alias).expect("the hard link");
    let mut sessions = Sessions::default();
    assert!(matches!(
        sessions.open(&target, false).expect("the first open"),
        Open::Fresh(_)
    ));
    // A file in the folder's place stops root too.
    let aside = dir.path().join("aside");
    std::fs::rename(&restricted, &aside).expect("the folder moves aside");
    write(&restricted, "in the way");
    let opened = sessions.open(&alias, false);
    let checked = sessions.is_open(&alias);
    std::fs::remove_file(&restricted).expect("the file goes");
    std::fs::rename(&aside, &restricted).expect("the folder returns");
    assert!(opened.is_err_and(|error| error.kind() == std::io::ErrorKind::NotADirectory));
    assert!(checked.is_err());
    assert_eq!(
        sessions.open(&alias, false).expect("the alias open"),
        Open::Focus(0)
    );
}

#[cfg(unix)]
#[test]
fn a_failed_identity_query_registers_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("a temp dir");
    let first = dir.path().join("first.md");
    let restricted = dir.path().join("restricted");
    std::fs::create_dir(&restricted).expect("the dir");
    let second = restricted.join("second.md");
    write(&first, "0");
    write(&second, "0");
    let mut sessions = Sessions::default();
    assert!(matches!(
        sessions.open(&first, false).expect("the first open"),
        Open::Fresh(_)
    ));
    assert!(matches!(
        sessions.open(&second, false).expect("the second open"),
        Open::Fresh(_)
    ));
    std::fs::set_permissions(&restricted, std::fs::Permissions::from_mode(0o000))
        .expect("the lockdown");
    let viewed = sessions.open(&first, true);
    std::fs::set_permissions(&restricted, std::fs::Permissions::from_mode(0o755))
        .expect("the restore");
    assert_eq!(
        viewed.expect("a definitive match outranks the stuck entry"),
        Open::View(2)
    );
    sessions.close(0);
    sessions.close(2);
    assert!(!sessions.is_open(&first).expect("no phantom entry"));
    assert!(matches!(
        sessions.open(&first, false).expect("the re-open"),
        Open::Fresh(3)
    ));
}

#[cfg(unix)]
#[test]
fn a_focus_returns_before_unreachable_entries() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("a temp dir");
    let first = dir.path().join("first.md");
    let restricted = dir.path().join("restricted");
    std::fs::create_dir(&restricted).expect("the dir");
    let second = restricted.join("second.md");
    write(&first, "0");
    write(&second, "0");
    let mut sessions = Sessions::default();
    assert!(matches!(
        sessions.open(&first, false).expect("the first open"),
        Open::Fresh(_)
    ));
    assert!(matches!(
        sessions.open(&second, false).expect("the second open"),
        Open::Fresh(_)
    ));
    std::fs::set_permissions(&restricted, std::fs::Permissions::from_mode(0o000))
        .expect("the lockdown");
    let focused = sessions.open(&first, false);
    let checked = sessions.is_open(&first);
    std::fs::set_permissions(&restricted, std::fs::Permissions::from_mode(0o755))
        .expect("the restore");
    assert_eq!(focused.expect("the focus"), Open::Focus(0));
    assert!(checked.expect("the check"));
}

#[cfg(unix)]
#[test]
fn a_focus_ignores_unreachable_entries_after_a_match() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("a temp dir");
    let restricted = dir.path().join("restricted");
    std::fs::create_dir(&restricted).expect("the dir");
    let first = restricted.join("first.md");
    let second = dir.path().join("second.md");
    write(&first, "0");
    write(&second, "0");
    let mut sessions = Sessions::default();
    assert!(matches!(
        sessions.open(&first, false).expect("the first open"),
        Open::Fresh(_)
    ));
    assert!(matches!(
        sessions.open(&second, false).expect("the second open"),
        Open::Fresh(_)
    ));
    std::fs::set_permissions(&restricted, std::fs::Permissions::from_mode(0o000))
        .expect("the lockdown");
    let focused = sessions.open(&second, false);
    let checked = sessions.is_open(&second);
    std::fs::set_permissions(&restricted, std::fs::Permissions::from_mode(0o755))
        .expect("the restore");
    assert_eq!(focused.expect("the focus"), Open::Focus(1));
    assert!(checked.expect("the check"));
}

#[cfg(unix)]
#[test]
fn a_rename_is_found_by_identity_not_by_link() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let first = dir.path().join("a.md");
    write(&first, "0");
    let mut sessions = Sessions::default();
    let Open::Fresh(id) = sessions.open(&first, false).expect("the open") else {
        panic!("a fresh session")
    };
    let moved = dir.path().join("b.md");
    std::fs::rename(&first, &moved).expect("the rename");
    std::os::unix::fs::symlink(&moved, dir.path().join("link.md")).expect("the link");
    let found = sessions.renamed_in(id, dir.path()).expect("the scan");
    assert_eq!(found, Some(moved.clone()));
    std::fs::remove_file(&moved).expect("the delete");
    assert_eq!(sessions.renamed_in(id, dir.path()).expect("the scan"), None);
}
