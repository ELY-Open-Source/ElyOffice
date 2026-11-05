//! Recovery snapshots give way to waiting input (R15.56).

use std::{
    io::ErrorKind,
    path::Path,
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

use super::docx::modified;
use super::roundtrip::type_in;
use super::*;

/// A key waits no longer than this (PERF-02).
const LAG: Duration = Duration::from_millis(100);

/// A deck of `slides` slides, five text boxes each.
fn big_deck(slides: usize) -> String {
    let frame = |n: usize, y: usize| {
        format!(r#"<draw:frame svg:x="2cm" svg:y="{y}cm" svg:width="20cm" svg:height="2cm"><draw:text-box><text:p>Slide {n} line {y} makes the deck big.</text:p></draw:text-box></draw:frame>"#)
    };
    let page = |n| format!(r#"<draw:page draw:name="S{n}">{}</draw:page>"#, (1..6).map(|y| frame(n, y * 3)).collect::<String>());
    let (head, rest) = super::show::DECK.split_once("<office:presentation>").expect("the deck's body");
    let (_, tail) = rest.split_once("</office:presentation>").expect("the body's end");
    format!("{head}<office:presentation>{}</office:presentation>{tail}", (0..slides).map(page).collect::<String>())
}

#[test]
fn a_waiting_key_stops_a_snapshot() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let deck = dir.path().join("big.fodp");
    write(&deck, &big_deck(400));
    let doc = worker.load(&crate::file_url(&deck)).expect("the deck loads");
    let point = crate::file_url(&dir.path().join("point.pptx"));
    let began = Instant::now();
    doc.snapshot(&mut worker, &point, "pptx").expect("a whole snapshot");
    let whole = began.elapsed();
    assert!(whole > LAG * 4, "too quick a snapshot to tell: {whole:?}");
    let waiter = worker.waiter();
    let shared = Arc::new(Mutex::new(worker));
    let (taken, held) = mpsc::channel();
    let snapshot = {
        let (shared, doc, point) = (Arc::clone(&shared), doc.clone(), point.clone());
        std::thread::spawn(move || {
            let mut worker = shared.lock().expect("the worker lock");
            taken.send(()).expect("the test waits");
            doc.snapshot(&mut worker, &point, "pptx")
        })
    };
    held.recv().expect("the snapshot holds the worker");
    std::thread::sleep(whole / 4);
    let began = Instant::now();
    waiter.hold(&shared).post_key(&doc, 'x' as u32, 0).expect("the key");
    let waited = began.elapsed();
    let stopped = snapshot.join().expect("the snapshot thread");
    assert!(waited < LAG, "the key waited {waited:?} behind a {whole:?} snapshot");
    let error = stopped.expect_err("the key stopped the snapshot");
    assert_eq!(error.kind(), ErrorKind::Interrupted, "{error}");
}

/// The document as flat XML, less what each load draws.
fn flat(worker: &mut Worker, doc: &Doc, path: &Path) -> String {
    doc.save_as(worker, &crate::file_url(path), path.extension().and_then(|ext| ext.to_str()).expect("a format"))
        .expect("the flat copy saves");
    let text = std::fs::read_to_string(path).expect("the flat copy reads");
    let (head, rest) = text.split_once("<office:meta>").expect("metadata");
    let (_, tail) = rest.split_once("</office:meta>").expect("the metadata's end");
    // Each load has its own window place, rsids, change ids.
    let drawn = ["WindowState", "Rsid", "RsidRoot"].map(|name| format!(r#"config:name="{name}""#));
    let kept = |line: &&str| !drawn.iter().any(|name| line.contains(name.as_str()));
    format!("{head}{tail}").lines().filter(kept).map(unnumbered).collect::<Vec<_>>().join("
")
}

/// `line` with Writer's change ids (`"ct` and an address) cut.
fn unnumbered(line: &str) -> String {
    let (mut out, mut rest) = (String::new(), line);
    while let Some(at) = rest.find("\"ct") {
        out.push_str(&rest[..at + 3]);
        rest = rest[at + 3..].trim_start_matches(|c: char| c.is_ascii_digit());
    }
    out + rest
}

/// Equal texts, or the first lines that differ.
fn same(left: &str, right: &str, what: &str) {
    let differ = left.lines().zip(right.lines()).position(|(a, b)| a != b);
    let near = |text: &str, at: usize| text.lines().skip(at).take(3).collect::<Vec<_>>().join("
");
    match differ {
        Some(at) => panic!("{what}: line {at}
{}
---
{}", near(left, at), near(right, at)),
        None => assert_eq!(left.lines().count(), right.lines().count(), "{what}: lengths"),
    }
}

/// The edited `file`: a stopped snapshot, then a whole one.
fn stopped_then_whole(file: &Path, (format, flat_format): (&str, &str), edit: impl FnOnce(&mut Worker, &Doc)) -> (String, String) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install().expect("LOK"), &dir.path().join("profile")).expect("the worker");
    let doc = worker.load(&crate::file_url(file)).expect("the file loads");
    let at = |name: &str, ext: &str| dir.path().join(format!("{name}.{ext}"));
    edit(&mut worker, &doc);
    let seen = poll_until(&mut worker, &doc, &Events::default(), HANG, modified);
    assert!(modified(&seen), "the edit marks it modified");
    doc.snapshot(&mut worker, &crate::file_url(&at("first", format)), format).expect("a whole snapshot");
    let before = flat(&mut worker, &doc, &at("before", flat_format));
    let waiter = worker.waiter();
    let shared = Arc::new(Mutex::new(worker));
    let mut held = shared.lock().expect("the worker lock");
    // A request waits before the snapshot starts.
    let waiting = {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || drop(waiter.hold(&shared)))
    };
    std::thread::sleep(Duration::from_millis(300));
    let stopped = doc.snapshot(&mut held, &crate::file_url(&at("stopped", format)), format);
    drop(held);
    waiting.join().expect("the waiting request");
    let error = stopped.expect_err("the waiting request stops the snapshot");
    assert_eq!(error.kind(), ErrorKind::Interrupted, "{error}");
    assert!(!at("stopped", format).exists(), "a stopped snapshot leaves no file");
    let mut worker = Arc::try_unwrap(shared).ok().expect("one owner").into_inner().expect("the lock");
    same(&flat(&mut worker, &doc, &at("after", flat_format)), &before, "the stopped snapshot changed the document");
    let later = poll_until(&mut worker, &doc, &seen, Duration::from_secs(1), |_| false);
    let unmarked = later.states.iter().any(|(name, state)| name == ".uno:ModifiedStatus" && state == "false");
    assert!(!unmarked, "the stopped snapshot marked it saved");
    doc.snapshot(&mut worker, &crate::file_url(&at("again", format)), format).expect("the next snapshot");
    let mut reread = |name: &str| {
        let back = worker.load(&crate::file_url(&at(name, format))).expect("the point opens");
        let text = flat(&mut worker, &back, &at(&format!("{name}-read"), flat_format));
        worker.close_doc(back).expect("closes");
        text
    };
    let (first, again) = (reread("first"), reread("again"));
    same(&again, &first, "the next snapshot is whole");
    (before, again)
}

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office").join(name)
}

#[test]
fn a_stopped_writer_snapshot_keeps_the_document_and_its_changes() {
    if install().is_none() {
        return eprintln!("SKIP: no pinned LOK build");
    }
    let (before, again) = stopped_then_whole(&fixture("redline.docx"), ("docx", "fodt"), |worker, doc| {
        type_in(worker, doc, "Typed ");
    });
    let changes = |text: &str| text.matches("<text:changed-region").count();
    assert!(changes(&before) >= 2, "the fixture's insertion and deletion: {}", changes(&before));
    assert_eq!(changes(&again), changes(&before), "the point keeps every change");
    assert!(again.contains("Typed"), "the point holds the edit");
}

/// Over Excel's 31 characters: the export shortens it.
const LONG: &str = "A sheet name of forty characters, really";

#[test]
fn a_stopped_calc_snapshot_keeps_long_sheet_names() {
    if install().is_none() {
        return eprintln!("SKIP: no pinned LOK build");
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let book = dir.path().join("long.fods");
    write(&book, &super::input::FODS.replace(r#"table:name="S""#, &format!(r#"table:name="{LONG}""#)));
    let (before, again) = stopped_then_whole(&book, ("xlsx", "fods"), |worker, doc| type_in(worker, doc, "7\n"));
    assert!(before.contains(LONG), "the name stays whole in the document");
    assert!(again.contains(&LONG[..31]), "the point holds the sheet");
}

#[test]
fn a_stopped_impress_snapshot_keeps_the_deck() {
    if install().is_none() {
        return eprintln!("SKIP: no pinned LOK build");
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let deck = dir.path().join("deck.fodp");
    write(&deck, &big_deck(20));
    let (before, again) = stopped_then_whole(&deck, ("pptx", "fodp"), |worker, doc| {
        worker.run(doc, "slide.hide", &[]).expect("hides a slide");
    });
    assert!(before.contains("Slide 19 line 15"), "the deck is whole");
    assert!(again.contains("Slide 19 line 15"), "the point holds every slide");
}
