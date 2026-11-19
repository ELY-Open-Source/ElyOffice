//! A worker that dies after connecting says why (R20.40).

use super::analysis::skip;
use super::*;

#[test]
fn a_worker_whose_kernel_will_not_load_says_why() {
    if skip() {
        return;
    }
    let real = install().expect("the pinned build");
    let fake = tempfile::tempdir().expect("a scratch kernel");
    let program = fake.path().join("program");
    // The real id and version pass the handshake.
    for name in [crate::endpoint::KERNEL_ID, crate::handshake::VERSION_FILE] {
        let to = program.join(name);
        std::fs::create_dir_all(to.parent().expect("a folder")).expect("its folder");
        std::fs::copy(real.join(name), &to).expect("copies the kernel's id");
    }
    // Truncated, as a kernel relinked in place.
    std::fs::write(program.join(crate::ffi::KERNEL), b"\x7fELF").expect("a truncated kernel");
    let profile = tempfile::tempdir().expect("a profile");
    // The loader's own words, as the worker gets them.
    let canonical = dunce::canonicalize(&program).expect("the kernel's path");
    let loader = crate::ffi::Library::open(&canonical).err().expect("the loader refuses it").to_string();
    let error = Worker::spawn_dev(&program, profile.path()).err().expect("no kernel loads").to_string();
    let said = error.split_once("the worker said: ").map(|(_, said)| said).unwrap_or_default();
    assert!(said.contains(&format!("ERROR ely_lok_worker] lok-worker: {loader}")), "{loader} in: {error}");
    assert!(error.contains("exit status: 1") || error.contains("exit code: 1"), "its exit: {error}");
}
