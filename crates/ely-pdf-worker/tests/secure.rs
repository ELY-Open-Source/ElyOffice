//! Protected copies and what they allow (PDF-11).

use std::path::Path;

use ely_pdf_engine::{
    Doc, Lock, Worker,
    proto::may,
    sample::{Extras, pdf_with},
};

fn worker() -> Worker {
    Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker")
}

fn plain(dir: &Path) -> (Worker, Doc) {
    let path = dir.join("doc.pdf");
    std::fs::write(&path, pdf_with(&[(612.0, 792.0, &["Hello world"])], &[], &Extras::default())).expect("writes");
    let mut worker = worker();
    let doc = worker.open(&path).expect("opens").doc;
    (worker, doc)
}

/// Writes a copy that opens with "user", allowing only `allow`.
fn locked(dir: &Path, allow: u32) -> std::path::PathBuf {
    let (mut worker, doc) = plain(dir);
    let out = dir.join("locked.pdf");
    let lock = Lock { user: "user".into(), owner: "owner".into(), allow };
    worker.protect(doc, &out, Some(&lock)).expect("protects");
    out
}

#[test]
fn a_protected_copy_asks_and_holds_its_limits() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let out = locked(dir.path(), may::PRINT);
    let bytes = std::fs::read(&out).expect("reads");
    assert!(bytes.windows(5).any(|w| w == b"AESV3"), "AES-256");
    let mut worker = worker();
    let opened = worker.open(&out).expect("opens");
    assert!(opened.locked);
    assert!(!worker.unlock(opened.doc, "wrong").expect("tries"));
    assert!(worker.unlock(opened.doc, "user").expect("tries"));
    let allowed = worker.permissions(opened.doc).expect("reads");
    assert_eq!(allowed & (may::PRINT | may::COPY | may::EDIT | may::OWNER), may::PRINT);
    let refused = worker.rotate_pages(opened.doc, &[0], 90).expect_err("refused");
    assert_eq!(ely_pdf_engine::refused(&refused), Some("assemble"));
    let text: String = worker.text(opened.doc, 0).expect("still reads").iter().map(|c| c.c).collect();
    assert!(text.contains("Hello"));
}

#[test]
fn a_wrong_owner_password_keeps_access_and_the_right_one_lifts_limits() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let out = locked(dir.path(), 0);
    let mut worker = worker();
    let doc = worker.open(&out).expect("opens").doc;
    worker.unlock(doc, "user").expect("unlocks");
    assert!(!worker.owner(doc, "nope").expect("tries"));
    assert_eq!(worker.pages(doc).expect("still open"), 1);
    assert!(worker.text(doc, 0).is_ok());
    assert!(worker.owner(doc, "owner").expect("tries"));
    assert_ne!(worker.permissions(doc).expect("reads") & may::OWNER, 0);
    worker.rotate_pages(doc, &[0], 90).expect("allowed now");
}

#[test]
fn changing_protection_needs_the_owner() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let out = locked(dir.path(), may::PRINT | may::COPY | may::EDIT);
    let mut worker = worker();
    let doc = worker.open(&out).expect("opens").doc;
    worker.unlock(doc, "user").expect("unlocks");
    let bare = dir.path().join("bare.pdf");
    let refused = worker.protect(doc, &bare, None).expect_err("refused");
    assert!(ely_pdf_engine::refused(&refused).is_some(), "refused: {refused}");
    assert!(!bare.exists());
    worker.owner(doc, "owner").expect("tries");
    worker.protect(doc, &bare, None).expect("unprotects");
    let mut other = self::worker();
    assert!(!other.open(&bare).expect("opens").locked);
}

#[test]
fn an_owner_password_is_needed_and_must_differ() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = plain(dir.path());
    let out = dir.path().join("out.pdf");
    for (user, owner) in [("a", ""), ("same", "same")] {
        let lock = Lock { user: user.into(), owner: owner.into(), allow: 0 };
        assert!(worker.protect(doc, &out, Some(&lock)).is_err(), "{user}/{owner}");
        assert!(!out.exists());
    }
}

#[test]
fn a_file_with_no_open_password_still_holds_limits() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = plain(dir.path());
    let out = dir.path().join("out.pdf");
    let lock = Lock { user: String::new(), owner: "owner".into(), allow: may::PRINT };
    worker.protect(doc, &out, Some(&lock)).expect("protects");
    let mut other = self::worker();
    let opened = other.open(&out).expect("opens");
    assert!(!opened.locked);
    let refused = other.annotate(opened.doc, &[]).expect_err("refused");
    assert_eq!(ely_pdf_engine::refused(&refused), Some("annotate"));
}

#[test]
fn a_wrong_owner_try_on_aes_128_keeps_the_text_readable() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/pdf/aes128.pdf");
    let mut worker = worker();
    let doc = worker.open(&fixture).expect("opens").doc;
    assert!(worker.unlock(doc, "user").expect("tries"));
    assert!(!worker.owner(doc, "nope").expect("tries"));
    let text: String = worker.text(doc, 0).expect("reads").iter().map(|c| c.c).collect();
    assert_eq!(text.trim(), "Hello");
}
