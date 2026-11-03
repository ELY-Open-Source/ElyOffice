//! Exports and the repaired copy (C11.8).

use std::path::PathBuf;

use ely_office::{Saving as _, Tabs as _};
use ely_pdf_engine::Optimizing;
use gpui::TestAppContext;

use super::{file, settle, shown, windowed};
use crate::output::{Kind, image_names};

#[test]
fn image_names_pad_alike() {
    assert_eq!(image_names("doc", &[0, 9], 12), ["doc-01.png", "doc-10.png"]);
}

#[test]
fn pages_export_as_images_and_text() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = file(dir.path(), "doc.pdf", &[(612.0, 792.0, &["First"]), (612.0, 792.0, &["Second"])]);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    shown(cx, &viewer);
    let id = cx.update(|_, app| viewer.read(app).core.tabs[0].id);
    let targets: Vec<PathBuf> = ["doc-1.png", "doc-2.png"].iter().map(|name| dir.path().join(name)).collect();
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_images(id, (vec![0, 1], 0), 72.0, targets.clone(), cx)));
    settle(cx, |_| targets.iter().all(|target| std::fs::metadata(target).is_ok_and(|meta| meta.len() > 100)));
    let txt = dir.path().join("doc.txt");
    let how = Optimizing { compress: true, from: 300, to: 0 };
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_export(id, Kind::Text, how, txt.clone(), cx)));
    settle(cx, |_| txt.is_file());
    let text = std::fs::read_to_string(&txt).expect("UTF-8");
    assert!(text.contains("First") && text.contains("Second"), "{text}");
}

#[cfg(unix)]
#[test]
fn a_write_the_system_refuses_reports_it_without_a_crash() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = file(dir.path(), "doc.pdf", &[(612.0, 792.0, &["First"])]);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    shown(cx, &viewer);
    let locked = dir.path().join("locked");
    std::fs::create_dir(&locked).expect("a folder");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o500)).expect("read-only");
    let id = cx.update(|_, app| viewer.read(app).core.tabs[0].id);
    let txt = locked.join("doc.txt");
    let how = Optimizing { compress: true, from: 300, to: 0 };
    // The window's toasts live on its root.
    let root = cx.update(|window, app| {
        let root = ely_ui::document_root(ely_platform::AppId::Pdf, viewer.clone().into(), &ely_ui::commands::FILE, Vec::new(), window, app);
        viewer.update(app, |viewer, _| viewer.core.root = root.downgrade());
        root
    });
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_export(id, Kind::Text, how, txt.clone(), cx)));
    let failed = |app: &gpui::App| {
        let toasts = root.read(app).toasts(app);
        toasts.iter().any(|(text, severity)| {
            *severity == Some(ely_gpui_component::primitives::Severity::Danger) && text.contains("Permission denied")
        })
    };
    settle(cx, failed);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o700)).expect("writable again");
    assert!(!txt.exists(), "nothing is written");
}

#[test]
fn a_damaged_file_opens_repaired_and_saves_as_a_new_file() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = file(dir.path(), "broken.pdf", &[(612.0, 792.0, &["Hello"])]);
    let mut bytes = std::fs::read(&path).expect("reads");
    let at = bytes.windows(9).rposition(|w| w == b"startxref").expect("the trailer");
    bytes.truncate(at);
    bytes.extend_from_slice(b"startxref\n9\n%%EOF\n");
    std::fs::write(&path, &bytes).expect("writes");
    let (viewer, cx) = windowed(&mut cx, dir.path(), std::slice::from_ref(&path), true);
    let view = shown(cx, &viewer);
    assert_eq!(cx.update(|_, app| viewer.read(app).active_tab().and_then(|tab| tab.risk)), Some(ely_office::Risk::Repaired));
    assert!(cx.update(|_, app| view.read(app).repair.as_ref().is_some_and(|report| !report.is_empty())));
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.show_repair(true, cx)));
    super::frames(cx);
    let id = cx.update(|_, app| viewer.read(app).core.tabs[0].id);
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.write_as(id, path.clone(), None, window, cx)));
    super::frames(cx);
    assert_eq!(std::fs::read(&path).expect("reads"), bytes, "Save As refuses the damaged original");
    let fixed = dir.path().join("fixed.pdf");
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.write_as(id, fixed.clone(), None, window, cx)));
    settle(cx, |app| fixed.is_file() && viewer.read(app).active_tab().is_some_and(|tab| tab.risk.is_none() && tab.view != view));
}

#[test]
fn an_edit_made_while_saving_stays_and_the_tab_reopens_after_the_next_save() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = file(dir.path(), "broken.pdf", &[(612.0, 792.0, &["Hello"])]);
    let mut bytes = std::fs::read(&path).expect("reads");
    let at = bytes.windows(9).rposition(|w| w == b"startxref").expect("the trailer");
    bytes.truncate(at);
    bytes.extend_from_slice(b"startxref\n9\n%%EOF\n");
    std::fs::write(&path, &bytes).expect("writes");
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    let (id, fixed) = (cx.update(|_, app| viewer.read(app).core.tabs[0].id), dir.path().join("fixed.pdf"));
    cx.update(|window, app| viewer.update(app, |viewer, cx| {
        viewer.write_as(id, fixed.clone(), None, window, cx);
        // Lands after the save took its snapshot.
        view.update(cx, |view, cx| view.edit("pdf.doing.turn", |w, doc| w.rotate_pages(doc, &[0], 90), cx));
    }));
    settle(cx, |app| fixed.is_file() && view.read(app).edited == 1 && viewer.read(app).active_tab().is_some_and(|tab| !tab.exporting));
    for _ in 0..10 {
        super::frames(cx);
    }
    let tab = cx.update(|_, app| viewer.read(app).active_tab().map(|tab| (tab.view == view, tab.dirty(app))));
    assert_eq!(tab, Some((true, true)), "the edit stays, unsaved");
    assert_eq!(cx.update(|_, app| view.read(app).sizes[0]), (792.0, 612.0));
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.save(id, None, window, cx)));
    settle(cx, |app| viewer.read(app).active_tab().is_some_and(|tab| tab.view != view && !tab.reopen));
    let reopened = shown(cx, &viewer);
    assert_eq!(cx.update(|_, app| reopened.read(app).sizes[0]), (792.0, 612.0), "the reopened file holds the edit");
}

#[test]
fn picked_pages_from_before_an_edit_export_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = file(dir.path(), "doc.pdf", &[(100.0, 100.0, &["One"]), (200.0, 100.0, &["Two"]), (300.0, 100.0, &["Three"])]);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    let id = cx.update(|_, app| viewer.read(app).core.tabs[0].id);
    let target = dir.path().join("doc-2.png");
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        let version = view.read(cx).version;
        view.update(cx, |view, cx| view.edit("pdf.doing.delete_pages", |w, doc| w.delete_pages(doc, &[0]), cx));
        viewer.write_images(id, (vec![1], version), 72.0, vec![target.clone()], cx);
    }));
    settle(cx, |app| view.read(app).edited == 1 && view.read(app).settled());
    for _ in 0..10 {
        super::frames(cx);
    }
    assert!(!target.exists(), "page 2 is another page now");
}

/// A page drawing `/Im`, a picture that is no zlib.
fn broken_picture(dir: &std::path::Path, height: f32, y: f32) -> (PathBuf, Vec<u8>) {
    let picture = "<< /Type /XObject /Subtype /Image /Width 4 /Height 4 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length 12 >>\nstream\nnot a zlib!!\nendstream".to_string();
    let page = format!(" /MediaBox [0 0 612 {height}] /Resources << /XObject << /Im 5 0 R >> >>");
    let bytes = ely_pdf_engine::sample::one_page("", &page, &format!("q 100 0 0 100 72 {y} cm /Im Do Q"), &[picture]);
    let path = dir.join("broken.pdf");
    std::fs::write(&path, &bytes).expect("writes");
    (path, bytes)
}

#[test]
fn a_picture_that_breaks_as_it_draws_stops_an_overwrite() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (path, bytes) = broken_picture(dir.path(), 792.0, 600.0);
    let (viewer, cx) = windowed(&mut cx, dir.path(), std::slice::from_ref(&path), true);
    shown(cx, &viewer);
    settle(cx, |app| viewer.read(app).active_tab().and_then(|tab| tab.risk) == Some(ely_office::Risk::Repaired));
    let id = cx.update(|_, app| viewer.read(app).core.tabs[0].id);
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.save(id, None, window, cx)));
    for _ in 0..10 {
        super::frames(cx);
    }
    assert_eq!(std::fs::read(&path).expect("reads"), bytes);
}

#[test]
fn a_picture_never_drawn_is_checked_before_an_overwrite() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (one, _) = broken_picture(dir.path(), 792.0, 600.0);
    // Thirty blank pages first: the picture's page is never drawn.
    let mut worker = ely_pdf_engine::Worker::spawn().expect("the worker");
    let mut doc = worker.open(&one).expect("opens").doc;
    for _ in 0..30 {
        let mark = worker.insert_blank(doc, 0, (612.0, 792.0)).expect("inserts");
        doc = doc.at(&mark).expect("the new state");
    }
    let path = dir.path().join("long.pdf");
    worker.save(doc, &path).expect("saves");
    let bytes = std::fs::read(&path).expect("reads");
    let (viewer, cx) = windowed(&mut cx, dir.path(), std::slice::from_ref(&path), true);
    shown(cx, &viewer);
    for _ in 0..10 {
        super::frames(cx);
    }
    let id = cx.update(|_, app| viewer.read(app).core.tabs[0].id);
    assert_eq!(cx.update(|_, app| viewer.read(app).active_tab().and_then(|tab| tab.risk)), None, "nothing drawn yet");
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.save(id, None, window, cx)));
    settle(cx, |app| viewer.read(app).active_tab().and_then(|tab| tab.risk) == Some(ely_office::Risk::Repaired));
    assert_eq!(std::fs::read(&path).expect("reads"), bytes, "Save turned into Save As");
}

#[test]
fn an_edited_document_does_not_quote_the_files_size() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = file(dir.path(), "doc.pdf", &[(612.0, 792.0, &["First"])]);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    // The window's toasts live on its root.
    let root = cx.update(|window, app| {
        let root = ely_ui::document_root(ely_platform::AppId::Pdf, viewer.clone().into(), &ely_ui::commands::FILE, Vec::new(), window, app);
        viewer.update(app, |viewer, _| viewer.core.root = root.downgrade());
        root
    });
    let note = ely_pdf_engine::NewAnnot { page: 0, shape: ely_pdf_engine::Shape::Note((100.0, 500.0)), rgb: None, contents: "n".into(), author: String::new() };
    cx.update(|_, app| view.update(app, |view, cx| view.annotate("pdf.doing.add_note", vec![note], cx)));
    settle(cx, |app| view.read(app).settled() && view.read(app).modified());
    let id = cx.update(|_, app| viewer.read(app).core.tabs[0].id);
    let out = dir.path().join("small.pdf");
    let how = Optimizing { compress: true, from: 300, to: 150 };
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_export(id, Kind::Optimized, how, out.clone(), cx)));
    let done = cx.update(|_, app| crate::viewer::Viewer::text(app, "pdf.export.done", &[("name", "small.pdf")]));
    settle(cx, |app| root.read(app).toasts(app).iter().any(|(text, _)| *text == done));
}
