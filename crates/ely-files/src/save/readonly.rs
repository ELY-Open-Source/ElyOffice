//! A read-only folder: worded, the file kept (R20.64).

use std::os::unix::fs::PermissionsExt;

use super::{save_atomic, unwritable};

#[test]
fn a_read_only_folder_is_named_and_the_file_kept() {
    // Root writes anywhere; the refusal never comes.
    if unsafe { libc::geteuid() } == 0 {
        return eprintln!("SKIP: running as root");
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let folder = dir.path().join("docs");
    std::fs::create_dir(&folder).expect("the folder");
    let path = folder.join("a.docx");
    std::fs::write(&path, b"before").expect("the file");
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o555)).expect("read-only");
    let refused = save_atomic(&path, b"after", 1).expect_err("a read-only folder refuses");
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755)).expect("writable again");
    let worded = refused.get_ref().and_then(|inner| inner.downcast_ref::<ely_ipc::Worded>()).expect("worded");
    assert_eq!((worded.key, worded.args.as_slice()), ("office.save.folder", &[("folder".to_string(), "docs".to_string())][..]));
    assert_eq!(refused.kind(), std::io::ErrorKind::PermissionDenied);
    assert_eq!(refused.to_string(), "office.save.folder", "no path in its text");
    assert_eq!(std::fs::read(&path).expect("the file"), b"before");
    assert_eq!(std::fs::read_dir(&folder).expect("the folder").count(), 1, "nothing left beside it");
}

#[test]
fn a_drive_root_is_worded_without_its_path() {
    let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
    let refused = unwritable(std::path::Path::new("/"), std::path::Path::new("/a.docx"), denied);
    let worded = refused.get_ref().and_then(|inner| inner.downcast_ref::<ely_ipc::Worded>()).expect("worded");
    assert_eq!((worded.key, worded.args.len()), ("office.save.root", 0), "no path as its name");
}
