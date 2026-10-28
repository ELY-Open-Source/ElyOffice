//! Signing by update and checking what was signed (PDF-12, QA-15).

use std::path::{Path, PathBuf};

use ely_pdf_engine::{
    Digest, Doc, Lock, Signing, Worker,
    proto::may,
    sample::{Extras, one_page, pdf_with},
};

fn worker() -> Worker {
    Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker")
}

fn opened(path: &Path) -> (Worker, Doc) {
    let mut worker = worker();
    let doc = worker.open(path).expect("opens").doc;
    (worker, doc)
}

fn plain(dir: &Path) -> PathBuf {
    let path = dir.join("doc.pdf");
    std::fs::write(&path, pdf_with(&[(612.0, 792.0, &["Hello world"])], &[], &Extras::default())).expect("writes");
    path
}

fn signing(object: Option<u32>, name: &str, password: &str) -> Signing {
    let pfx = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/pdf/signer.p12");
    Signing {
        page: 0,
        object,
        rect: [72.0, 600.0, 272.0, 660.0],
        name: name.into(),
        pfx: pfx.to_str().expect("UTF-8").into(),
        pfx_password: password.into(),
        reason: "Approved".into(),
        location: "Here".into(),
    }
}

#[test]
fn a_new_signature_checks_and_the_document_stays() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = plain(dir.path());
    let (mut worker, doc) = opened(&path);
    let before = worker.history(doc).expect("history");
    let out = dir.path().join("signed.pdf");
    worker.sign(doc, &out, &signing(None, "Signature1", "ely")).expect("signs");
    assert!(worker.signatures(doc, &path).expect("reads").is_empty(), "the open document has none");
    assert_eq!(worker.history(doc).expect("history"), before);
    let (mut other, read) = opened(&out);
    let found = other.signatures(read, &out).expect("reads");
    assert_eq!(found.len(), 1);
    let signed = found[0].signed.as_ref().expect("signed");
    assert_eq!((signed.digest, signed.later, signed.broken), (Digest::Intact, false, false));
    assert_eq!((signed.common.as_str(), signed.reason.as_str(), signed.location.as_str()), ("Ely Test Signer", "Approved", "Here"));
    assert!(signed.date.starts_with("D:"), "{}", signed.date);
    assert_eq!(found[0].page, Some(0), "found through the page's annotations");
    assert!(signed.certs.first().is_some_and(|der| der.first() == Some(&0x30)), "DER");
}

#[test]
fn a_wrong_certificate_password_writes_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&plain(dir.path()));
    let out = dir.path().join("signed.pdf");
    assert!(worker.sign(doc, &out, &signing(None, "Signature1", "nope")).is_err());
    assert!(!out.exists());
}

#[test]
fn a_changed_byte_shows_as_changed() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&plain(dir.path()));
    let out = dir.path().join("signed.pdf");
    worker.sign(doc, &out, &signing(None, "Signature1", "ely")).expect("signs");
    let mut bytes = std::fs::read(&out).expect("reads");
    let at = bytes.windows(5).position(|w| w == b"Hello").expect("the text");
    bytes[at] = b'J';
    std::fs::write(&out, bytes).expect("writes");
    let (mut other, read) = opened(&out);
    let signed = other.signatures(read, &out).expect("reads")[0].signed.clone().expect("signed");
    assert_eq!(signed.digest, Digest::Changed);
}

#[test]
fn a_second_signature_leaves_the_first_intact_and_marked_later() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&plain(dir.path()));
    let once = dir.path().join("once.pdf");
    worker.sign(doc, &once, &signing(None, "First", "ely")).expect("signs");
    let (mut other, read) = opened(&once);
    let twice = dir.path().join("twice.pdf");
    let mut second = signing(None, "Second", "ely");
    second.rect = [300.0, 600.0, 500.0, 660.0];
    other.sign(read, &twice, &second).expect("signs again");
    let (mut third, last) = opened(&twice);
    let found = third.signatures(last, &twice).expect("reads");
    let first = found.iter().find(|s| s.name == "First").and_then(|s| s.signed.clone()).expect("first");
    assert_eq!((first.digest, first.later), (Digest::Intact, true));
    let next = found.iter().find(|s| s.name == "Second").and_then(|s| s.signed.clone()).expect("second");
    assert_eq!((next.digest, next.later), (Digest::Intact, false));
}

#[test]
fn an_empty_field_is_signed_in_place() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let field = "<< /Type /Annot /Subtype /Widget /FT /Sig /T (Sign here) /Rect [72 600 272 660] /P 3 0 R /F 4 >>".to_string();
    let path = dir.path().join("form.pdf");
    std::fs::write(&path, one_page(" /AcroForm << /Fields [5 0 R] /SigFlags 3 >>", " /Annots [5 0 R]", "", &[field])).expect("writes");
    let (mut worker, doc) = opened(&path);
    let empty = worker.signatures(doc, &path).expect("reads");
    assert_eq!((empty.len(), empty[0].signed.is_none(), empty[0].name.as_str()), (1, true, "Sign here"));
    let out = dir.path().join("signed.pdf");
    worker.sign(doc, &out, &signing(Some(empty[0].object), "", "ely")).expect("signs");
    let (mut other, read) = opened(&out);
    let found = other.signatures(read, &out).expect("reads");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].signed.as_ref().map(|s| s.digest), Some(Digest::Intact));
}

#[test]
fn a_new_field_needs_leave_to_change_the_page() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&plain(dir.path()));
    let limited = dir.path().join("limited.pdf");
    let lock = Lock { user: String::new(), owner: "owner".into(), allow: may::PRINT | may::FILL };
    worker.protect(doc, &limited, Some(&lock)).expect("protects");
    let (mut other, read) = opened(&limited);
    let refused = other.sign(read, &dir.path().join("signed.pdf"), &signing(None, "S", "ely")).expect_err("refused");
    assert!(ely_pdf_engine::refused(&refused).is_some(), "refused: {refused}");
}

#[test]
fn an_ordinary_save_of_a_signed_file_carries_no_signature() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&plain(dir.path()));
    let signed = dir.path().join("signed.pdf");
    worker.sign(doc, &signed, &signing(None, "Signature1", "ely")).expect("signs");
    let (mut other, read) = opened(&signed);
    assert_eq!(other.signed(read).expect("counts"), 1);
    other.rotate_pages(read, &[0], 90).expect("edits");
    let copy = dir.path().join("copy.pdf");
    other.save(read, &copy).expect("saves");
    let (mut third, again) = opened(&copy);
    assert_eq!(third.signed(again).expect("counts"), 0);
    let fields = third.signatures(again, &copy).expect("reads");
    assert_eq!((fields.len(), fields[0].signed.is_none()), (1, true), "the field stays, empty");
}

#[test]
fn the_signer_is_the_certificate_given() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&plain(dir.path()));
    let out = dir.path().join("signed.pdf");
    worker.sign(doc, &out, &signing(None, "Signature1", "ely")).expect("signs");
    let (mut other, read) = opened(&out);
    let signed = other.signatures(read, &out).expect("reads")[0].signed.clone().expect("signed");
    let der = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/pdf/signer.der")).expect("reads");
    assert_eq!(signed.certs, vec![der]);
}

#[test]
fn a_typed_signature_writes_in_the_script_face_and_says_what_it_lacks() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&plain(dir.path()));
    let writing = |text: &str| ely_pdf_engine::Writing {
        block: None,
        expected: String::new(),
        text: text.into(),
        font: "script".into(),
        size: 28.0,
        rgb: 0,
        bx: [72.0, 600.0, 400.0],
    };
    assert!(matches!(worker.put_text(doc, 0, &writing("Ada Lovelace"), 0.0).expect("writes"), ely_pdf_engine::Wrote::Done(_)));
    doc = worker.now(doc).expect("the state");
    let text: String = worker.text(doc, 0).expect("reads").iter().map(|c| c.c).collect();
    assert!(text.contains("Ada Lovelace"), "{text}");
    let saved = dir.path().join("typed.pdf");
    worker.save(doc, &saved).expect("saves");
    let bytes = std::fs::read(&saved).expect("reads");
    assert!(bytes.windows(10).any(|w| w == b"GreatVibes"), "the script face is embedded");
    assert!(matches!(worker.put_text(doc, 0, &writing("王"), 0.0).expect("asks"), ely_pdf_engine::Wrote::Missing('王')));
}

#[test]
fn signatures_are_read_from_the_file_named() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&plain(dir.path()));
    let signed = dir.path().join("signed.pdf");
    worker.sign(doc, &signed, &signing(None, "Signature1", "ely")).expect("signs");
    let (mut other, read) = opened(&signed);
    let (mark, copy) = (other.history(read).expect("history"), dir.path().join("copy.pdf"));
    other.save(read, &copy).expect("saves");
    assert_eq!(other.history(read).expect("history"), mark, "saving edits nothing");
    assert!(other.signatures(read, &copy).expect("reads")[0].signed.is_none(), "the copy holds none");
    assert!(other.signatures(read, &signed).expect("reads")[0].signed.is_some(), "the signed file still does");
}

#[test]
fn a_signature_on_no_page_is_still_listed() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&plain(dir.path()));
    let out = dir.path().join("signed.pdf");
    worker.sign(doc, &out, &signing(None, "Signature1", "ely")).expect("signs");
    let mut bytes = std::fs::read(&out).expect("reads");
    // No page lists the widget; the form still does.
    let at = bytes.windows(7).rposition(|w| w == b"/Annots").expect("the page's annotations");
    bytes[at + 1] = b'X';
    std::fs::write(&out, bytes).expect("writes");
    let (mut other, read) = opened(&out);
    let found = other.signatures(read, &out).expect("reads");
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].page, found[0].signed.is_some()), (None, true));
}

#[test]
fn a_failed_save_leaves_the_history_whole() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&plain(dir.path()));
    assert!(worker.save(doc, &dir.path().join("missing/out.pdf")).is_err());
    let mark = worker.rotate_pages(doc, &[0], 90).expect("rotates");
    doc = worker.now(doc).expect("the state");
    assert_ne!(mark, "0", "the edit is a step of its own");
    worker.undo(doc, false).expect("undoes");
}

#[test]
fn a_field_emptied_by_a_save_can_be_signed_again() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&plain(dir.path()));
    let signed = dir.path().join("signed.pdf");
    worker.sign(doc, &signed, &signing(None, "Signature1", "ely")).expect("signs");
    let (mut other, read) = opened(&signed);
    let copy = dir.path().join("copy.pdf");
    other.save(read, &copy).expect("saves");
    let (mut third, again) = opened(&copy);
    let field = third.signatures(again, &copy).expect("reads")[0].clone();
    let out = dir.path().join("resigned.pdf");
    third.sign(again, &out, &signing(Some(field.object), "", "ely")).expect("signs the emptied field");
    let found = third.signatures(again, &out).expect("reads");
    assert_eq!(found[0].signed.as_ref().map(|s| s.digest), Some(Digest::Intact));
}
