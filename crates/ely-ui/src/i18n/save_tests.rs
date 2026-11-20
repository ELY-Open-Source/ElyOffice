//! A failed save's words: no temp name, no path (R20.64).

use std::io;

use super::{Worded, install_as, save_error, text};

/// What `save_atomic` gave before: a path and the system's text.
fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "/home/u/docs/a.docx: Permission denied (os error 13) at path \"/home/u/docs/.tmpfOTKQT\"")
}

/// What LOK's doc_saveAs reports when the store throws.
fn lok() -> io::Error {
    io::Error::other("saveAs failed: exception: SfxBaseModel::impl_store <file:///home/u/.local/share/ElyOffice/ely-doc/work/doc-1/a.docx> failed: 0xc10(Error Area:Io Class:Write Code:16)")
}

/// What MuPDF throws when its output will not open.
fn mupdf() -> io::Error {
    io::Error::other("cannot open file '/home/u/docs/.tmpfOTKQT': Permission denied")
}

#[gpui::test]
fn a_failed_save_says_why_and_what_to_do_without_paths(cx: &mut gpui::TestAppContext) {
    let folder = Worded::error(io::ErrorKind::PermissionDenied, "office.save.folder", &[("folder", "docs")]);
    let full = io::Error::new(io::ErrorKind::StorageFull, "/home/u/docs/a.docx: No space left on device (os error 28)");
    let gone = io::Error::new(io::ErrorKind::NotFound, "/home/u/docs/a.docx: No such file or directory (os error 2)");
    cx.update(|cx| {
        install_as("en-US", cx);
        let toast = |error: &io::Error, cx: &gpui::App| text(cx, "office.error.save", &[("name", "a.docx"), ("error", &save_error(cx, error))]);
        assert_eq!(toast(&folder, cx), "a.docx did not save: the folder docs can't be written to; use Save As… to save it somewhere else");
        for error in [&folder, &denied(), &full, &gone] {
            let said = toast(error, cx);
            assert!(!said.contains('/') && !said.contains(".tmp") && !said.contains("os error"), "{said}");
            assert!(said.contains("Save As…"), "says what to do: {said}");
        }
        assert!(save_error(cx, &denied()).starts_with("the file can't be written to"));
        assert!(save_error(cx, &full).starts_with("the disk is full"));
        // LOK and MuPDF quote the file they could not write.
        for engine in [lok(), mupdf(), Worded::then(lok(), mupdf())] {
            let said = toast(&engine, cx);
            assert!(!said.contains('/') && !said.contains("file:") && !said.contains(".tmp"), "{said}");
        }
        assert!(save_error(cx, &lok()).starts_with("the document engine could not write it"));
        // A failed write, then a failed cleanup, both with paths.
        let cleanup = io::Error::new(io::ErrorKind::PermissionDenied, "/home/u/docs/part-2.pdf: Permission denied (os error 13)");
        let both = toast(&Worded::then(denied(), cleanup), cx);
        assert!(!both.contains('/') && !both.contains("os error") && both.contains("Save As…"), "{both}");
        let root = Worded::error(io::ErrorKind::PermissionDenied, "office.save.root", &[]);
        assert!(save_error(cx, &root).starts_with("the drive's top folder"));
    });
}

#[gpui::test]
fn every_language_words_a_failed_save_without_paths(cx: &mut gpui::TestAppContext) {
    let folder = Worded::error(io::ErrorKind::PermissionDenied, "office.save.folder", &[("folder", "docs")]);
    let kinds = [io::ErrorKind::PermissionDenied, io::ErrorKind::StorageFull, io::ErrorKind::NotFound, io::ErrorKind::Other];
    cx.update(|cx| {
        for tag in ["en-US", "zh-CN", "es-ES", "ja-JP", "ko-KR", "vi-VN", "fr-FR", "de-DE"] {
            install_as(tag, cx);
            let mut said = vec![save_error(cx, &folder)];
            said.extend(kinds.map(|kind| save_error(cx, &io::Error::new(kind, "/home/u/docs/.tmpfOTKQT: os error"))));
            assert!(said[0].contains("docs"), "{tag}: the folder is named: {}", said[0]);
            for words in &said {
                assert!(!words.contains('/') && !words.contains(".tmp") && !words.contains("office.save"), "{tag}: {words}");
            }
        }
    });
}
