//! OCR and the hidden text layer, real worker (PDF-08).

use std::path::{Path, PathBuf};

use ely_pdf_engine::{
    Doc, OcrLine, Worker, Writing, Wrote,
    sample::{pdf, scan},
    tessdata,
};

fn worker() -> Worker {
    Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker")
}

/// Points a pixel when scanning.
const SCALE: f32 = 200.0 / 72.0;

/// Page one of `doc`, saved as a gray scan.
fn scan_of(worker: &mut Worker, doc: Doc, out: PathBuf) -> PathBuf {
    let (w, h) = ((612.0 * SCALE).ceil() as i32, (792.0 * SCALE).ceil() as i32);
    let bgra = worker.render(doc, 0, (SCALE, 0), [0, 0, w, h]).expect("draws").bgra;
    let gray: Vec<u8> = bgra.chunks_exact(4).map(|p| ((u16::from(p[0]) + u16::from(p[1]) + u16::from(p[2])) / 3) as u8).collect();
    std::fs::write(&out, scan((612.0, 792.0), (w as u32, h as u32), &gray)).expect("writes");
    out
}

/// A scan of two English lines, opened.
fn english(dir: &Path) -> (Worker, Doc) {
    let source = dir.join("text.pdf");
    std::fs::write(&source, pdf(&[(612.0, 792.0, &["Hello world", "Second line"])], &[])).expect("writes");
    let mut worker = worker();
    let doc = worker.open(&source).expect("opens").doc;
    let path = scan_of(&mut worker, doc, dir.join("scan.pdf"));
    let doc = worker.open(&path).expect("opens").doc;
    (worker, doc)
}

fn words(worker: &mut Worker, doc: Doc) -> String {
    worker.text(doc, 0).expect("text").iter().map(|c| c.c).collect()
}

#[test]
fn a_scan_reads_and_its_text_layer_finds_without_drawing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = english(dir.path());
    assert_eq!(words(&mut worker, doc), "", "a scan has no text");
    let lines = worker.ocr_page(doc, 0, "eng", &tessdata().expect("data"), 300.0).expect("reads");
    let read: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
    assert!(read.iter().any(|l| l.contains("Hello world")) && read.iter().any(|l| l.contains("Second line")), "{read:?}");
    let hello = lines.iter().find(|l| l.text.contains("Hello")).expect("hello");
    assert!((hello.rect[0] - 72.0).abs() < 6.0 && (hello.rect[3] - 76.0).abs() < 8.0, "in page points: {:?}", hello.rect);
    let before = worker.render(doc, 0, (1.0, 0), [0, 0, 612, 792]).expect("draws").bgra;
    let mark = worker.text_layer(doc, &[(0, lines)], false).expect("lays");
    doc = worker.now(doc).expect("the state");
    assert!(words(&mut worker, doc).contains("Hello world"), "searchable now");
    assert_eq!(worker.search(doc, 0, "second line").expect("finds").len(), 1);
    assert!(worker.render(doc, 0, (1.0, 0), [0, 0, 612, 792]).expect("draws").bgra == before, "nothing drawn");
    worker.undo(doc, false).expect("undoes");
    doc = worker.now(doc).expect("the state");
    assert_eq!(words(&mut worker, doc), "", "one step");
    assert_eq!(worker.undo(doc, true).expect("redoes"), mark);
    doc = worker.now(doc).expect("the state");
    let saved = dir.path().join("saved.pdf");
    worker.save(doc, &saved).expect("saves");
    let mut other = self::worker();
    let reopened = other.open(&saved).expect("opens").doc;
    assert!(words(&mut other, reopened).contains("Hello world"), "kept in the file");
}

#[test]
fn a_new_layer_replaces_the_last_or_adds_beside_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = english(dir.path());
    let line = OcrLine { rect: [72.0, 60.0, 200.0, 78.0], text: "Hello world".into() };
    worker.text_layer(doc, &[(0, vec![line.clone()])], false).expect("lays");
    doc = worker.now(doc).expect("the state");
    worker.text_layer(doc, &[(0, vec![line.clone()])], false).expect("lays again");
    doc = worker.now(doc).expect("the state");
    assert_eq!(worker.search(doc, 0, "hello").expect("finds").len(), 2, "two layers");
    worker.text_layer(doc, &[(0, vec![line])], true).expect("replaces");
    doc = worker.now(doc).expect("the state");
    assert_eq!(worker.search(doc, 0, "hello").expect("finds").len(), 1, "old layers gone");
}

#[test]
fn a_bad_layer_changes_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = english(dir.path());
    let before = worker.history(doc).expect("a mark");
    let good = (0, vec![OcrLine { rect: [72.0, 60.0, 200.0, 78.0], text: "fine".into() }]);
    for bad in [
        (0, vec![OcrLine { rect: [72.0, 60.0, 200.0, 78.0], text: "𠀀".into() }]),
        (0, vec![OcrLine { rect: [72.0, 60.0, 200.0, 78.0], text: String::new() }]),
        (5, vec![OcrLine { rect: [72.0, 60.0, 200.0, 78.0], text: "x".into() }]),
    ] {
        assert!(worker.text_layer(doc, &[good.clone(), bad.clone()], true).is_err(), "{bad:?}");
    }
    assert_eq!((worker.history(doc).expect("a mark"), words(&mut worker, doc)), (before, String::new()));
}

#[test]
fn chinese_reads_in_both_scripts() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let source = dir.path().join("blank.pdf");
    std::fs::write(&source, pdf(&[(612.0, 792.0, &[])], &[])).expect("writes");
    let mut worker = worker();
    let mut doc = worker.open(&source).expect("opens").doc;
    for (font, text, at) in [("cjk-sc", "简体中文识别", 100.0), ("cjk-tc", "繁體中文識別", 300.0)] {
        let writing = Writing { block: None, expected: String::new(), text: text.into(), font: font.into(), size: 36.0, rgb: 0, bx: [72.0, at, 400.0] };
        assert!(matches!(worker.put_text(doc, 0, &writing, 0.0).expect("writes"), Wrote::Done(_)));
        doc = worker.now(doc).expect("the state");
    }
    let path = scan_of(&mut worker, doc, dir.path().join("scan.pdf"));
    let doc = worker.open(&path).expect("opens").doc;
    let lines = worker.ocr_page(doc, 0, "chi_sim+chi_tra+eng", &tessdata().expect("data"), 300.0).expect("reads");
    let read: String = lines.iter().map(|l| l.text.replace(' ', "")).collect();
    assert!(read.contains("简体中文") && read.contains("繁體中文"), "{read}");
}

#[test]
fn a_bad_language_says_so() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = english(dir.path());
    let error = worker.ocr_page(doc, 0, "xyz", &tessdata().expect("data"), 300.0).expect_err("no such data");
    assert!(error.to_string().contains("Tesseract"), "{error}");
    assert!(worker.ocr_page(doc, 0, "eng", &tessdata().expect("data"), 300.0).is_ok(), "the worker goes on");
}

#[test]
fn a_language_that_does_not_load_fails_the_read() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = english(dir.path());
    let data = dir.path().join("tessdata");
    std::fs::create_dir(&data).expect("a data dir");
    std::fs::copy(tessdata().expect("data").join("eng.traineddata"), data.join("eng.traineddata")).expect("copies");
    let error = worker.ocr_page(doc, 0, "eng+chi_sim", &data, 300.0).expect_err("chi_sim is missing");
    assert!(error.to_string().contains("1 of 2 languages"), "{error}");
}

#[test]
fn text_added_after_a_layer_still_shows() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = english(dir.path());
    let line = OcrLine { rect: [72.0, 60.0, 200.0, 78.0], text: "Hello world".into() };
    worker.text_layer(doc, &[(0, vec![line])], false).expect("lays");
    doc = worker.now(doc).expect("the state");
    let before = worker.render(doc, 0, (1.0, 0), [0, 0, 612, 792]).expect("draws").bgra;
    let writing = Writing { block: None, expected: String::new(), text: "VISIBLE".into(), font: "helvetica".into(), size: 24.0, rgb: 0, bx: [72.0, 600.0, 300.0] };
    assert!(matches!(worker.put_text(doc, 0, &writing, 0.0).expect("writes"), Wrote::Done(_)));
    doc = worker.now(doc).expect("the state");
    assert!(worker.render(doc, 0, (1.0, 0), [0, 0, 612, 792]).expect("draws").bgra != before, "the new text is drawn");
}
