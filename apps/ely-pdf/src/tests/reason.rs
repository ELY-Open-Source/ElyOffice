//! Step failures say why in words, never a path (R20.70).

use std::io;

use gpui::TestAppContext;

use super::{file, rooted, settle, shown, windowed};
use crate::viewer::Viewer;

/// MuPDF's and the system's words: paths and temp names.
fn raw() -> [io::Error; 4] {
    [
        io::Error::other("cannot open file '/home/u/docs/.tmpfOTKQT': Permission denied"),
        io::Error::new(io::ErrorKind::PermissionDenied, "/home/u/docs/a.pdf: Permission denied (os error 13)"),
        io::Error::new(io::ErrorKind::NotFound, "/home/u/work/print.pdf: No such file or directory (os error 2)"),
        ely_ipc::Worded::then(
            io::Error::new(io::ErrorKind::StorageFull, "/home/u/a.pdf: No space left on device (os error 28)"),
            io::Error::other("cannot remove file '/home/u/.tmpX'"),
        ),
    ]
}

fn clean(said: &str) {
    assert!(!said.contains('/') && !said.contains(".tmp") && !said.contains("os error"), "{said}");
}

#[gpui::test]
fn every_language_says_why_without_paths(cx: &mut TestAppContext) {
    cx.update(|cx| {
        for tag in ["en-US", "zh-CN", "es-ES", "ja-JP", "ko-KR", "vi-VN", "fr-FR", "de-DE"] {
            ely_ui::i18n::install_as(tag, cx);
            for error in raw() {
                let said = crate::reason::reason(cx, &error);
                assert!(!said.contains("pdf.error") && !said.is_empty(), "{tag}: {said}");
                clean(&said);
            }
        }
        ely_ui::i18n::install_as("en-US", cx);
        assert_eq!(crate::reason::reason(cx, &raw()[0]), "the PDF engine could not do it");
        let locked = ely_ipc::Worded::error(io::ErrorKind::PermissionDenied, "pdf.error.locked", &[]);
        assert_eq!(crate::reason::reason(cx, &locked), "a password is needed", "a worded error keeps its words");
    });
}

#[test]
fn each_handler_words_a_system_error() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = file(dir.path(), "doc.pdf", &[(612.0, 792.0, &["First"])]);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    let root = rooted(cx, &viewer);
    let notices = |app: &gpui::App| root.read(app).toasts(app).into_iter().map(|(text, _)| text.to_string()).collect::<Vec<_>>();
    let before = cx.update(|_, app| notices(app).len());
    for (at, error) in raw().into_iter().enumerate() {
        cx.update(|_, app| match at % 3 {
            0 => viewer.update(app, |viewer, cx| viewer.failed("pdf.doing.print", &error, cx)),
            1 => viewer.update(app, |viewer, cx| viewer.failed_read("pdf.doing.preview", &error, cx)),
            _ => view.update(app, |view, cx| view.fail("pdf.doing.text", &error, cx)),
        });
    }
    settle(cx, |app| notices(app).len() >= before + 4);
    let said = cx.update(|_, app| notices(app));
    for notice in &said[before..] {
        clean(notice);
    }
}

/// Waits for a clean Danger notice naming step `what`.
pub(crate) fn named(cx: &mut gpui::VisualTestContext, root: &gpui::Entity<ely_ui::Root>, what: &str) {
    let step = cx.update(|_, app| Viewer::text(app, what, &[]).to_string());
    let danger = Some(ely_gpui_component::primitives::Severity::Danger);
    let found = |app: &gpui::App| {
        let toasts = root.read(app).toasts(app);
        toasts.into_iter().find(|(text, severity)| *severity == danger && text.starts_with(&step)).map(|(text, _)| text.to_string())
    };
    settle(cx, |app| found(app).is_some());
    let said = cx.update(|_, app| found(app)).expect("the notice");
    clean(&said);
    assert!(!said.contains("pdf."), "no raw key: {said}");
}

/// With the worker dead, each step's notice names it.
#[test]
fn each_step_names_itself_when_it_fails() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = file(dir.path(), "doc.pdf", &[(612.0, 792.0, &["Hello world"])]);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    shown(cx, &viewer);
    let root = rooted(cx, &viewer);
    cx.update(|_, app| viewer.read(app).core.worker.lock().expect("the lock").get().expect("a worker").kill()).expect("kills");
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.on_print(&ely_ui::commands::Print, window, cx)));
    named(cx, &root, "pdf.doing.print");
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.ask_properties(window, cx)));
    named(cx, &root, "pdf.doing.read_properties");
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.ask_clean(cx)));
    named(cx, &root, "pdf.doing.clean_scan");
    cx.update(|window, app| viewer.update(app, |viewer, cx| {
        viewer.ask_ocr(window, cx);
        viewer.start_ocr(cx);
    }));
    named(cx, &root, "pdf.doing.ocr");
}
