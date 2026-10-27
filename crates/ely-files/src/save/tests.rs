use super::*;
use crate::Session;

fn write(path: &Path, body: &str) {
    std::fs::write(path, body).expect("the fixture file writes");
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).expect("the file reads")
}

#[test]
fn an_atomic_save_lands_and_fingerprints() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("doc.md");
    let written = save_atomic(&target, b"body", 3).expect("the save lands");
    assert_eq!(read(&target), "body");
    assert_eq!(written, Fingerprint::of(&target).expect("the fingerprint"));
    assert_eq!(written.len, 4);
}

#[test]
fn a_bare_relative_name_saves_and_probes() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let here = std::env::current_dir().expect("the cwd");
    std::env::set_current_dir(dir.path()).expect("into the temp dir");
    let result = save_atomic(Path::new("report.md"), b"0", 0)
        .map(|_| atomic_capable(Path::new("report.md")));
    std::env::set_current_dir(here).expect("back to the cwd");
    assert!(result.expect("the save lands").expect("the probe"));
    assert_eq!(read(&dir.path().join("report.md")), "0");
}

#[cfg(unix)]
#[test]
fn a_failed_write_keeps_the_original() {
    use std::os::unix::fs::PermissionsExt;
    // Root ignores modes; only read-only mounts stop it.
    if unsafe { libc::geteuid() } == 0 {
        return eprintln!("SKIP: root ignores the folder's mode");
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("keep.md");
    write(&target, "original");
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555))
        .expect("the lockdown");
    let failed = save_atomic(&target, b"new", 1);
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755))
        .expect("the restore");
    assert!(failed.is_err());
    assert_eq!(read(&target), "original");
}

#[cfg(unix)]
#[test]
fn a_save_keeps_the_original_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("shared.md");
    write(&target, "0");
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o640))
        .expect("the group read");
    save_atomic(&target, b"1", 1).expect("the save lands");
    let mode = std::fs::metadata(&target)
        .expect("the metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o640);
}

#[test]
fn paths_keep_spaces_cjk_and_case() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let name = "报告 Final 版本 — Copy.TXT";
    let target = dir.path().join(name);
    save_atomic(&target, b"0", 0).expect("the save lands");
    let listed: Vec<String> = std::fs::read_dir(dir.path())
        .expect("the dir lists")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(listed, vec![name.to_string()]);
}

#[test]
fn a_backup_save_keeps_the_previous_file() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("net.md");
    write(&target, "previous");
    let (_, backup) = save_with_backup(&target, b"new", 1).expect("the save lands");
    let backup = backup.expect("a backup exists");
    assert_eq!(read(&target), "new");
    assert_eq!(read(&backup), "previous");
    assert_eq!(backup, dir.path().join("net.md.bak"));
}

#[cfg(unix)]
#[test]
fn a_backup_never_truncates_or_follows_links() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("report.md");
    write(&target, "previous");
    let taken = dir.path().join("report.md.bak");
    std::fs::hard_link(&target, &taken).expect("the hard link");
    let other = dir.path().join("other.md");
    write(&other, "other");
    let linked = dir.path().join("report.md-1.bak");
    std::os::unix::fs::symlink(&other, &linked).expect("the symlink");
    let (_, backup) = save_with_backup(&target, b"new", 1).expect("the save lands");
    let backup = backup.expect("a backup exists");
    assert!(
        !matches!(backup.file_name(), Some(n) if n == "report.md.bak" || n == "report.md-1.bak")
    );
    assert_eq!(read(&target), "new");
    assert_eq!(read(&other), "other");
    assert_eq!(read(&backup), "previous");
}

#[test]
fn a_backup_name_fits_long_file_names() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let name = format!("{}.md", "x".repeat(250));
    let target = dir.path().join(name);
    write(&target, "previous");
    let (_, backup) = save_with_backup(&target, b"new", 1).expect("the save lands");
    let backup = backup.expect("a backup exists");
    assert_eq!(read(&backup), "previous");
    let len = backup.file_name().expect("a name").len();
    assert!(len <= 255, "the backup name fits: {len}");
}

#[test]
fn an_export_is_independent_and_never_overwrites() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let editing = dir.path().join("report.md");
    write(&editing, "session");
    let out = dir.path().join("out.pdf");
    save_atomic(&out, b"%PDF", 2).expect("the export lands");
    assert_eq!(read(&editing), "session");
    let refused = export(&editing, Some(editing.as_path()), b"%PDF", 2);
    assert!(refused.is_err());
    assert_eq!(read(&editing), "session");
    assert!(export(&out, Some(editing.as_path()), b"%PDF2", 2).is_ok());
}

#[cfg(unix)]
#[test]
fn an_export_refuses_path_aliases_of_the_session() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let editing = dir.path().join("report.md");
    write(&editing, "session");
    let child = dir.path().join("child");
    std::fs::create_dir(&child).expect("the child dir");
    let dotted = child.join("..").join("report.md");
    assert!(export(&dotted, Some(editing.as_path()), b"%PDF", 2).is_err());
    let linked = dir.path().join("linked");
    std::os::unix::fs::symlink(&editing, &linked).expect("the symlink");
    assert!(export(&linked, Some(editing.as_path()), b"%PDF", 2).is_err());
    let alias = dir.path().join("alias.md");
    std::fs::hard_link(&editing, &alias).expect("the hard link");
    assert!(export(&alias, Some(editing.as_path()), b"%PDF", 2).is_err());
    assert_eq!(read(&editing), "session");
}


#[test]
fn the_save_flow_checks_external_changes_first() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("flow.md");
    write(&target, "0");
    let mut session = Session::load(target.clone()).expect("the file loads");
    session.edit();
    write(&target, "external");
    let outcome = save(&mut session, b"local");
    assert!(matches!(outcome, Err(SaveError::Conflict)));
    assert_eq!(read(&target), "external");
    assert_eq!(session.revision(), 1);
    assert_eq!(session.state(), crate::SaveState::ExternalModified);
    let ticket = session.begin_save_overwrite().expect("keep-local starts");
    write(&target, "local");
    let written = Fingerprint::of(&target).expect("the fingerprint");
    session
        .finish_save(ticket, target, written)
        .expect("the overwrite lands");
    assert_eq!(session.state(), crate::SaveState::Saved);
}

#[test]
fn the_save_flow_writes_and_confirms() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("ok.md");
    write(&target, "0");
    let mut session = Session::load(target).expect("the file loads");
    session.edit();
    let saved = save(&mut session, b"edited").expect("the save lands");
    assert_eq!(read(&saved.path), "edited");
    assert_eq!(saved.backup, None);
    assert_eq!(session.state(), crate::SaveState::Saved);
}

#[test]
fn external_change_detection_follows_the_fingerprint() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("x.md");
    write(&target, "0");
    let mut session = Session::load(target.clone()).expect("the file loads");
    assert!(!session.external_changed().expect("the check"));
    write(&target, "longer content");
    assert!(session.external_changed().expect("the check"));
    session.reload().expect("the file reloads");
    assert!(!session.external_changed().expect("the check"));
    let fresh = Session::new_file();
    assert!(!fresh.external_changed().expect("the check"));
}

#[test]
fn local_volumes_are_atomic_capable() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("local.md");
    assert!(atomic_capable(&target).expect("the probe"));
}

#[test]
fn the_save_flow_keeps_a_backup_on_a_limited_volume() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("net.md");
    write(&target, "previous");
    let mut session = Session::load(target.clone()).expect("the file loads");
    session.edit();
    let saved = save_mode(&mut session, &target, b"new", true).expect("the save lands");
    let backup = saved.backup.expect("a backup on a limited volume");
    assert_eq!(read(&target), "new");
    assert_eq!(read(&backup), "previous");
    assert_eq!(session.state(), crate::SaveState::Saved);
}

#[cfg(target_os = "macos")]
#[test]
fn a_save_keeps_the_original_acl() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("acl.md");
    write(&target, "0");
    let chmod = std::process::Command::new("chmod")
        .args(["+a", "nobody deny read", target.to_str().expect("utf8")])
        .status()
        .expect("chmod runs");
    assert!(chmod.success());
    save_atomic(&target, b"1", 1).expect("the save lands");
    let listed = std::process::Command::new("/bin/ls")
        .args(["-le", target.to_str().expect("utf8")])
        .output()
        .expect("ls runs");
    let text = String::from_utf8_lossy(&listed.stdout).into_owned();
    assert!(text.contains("deny read"), "the ACL survived: {text}");
}

#[cfg(unix)]
#[test]
fn a_backup_keeps_the_source_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("private.md");
    write(&target, "secret");
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600))
        .expect("the private mode");
    let (_, backup) = save_with_backup(&target, b"new", 1).expect("the save lands");
    let mode = std::fs::metadata(backup.expect("a backup"))
        .expect("the metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
}

#[cfg(unix)]
#[test]
fn a_save_through_a_symlink_keeps_the_link() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let real = dir.path().join("report.md");
    let alias = dir.path().join("alias.md");
    write(&real, "0");
    std::os::unix::fs::symlink(&real, &alias).expect("the symlink");
    let mut session = Session::load(alias.clone()).expect("the file loads");
    session.edit();
    save(&mut session, b"edited").expect("the save lands");
    assert_eq!(read(&real), "edited");
    let kind = std::fs::symlink_metadata(&alias)
        .expect("the metadata")
        .file_type();
    assert!(kind.is_symlink(), "the link survives the save");
    assert_eq!(session.state(), crate::SaveState::Saved);
}

#[test]
fn backups_never_run_out_of_names() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("report.md");
    write(&target, "0");
    for n in 0..105u32 {
        save_with_backup(&target, format!("v{n}").as_bytes(), n as u64).expect("the save lands");
    }
    let names = std::fs::read_dir(dir.path())
        .expect("the dir lists")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".bak"))
        .count();
    assert_eq!(names, 105);
}

#[test]
fn a_long_extension_import_fits_twice() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let source = dir.path().join(format!("a.{}", "x".repeat(253)));
    write(&source, "png");
    let attachments = dir.path().join("attachments");
    import_attachment(&source, &attachments).expect("the first import");
    let second = import_attachment(&source, &attachments).expect("the second import");
    let len = second.file_name().expect("a name").len();
    assert!(len <= 255, "the numbered name fits: {len}");
    assert_eq!(read(&second), "png");
}

#[cfg(unix)]
#[test]
fn a_save_through_a_cross_dir_link_probes_the_real_volume() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let home = dir.path().join("home");
    let cloud = dir.path().join("cloud");
    std::fs::create_dir(&home).expect("the home dir");
    std::fs::create_dir(&cloud).expect("the cloud dir");
    let real = cloud.join("report.md");
    let alias = home.join("alias.md");
    write(&real, "previous");
    std::os::unix::fs::symlink(&real, &alias).expect("the symlink");
    let mut session = Session::load(alias).expect("the file loads");
    session.edit();
    let limited = resolved(&real).expect("the resolved target");
    let probe = move |path: &Path| Ok(path != limited);
    let saved = save_with_probe(&mut session, b"new", probe).expect("the save lands");
    let backup = saved.backup.expect("a backup on the limited volume");
    let real_dir = resolved(&cloud).expect("the resolved cloud dir");
    assert_eq!(backup.parent(), Some(real_dir.as_path()));
    assert_eq!(read(&backup), "previous");
    assert_eq!(read(&real), "new");
    assert_eq!(session.state(), crate::SaveState::Saved);
}

#[test]
fn a_full_length_name_gets_a_backup() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join("x".repeat(255));
    write(&target, "previous");
    let (_, backup) = save_with_backup(&target, b"new", 1).expect("the save lands");
    let backup = backup.expect("a backup exists");
    let len = backup.file_name().expect("a name").len();
    assert!(len <= 255, "the backup name fits: {len}");
    assert_eq!(read(&backup), "previous");
}

#[test]
fn a_long_name_gets_distinct_backups_every_time() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let target = dir.path().join(format!("{}.md", "x".repeat(250)));
    write(&target, "0");
    let mut names = std::collections::HashSet::new();
    for n in 0..12u32 {
        let (_, backup) =
            save_with_backup(&target, format!("v{n}").as_bytes(), n as u64).expect("the save");
        let name = backup
            .expect("a backup")
            .file_name()
            .expect("a name")
            .to_owned();
        assert!(name.len() <= 255, "the backup name fits");
        assert!(names.insert(name), "every backup name differs");
    }
}

#[test]
fn a_full_length_attachment_imports_twice() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let source = dir.path().join("x".repeat(255));
    write(&source, "png");
    let attachments = dir.path().join("attachments");
    let first = import_attachment(&source, &attachments).expect("the first import");
    let second = import_attachment(&source, &attachments).expect("the second import");
    assert_ne!(first, second);
    let len = second.file_name().expect("a name").len();
    assert!(len <= 255, "the numbered name fits: {len}");
    assert_eq!(read(&second), "png");
}

#[test]
fn save_as_gives_a_new_file_its_path_and_leaves_the_source() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut session = Session::new_file();
    session.edit();
    let first = dir.path().join("Untitled.docx");
    let saved = save_as(&mut session, &first, b"one").expect("the first save");
    assert_eq!(read(&first), "one");
    assert_eq!(
        session.path(),
        Some(saved.path.as_path()),
        "the session takes the path"
    );
    assert_eq!(session.state(), SaveState::Saved);
    session.edit();
    let second = dir.path().join("copy.odt");
    save_as(&mut session, &second, b"two").expect("the second save");
    assert_eq!(read(&first), "one", "the source stays");
    assert_eq!(read(&second), "two");
    assert!(
        session
            .path()
            .is_some_and(|path| path.ends_with("copy.odt"))
    );
    session.edit();
    save(&mut session, b"three").expect("a plain save follows the new path");
    assert_eq!(read(&second), "three");
}

#[test]
fn save_as_onto_its_own_file_checks_for_outside_changes() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let own = dir.path().join("own.docx");
    let mut session = Session::new_file();
    session.edit();
    save_as(&mut session, &own, b"mine").expect("the first save");
    session.edit();
    std::thread::sleep(std::time::Duration::from_millis(20));
    write(&own, "EXTERNAL REVISION");
    let alias = dir.path().join("alias.docx");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&own, &alias).expect("the alias");
    #[cfg(windows)]
    std::fs::hard_link(&own, &alias).expect("the alias");
    assert!(matches!(
        save_as(&mut session, &alias, b"local"),
        Err(SaveError::Conflict)
    ));
    assert_eq!(read(&own), "EXTERNAL REVISION", "the outside change stays");
    let copy = dir.path().join("local-copy.docx");
    save_as(&mut session, &copy, b"local").expect("a new file keeps the local version");
    assert_eq!(read(&copy), "local");
    assert_eq!(read(&own), "EXTERNAL REVISION");
    assert_eq!(
        session.state(),
        SaveState::Saved,
        "the session moved to the copy"
    );
}

#[test]
fn overwrite_resolves_a_conflict_with_the_local_version() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let own = dir.path().join("own.odt");
    let mut session = Session::new_file();
    session.edit();
    save_as(&mut session, &own, b"mine").expect("the first save");
    session.edit();
    std::thread::sleep(std::time::Duration::from_millis(20));
    write(&own, "theirs");
    assert!(matches!(
        save(&mut session, b"local"),
        Err(SaveError::Conflict)
    ));
    save_overwrite(&mut session, b"local").expect("keep-local overwrites");
    assert_eq!(read(&own), "local");
    assert_eq!(session.state(), SaveState::Saved);
}

#[cfg(windows)]
mod long;


mod names;
