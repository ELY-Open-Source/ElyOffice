//! LOK-11: Word's linked objects stay links, read on request.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::independent::read_with;
use super::linked::{rates, update};
use super::*;
use crate::links;

/// `source` as an xlsx whose S1.A1 is `value`.
pub(super) fn worksheet(dir: &Path, worker: &mut Worker, source: &Path, value: u32) {
    let flat = dir.join("rates.fods");
    rates(&flat, value);
    let made = worker.load(&crate::file_url(&flat)).expect("loads");
    made.save_as(worker, &crate::file_url(source), "xlsx").expect("the source");
    worker.close_doc(made).expect("closes");
}

/// A saved package's pictures: name, size and CRC.
fn previews(path: &Path) -> String {
    let script = "import sys, zipfile\nfor i in zipfile.ZipFile(sys.argv[1]).infolist():\n    \
                  if i.filename.startswith('word/media/'): print(i.filename, i.file_size, i.CRC)";
    let out = Command::new("python3").args(["-c", script]).arg(path).output().expect("python3 runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).expect("text")
}

/// Opens `main`, updates twice, then misses its source.
fn stays_a_link(dir: &Path, worker: &mut Worker, main: &Path) {
    let source = dir.join("source.xlsx");
    worksheet(dir, worker, &source, 4);
    let doc = worker.load(&crate::file_url(main)).expect("loads");
    let loaded = poll_until(worker, &doc, &Events::default(), HANG, super::docx::modified);
    assert!(!super::docx::modified(&loaded), "the link made at load marked it modified");
    let named = links::sources(&worker.values(&doc, links::LIST).expect("reads")).expect("parses");
    assert_eq!(named, [crate::file_url(&source)], "the source under its real URL");
    let saved = |worker: &mut Worker, doc: &Doc, name: &str| {
        let out = dir.join(name);
        doc.save_as(worker, &crate::file_url(&out), "docx").expect("saves");
        read_with("ooxml_word.py", &out, "ole-link");
        (previews(&out), out)
    };
    let (opened, _) = saved(worker, &doc, "opened.docx");
    assert_eq!(update(worker, &doc), Vec::<String>::new());
    let (updated, _) = saved(worker, &doc, "updated.docx");
    assert_ne!(updated, opened, "the update redraws the preview");
    worksheet(dir, worker, &source, 9);
    assert_eq!(update(worker, &doc), Vec::<String>::new());
    let (again, out) = saved(worker, &doc, "again.docx");
    assert_ne!(again, updated, "a changed source redraws it again");
    std::fs::remove_file(&source).expect("the source goes");
    let reopened = worker.load(&crate::file_url(&out)).expect("reloads");
    assert_eq!(update(worker, &reopened), [crate::file_url(&source)], "the missing file is named");
    saved(worker, &reopened, "missing.docx");
}

fn fresh() -> (tempfile::TempDir, Worker) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let install = install().expect("the pinned LOK build");
    let worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    (dir, worker)
}

#[test]
fn a_linked_worksheet_stays_a_link_through_saves() {
    if super::analysis::skip() {
        return;
    }
    let (dir, mut worker) = fresh();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office/ole-link.docx");
    let main = dir.path().join("main.docx");
    std::fs::copy(fixture, &main).expect("copies");
    stays_a_link(dir.path(), &mut worker, &main);
}

#[test]
fn word_s_absolute_target_finds_its_file() {
    if super::analysis::skip() {
        return;
    }
    let (dir, mut worker) = fresh();
    let source = dir.path().join("source.xlsx");
    // Word writes file:/// and the system path, backslashes kept.
    let target = match cfg!(windows) {
        true => format!("file:///{}", source.display()),
        false => format!("file://{}", source.display()),
    };
    let made = dir.path().join("made");
    std::fs::create_dir(&made).expect("a folder");
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office/make-packages.py");
    let out = Command::new("python3").arg(script).arg(&made).arg(&target).output().expect("python3 runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let main: PathBuf = made.join("ole-link-word.docx");
    stays_a_link(dir.path(), &mut worker, &main);
}

#[test]
fn a_link_of_another_kind_is_named_even_in_a_locked_file() {
    if super::analysis::skip() {
        return;
    }
    let (dir, mut worker) = fresh();
    let office = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office");
    let cases = [
        ("ole-link.docx", "", Vec::<&str>::new()),
        ("includetext.docx", "", Vec::new()),
        ("ole-link-other.docx", "", vec!["source.pdf"]),
        ("ole-link-other-locked.docx", "ely", vec!["source.pdf"]),
    ];
    for (name, password, want) in cases {
        let copy = dir.path().join(name);
        std::fs::copy(office.join(name), &copy).expect("copies");
        let doc = worker.unlock(&crate::file_url(&copy), password).expect(name);
        let listed = worker.values(&doc, links::LIST).expect("reads");
        assert_eq!(links::pictured(&listed).expect("parses"), want, "{name}");
        worker.close_doc(doc).expect("closes");
    }
}
