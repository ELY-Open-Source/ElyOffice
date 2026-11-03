//! Profiles near Windows path limits start; dying workers say why.

use std::path::{Path, PathBuf};

use super::*;

/// A profile folder under `base` whose path has `length` characters.
fn profile_of(base: &Path, length: usize) -> PathBuf {
    let base = dunce::canonicalize(base).expect("the temp dir resolves");
    let used = base.to_string_lossy().encode_utf16().count() + 1;
    assert!(used < length, "the temp dir {} is too long", base.display());
    base.join("p".repeat(length - used))
}

#[test]
fn profiles_whose_kernel_folders_end_at_248_characters_start() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut failed = Vec::new();
    for length in [None, Some(149), Some(150), Some(160)] {
        let dir = tempfile::tempdir().expect("a temp dir");
        let profile = match length {
            Some(length) => profile_of(dir.path(), length),
            None => dir.path().join("profile"),
        };
        match Worker::spawn(&install, &profile) {
            Ok(mut worker) => worker.shutdown().expect("the worker shuts down"),
            Err(error) => failed.push(format!("{length:?}: {error}")),
        }
    }
    assert!(failed.is_empty(), "profiles that did not start:\n{}", failed.join("\n"));
}

/// Windows' abort text; other systems word it otherwise.
#[cfg(windows)]
#[test]
fn a_worker_that_dies_before_its_check_says_its_last_line() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let profile = dir.path().join("profile");
    std::fs::create_dir_all(profile.join("user")).expect("the profile is made");
    // A file where the kernel makes a folder aborts it.
    std::fs::write(profile.join("user").join("extensions"), "").expect("the file is written");
    let error = match Worker::spawn(&install, &profile) {
        Ok(_) => panic!("the worker passed its check on a broken profile"),
        Err(error) => error.to_string(),
    };
    let last = "fatal runtime error: Rust cannot catch foreign exceptions, aborting";
    assert!(error.starts_with("the lok worker failed"), "{error}");
    assert!(error.ends_with(last), "{error}");
}
