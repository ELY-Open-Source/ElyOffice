//! Limits, the owner password and protected copies (C11.7).

use ely_pdf_engine::{Lock, Worker, proto::may, sample::{Extras, pdf_with}};
use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{frames, settle, shown, windowed};
use crate::protect::Secure;
use crate::select::Selection;
use crate::view::PdfView;
use crate::viewer::Viewer;

/// Opens a copy with no open password allowing only `allow`.
fn limited<'a>(cx: &'a mut TestAppContext, dir: &std::path::Path, allow: u32) -> (Entity<Viewer>, &'a mut VisualTestContext, Entity<PdfView>) {
    let plain = dir.join("plain.pdf");
    std::fs::write(&plain, pdf_with(&[(612.0, 792.0, &["Hello world"])], &[], &Extras::default())).expect("writes");
    let mut worker = Worker::spawn().expect("the worker");
    let doc = worker.open(&plain).expect("opens").doc;
    let path = dir.join("limited.pdf");
    let lock = Lock { user: String::new(), owner: "owner".into(), allow };
    worker.protect(doc, &path, Some(&lock)).expect("protects");
    let (viewer, cx) = windowed(cx, dir, &[path], true);
    let view = shown(cx, &viewer);
    (viewer, cx, view)
}

fn type_in(cx: &mut VisualTestContext, viewer: &Entity<Viewer>, owner: bool, text: &str) {
    cx.update(|_, app| {
        let field = match viewer.read(app).secure.as_ref() {
            Some(Secure::Owner { field, .. }) => field.clone(),
            Some(Secure::Protect { user, owner: field, .. }) => if owner { field.clone() } else { user.clone() },
            None => panic!("no dialog"),
        };
        field.update(app, |field, cx| field.set_text(text.to_string(), cx));
    });
}

#[test]
fn limits_hold_until_the_owner_password_lifts_them() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = limited(&mut cx, dir.path(), may::PRINT);
    assert_eq!(cx.update(|_, app| view.read(app).allowed) & (may::ASSEMBLE | may::OWNER), 0);
    cx.update(|_, app| view.update(app, |view, cx| view.edit("pdf.doing.turn", |w, doc| w.rotate_pages(doc, &[0], 90), cx)));
    settle(cx, |app| view.read(app).settled());
    assert_eq!(cx.update(|_, app| view.read(app).edited), 0, "refused");
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.ask_owner(window, cx)));
    type_in(cx, &viewer, true, "wrong");
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.try_owner(cx)));
    frames(cx);
    assert!(cx.update(|_, app| viewer.read(app).secure.is_some()), "asks again");
    type_in(cx, &viewer, true, "owner");
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.try_owner(cx)));
    settle(cx, |app| view.read(app).allowed & may::OWNER != 0);
    assert!(cx.update(|_, app| viewer.read(app).secure.is_none()));
    cx.update(|_, app| view.update(app, |view, cx| view.edit("pdf.doing.turn", |w, doc| w.rotate_pages(doc, &[0], 90), cx)));
    settle(cx, |app| view.read(app).edited == 1);
}

#[test]
fn copying_is_refused_where_the_file_forbids_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = limited(&mut cx, dir.path(), may::PRINT);
    cx.update(|_, app| view.update(app, |view, cx| view.page_text(0, cx)));
    settle(cx, |app| view.read(app).text.contains_key(&0));
    cx.update(|_, app| view.update(app, |view, cx| {
        view.selection = Some(Selection { anchor: (0, (70.0, 60.0)), head: (0, (140.0, 75.0)) });
        view.copy(cx);
    }));
    frames(cx);
    assert!(cx.update(|_, app| app.read_from_clipboard()).is_none());
}

#[test]
fn protecting_needs_a_distinct_owner_password_and_writes_a_locked_copy() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = super::file(dir.path(), "doc.pdf", &[(612.0, 792.0, &["Hello"])]);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    shown(cx, &viewer);
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.ask_protect(window, cx)));
    type_in(cx, &viewer, false, "same");
    type_in(cx, &viewer, true, "same");
    assert!(cx.update(|_, app| viewer.read(app).lock(app)).is_none(), "owner equals open");
    type_in(cx, &viewer, true, "owner");
    let lock = cx.update(|_, app| viewer.read(app).lock(app)).flatten().expect("a lock");
    let tab = cx.update(|_, app| viewer.read(app).core.tabs[0].id);
    let target = dir.path().join("locked.pdf");
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_protected(tab, Some(lock), target.clone(), cx)));
    settle(cx, |_| target.is_file());
    let mut worker = Worker::spawn().expect("the worker");
    let opened = worker.open(&target).expect("opens");
    assert!(opened.locked);
    assert!(worker.unlock(opened.doc, "same").expect("tries"));
}

#[test]
fn an_owner_reply_closes_only_its_own_dialog() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = limited(&mut cx, dir.path(), may::PRINT);
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.ask_owner(window, cx)));
    type_in(cx, &viewer, true, "owner");
    cx.update(|window, app| viewer.update(app, |viewer, cx| {
        viewer.try_owner(cx);
        // A newer dialog opens before the answer.
        viewer.secure = None;
        viewer.ask_owner(window, cx);
    }));
    settle(cx, |app| view.read(app).allowed & may::OWNER != 0);
    assert!(cx.update(|_, app| viewer.read(app).secure.is_some()), "the newer dialog stays");
}
