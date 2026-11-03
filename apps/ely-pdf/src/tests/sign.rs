//! Signatures beside the pages, signed copies, signed files (C11.7).

use std::path::{Path, PathBuf};

use ely_office::{Saving as _, Tabs as _};
use ely_pdf_engine::{Digest, Signing, Worker, sample::{Extras, pdf_with}};
use ely_platform::trust::Trust;
use gpui::{Entity, TestAppContext, VisualTestContext};

use crate::view::PdfView;
use crate::viewer::Viewer;

use super::{settle, shown, windowed};
use crate::sign::SignAt;

fn signing(name: &str) -> Signing {
    let pfx = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/pdf/signer.p12");
    Signing {
        page: 0,
        object: None,
        rect: [72.0, 600.0, 272.0, 660.0],
        name: name.into(),
        pfx: pfx.to_str().expect("UTF-8").into(),
        pfx_password: "ely".into(),
        reason: String::new(),
        location: String::new(),
    }
}

fn plain(dir: &Path) -> PathBuf {
    let path = dir.join("plain.pdf");
    std::fs::write(&path, pdf_with(&[(612.0, 792.0, &["Hello world"])], &[], &Extras::default())).expect("writes");
    path
}

fn signed(dir: &Path) -> PathBuf {
    let mut worker = Worker::spawn().expect("the worker");
    let doc = worker.open(&plain(dir)).expect("opens").doc;
    let path = dir.join("signed.pdf");
    worker.sign(doc, &path, &signing("Signature1")).expect("signs");
    path
}

#[test]
fn a_signed_file_lists_its_checks_and_saves_as_a_copy() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = signed(dir.path());
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    assert_eq!(cx.update(|_, app| viewer.read(app).active_tab().and_then(|tab| tab.risk)), Some(ely_office::Risk::Signed));
    let path = tab_path(cx, &viewer);
    cx.update(|_, app| view.update(app, |view, cx| view.ask_signatures(path, cx)));
    settle(cx, |app| view.read(app).signatures.is_some());
    cx.update(|_, app| {
        let read = view.read(app);
        let checked = &read.signatures.as_ref().expect("read").1[0];
        assert_eq!(checked.field.signed.as_ref().map(|s| s.digest), Some(Digest::Intact));
        assert!(matches!(checked.trust, Some(Ok(Trust::Untrusted(_)))), "self-signed");
    });
    // The dialog draws every fact.
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.show_signatures(true, cx)));
    super::frames(cx);
    assert!(cx.update(|_, app| viewer.read(app).signs_open));
}

#[test]
fn signing_writes_a_signed_copy_and_leaves_the_document() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[plain(dir.path())], true);
    let view = shown(cx, &viewer);
    let at = SignAt { page: 0, object: None, rect: [72.0, 600.0, 272.0, 660.0], version: 0 };
    cx.update(|_, app| view.update(app, |view, _| view.sign_at = Some(at)));
    let (tab, target) = (cx.update(|_, app| viewer.read(app).core.tabs[0].id), dir.path().join("out.pdf"));
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_signed(tab, at, signing(""), target.clone(), cx)));
    settle(cx, |app| target.is_file() && view.read(app).sign_at.is_none());
    let mut worker = Worker::spawn().expect("the worker");
    let doc = worker.open(&target).expect("opens").doc;
    let found = worker.signatures(doc, &target).expect("reads");
    assert_eq!((found.len(), found[0].name.as_str()), (1, "Signature1"));
    assert_eq!(cx.update(|_, app| view.read(app).edited), 0);
}

#[test]
fn a_place_from_before_an_edit_signs_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[plain(dir.path())], true);
    let view = shown(cx, &viewer);
    let at = SignAt { page: 0, object: None, rect: [72.0, 600.0, 272.0, 660.0], version: 0 };
    cx.update(|_, app| view.update(app, |view, cx| {
        view.sign_at = Some(at);
        view.edit("pdf.doing.turn", |w, doc| w.rotate_pages(doc, &[0], 90), cx)
    }));
    settle(cx, |app| view.read(app).edited == 1);
    let (tab, target) = (cx.update(|_, app| viewer.read(app).core.tabs[0].id), dir.path().join("out.pdf"));
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_signed(tab, at, signing(""), target.clone(), cx)));
    super::frames(cx);
    assert!(!target.exists());
}

#[test]
fn a_finished_signing_clears_only_its_own_place() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (a, b) = (plain(dir.path()), dir.path().join("b.pdf"));
    std::fs::copy(&a, &b).expect("copies");
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[a, b], true);
    let (first, second) = cx.update(|_, app| (viewer.read(app).core.tabs[0].view.clone(), viewer.read(app).core.tabs[1].view.clone()));
    let at = SignAt { page: 0, object: None, rect: [72.0, 600.0, 272.0, 660.0], version: 0 };
    cx.update(|_, app| {
        first.update(app, |view, _| view.sign_at = Some(at));
        second.update(app, |view, _| view.sign_at = Some(at));
    });
    let (tab, target) = (cx.update(|_, app| viewer.read(app).core.tabs[0].id), dir.path().join("out.pdf"));
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_signed(tab, at, signing(""), target.clone(), cx)));
    settle(cx, |app| target.is_file() && first.read(app).sign_at.is_none());
    assert_eq!(cx.update(|_, app| second.read(app).sign_at), Some(at), "the other tab keeps its place");
}

fn tab_path(cx: &mut VisualTestContext, viewer: &Entity<Viewer>) -> PathBuf {
    cx.update(|_, app| viewer.read(app).active_tab().and_then(|tab| tab.session.path().map(PathBuf::from)).expect("a file"))
}

/// Whether the tab's file lists a signature.
fn shows_signed(cx: &mut VisualTestContext, viewer: &Entity<Viewer>, view: &Entity<PdfView>) -> bool {
    let path = tab_path(cx, viewer);
    cx.update(|_, app| view.update(app, |view, cx| view.ask_signatures(path.clone(), cx)));
    settle(cx, |app| view.read(app).signatures.as_ref().is_some_and(|(read, _)| *read == path));
    cx.update(|_, app| view.read(app).signatures.as_ref().expect("read").1.iter().any(|c| c.field.signed.is_some()))
}

/// Saves the signed tab as `copy`; waits for the reopen.
fn save_signed_as(cx: &mut VisualTestContext, viewer: &Entity<Viewer>, old: &Entity<PdfView>, copy: &Path) -> Entity<PdfView> {
    let id = cx.update(|_, app| viewer.read(app).core.tabs[0].id);
    let target = copy.to_path_buf();
    std::fs::write(copy, b"").expect("a place");
    let copy = dunce::canonicalize(copy).expect("the real path");
    std::fs::remove_file(&copy).expect("clears the place");
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.write_as(id, target, None, window, cx)));
    settle(cx, |app| {
        let read = viewer.read(app);
        read.core.tabs.len() == 1 && read.active_tab().is_some_and(|tab| tab.session.path() == Some(copy.as_path()) && tab.view != *old)
    });
    shown(cx, viewer)
}

#[test]
fn a_signed_file_saved_as_a_copy_reopens_as_the_copy() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = signed(dir.path());
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    assert!(shows_signed(cx, &viewer, &view));
    cx.update(|_, app| view.update(app, |view, cx| view.edit("pdf.doing.turn", |w, doc| w.rotate_pages(doc, &[0], 90), cx)));
    settle(cx, |app| view.read(app).edited == 1);
    let copy = dir.path().join("copy.pdf");
    let reopened = save_signed_as(cx, &viewer, &view, &copy);
    let tab = cx.update(|_, app| viewer.read(app).active_tab().map(|tab| (tab.risk, tab.dirty(app))));
    assert_eq!(tab, Some((None, false)), "an unsigned, saved copy");
    assert!(!shows_signed(cx, &viewer, &reopened), "the copy holds no signature");
    assert_eq!(cx.update(|_, app| reopened.read(app).sizes[0]), (792.0, 612.0), "the edit is in the copy");
}

#[test]
fn signing_after_save_as_signs_the_copy() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = signed(dir.path());
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    let copy = dir.path().join("copy.pdf");
    let reopened = save_signed_as(cx, &viewer, &view, &copy);
    let mut worker = Worker::spawn().expect("the worker");
    let read = worker.open(&copy).expect("opens").doc;
    let field = worker.signatures(read, &copy).expect("reads")[0].clone();
    assert!(field.signed.is_none(), "the copy's field is empty");
    let at = SignAt { page: 0, object: Some(field.object), rect: field.rect, version: cx.update(|_, app| reopened.read(app).version) };
    cx.update(|_, app| reopened.update(app, |view, _| view.sign_at = Some(at)));
    let (tab, target) = (cx.update(|_, app| viewer.read(app).core.tabs[0].id), dir.path().join("resigned.pdf"));
    let mut how = signing("");
    how.object = Some(field.object);
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_signed(tab, at, how, target.clone(), cx)));
    settle(cx, |_| target.is_file());
    let doc = worker.open(&target).expect("opens").doc;
    let found = worker.signatures(doc, &target).expect("reads");
    assert_eq!(found.len(), 1, "only the copy's one field");
    assert_eq!(found[0].signed.as_ref().map(|s| (s.digest, s.later)), Some((Digest::Intact, false)));
}

#[test]
fn a_failed_save_as_keeps_the_signed_file_and_its_signatures() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = signed(dir.path());
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    let (id, blocked) = (cx.update(|_, app| viewer.read(app).core.tabs[0].id), dir.path().join("directory.pdf"));
    std::fs::create_dir(&blocked).expect("a directory in the way");
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.write_as(id, blocked.clone(), None, window, cx)));
    for _ in 0..20 {
        super::frames(cx);
    }
    assert_eq!(cx.update(|_, app| viewer.read(app).active_tab().and_then(|tab| tab.risk)), Some(ely_office::Risk::Signed));
    assert!(shows_signed(cx, &viewer, &view), "the signed file and its list are as they were");
}

#[test]
fn reopening_the_dialog_checks_the_file_as_it_is_now() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = signed(dir.path());
    let (viewer, cx) = windowed(&mut cx, dir.path(), std::slice::from_ref(&path), true);
    let view = shown(cx, &viewer);
    let digest = |cx: &mut VisualTestContext| {
        cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.show_signatures(true, cx)));
        settle(cx, |app| view.read(app).signatures.is_some());
        let digest = cx.update(|_, app| view.read(app).signatures.as_ref().expect("read").1[0].field.signed.as_ref().map(|s| s.digest));
        cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.show_signatures(false, cx)));
        digest
    };
    assert_eq!(digest(cx), Some(Digest::Intact));
    let mut bytes = std::fs::read(&path).expect("reads");
    let at = bytes.windows(5).position(|w| w == b"Hello").expect("the text");
    bytes[at] = b'J';
    std::fs::write(&path, bytes).expect("writes");
    assert_eq!(digest(cx), Some(Digest::Changed));
}
