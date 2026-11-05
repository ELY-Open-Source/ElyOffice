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

/// Snapshots under four of these are too quick to time.
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
fn a_noted_wait_stops_a_snapshot_before_it_starts() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let deck = dir.path().join("deck.fodp");
    write(&deck, &big_deck(2));
    let doc = worker.load(&crate::file_url(&deck)).expect("the deck loads");
    let point = |name: &str| dir.path().join(format!("{name}.pptx"));
    let waiter = worker.waiter();
    // The pipe keeps order: the note lands first.
    waiter.note(true);
    let stopped = doc.snapshot(&mut worker, &crate::file_url(&point("stopped")), "pptx");
    waiter.note(false);
    let error = stopped.expect_err("a noted wait stops the snapshot");
    assert_eq!(error.kind(), ErrorKind::Interrupted, "{error}");
    assert!(!point("stopped").exists(), "a stopped snapshot leaves no file");
    doc.snapshot(&mut worker, &crate::file_url(&point("whole")), "pptx").expect("the wait ended: a whole one");
    waiter.note(true);
    let saved = doc.save_as(&mut worker, &crate::file_url(&point("saved")), "pptx");
    waiter.note(false);
    saved.expect("a save never gives way");
}

/// A measurement more than a proof: timing varies by machine.
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
    eprintln!("a key {:?} into a {whole:?} snapshot waited {waited:?}", whole / 4);
    // Unstopped, it would wait three quarters of `whole`.
    assert!(waited < whole / 2, "the key waited {waited:?} behind a {whole:?} snapshot");
    let error = stopped.expect_err("the key stopped the snapshot");
    assert_eq!(error.kind(), ErrorKind::Interrupted, "{error}");
}

/// The open document as flat XML, less its metadata.
fn flat(worker: &mut Worker, doc: &Doc, path: &Path) -> String {
    doc.save_as(worker, &crate::file_url(path), path.extension().and_then(|ext| ext.to_str()).expect("a format"))
        .expect("the flat copy saves");
    let text = std::fs::read_to_string(path).expect("the flat copy reads");
    // Each copy of an edited document dates itself anew.
    let (head, rest) = text.split_once("<office:meta>").expect("metadata");
    let (_, tail) = rest.split_once("</office:meta>").expect("the metadata's end");
    format!("{head}{tail}")
}

/// A point as flat XML, less what loads differ in.
fn reread(worker: &mut Worker, point: &Path, path: &Path) -> String {
    let back = worker.load(&crate::file_url(point)).expect("the point opens");
    let text = flat(worker, &back, path);
    worker.close_doc(back).expect("closes");
    comparable(&text)
}

/// `text` less what two loads of one point differ in.
fn comparable(text: &str) -> String {
    // WindowState, rsids, change ids, Calc's style order.
    let drawn = ["WindowState", "Rsid", "RsidRoot"].map(|name| format!(r#"config:name="{name}""#));
    let kept = |line: &&str| !drawn.iter().any(|name| line.contains(name.as_str()));
    let mut lines: Vec<String> = layouts_named(text).lines().filter(kept).map(unnumbered).collect();
    lines.sort_unstable();
    lines.join("\n")
}

/// `text` with each page layout's name its properties.
fn layouts_named(text: &str) -> String {
    const OPEN: &str = r#"<style:page-layout style:name=""#;
    let (mut named, mut rest) = (Vec::new(), text);
    while let Some(at) = rest.find(OPEN) {
        let block = &rest[at..];
        let end = block.find("</style:page-layout>").expect("a page layout's end");
        let name = &block[OPEN.len()..OPEN.len() + block[OPEN.len()..].find('"').expect("a name's end")];
        let body = block[..end].replacen(&format!(r#"style:name="{name}""#), "", 1);
        named.push((format!("\"{name}\""), format!("\"{}\"", body.split_whitespace().collect::<Vec<_>>().join(" "))));
        rest = &block[end..];
    }
    named.iter().fold(text.to_string(), |text, (name, body)| text.replace(name, body))
}

/// `line` less Writer's change ids.
fn unnumbered(line: &str) -> String {
    let (mut out, mut rest) = (String::new(), line);
    while let Some(at) = rest.find("\"ct") {
        out.push_str(&rest[..at + 3]);
        rest = rest[at + 3..].trim_start_matches(|c: char| c.is_ascii_digit());
    }
    out + rest
}

/// Two page layouts and two master pages using them.
const LAYOUTS: &str = r#"<style:page-layout style:name="pm1">
<style:page-layout-properties fo:page-width="8.5in" style:first-page-number="continue"/>
</style:page-layout>
<style:page-layout style:name="pm2">
<style:page-layout-properties fo:page-width="8.5in" style:first-page-number="1"/>
</style:page-layout>
<style:master-page style:name="A" style:page-layout-name="pm1"/>
<style:master-page style:name="B" style:page-layout-name="pm2"/>"#;

#[test]
fn page_layouts_compare_by_what_they_are() {
    let swapped = LAYOUTS.replace(r#""pm1""#, r#""x""#).replace(r#""pm2""#, r#""pm1""#).replace(r#""x""#, r#""pm2""#);
    assert_eq!(comparable(&swapped), comparable(LAYOUTS), "renumbered layouts are the same");
    let moved = LAYOUTS.replace(r#""B" style:page-layout-name="pm2""#, r#""B" style:page-layout-name="pm1""#);
    assert_ne!(comparable(&moved), comparable(LAYOUTS), "a page on another layout differs");
    let changed = LAYOUTS.replace(r#"first-page-number="1""#, r#"first-page-number="2""#);
    assert_ne!(comparable(&changed), comparable(LAYOUTS), "a changed layout differs");
}

/// Equal texts, or the first lines that differ.
fn same(left: &str, right: &str, what: &str) {
    let differ = left.lines().zip(right.lines()).position(|(a, b)| a != b);
    let near = |text: &str, at: usize| text.lines().skip(at).take(3).collect::<Vec<_>>().join("\n");
    match differ {
        Some(at) => panic!("{what}: line {at}\n{}\n---\n{}", near(left, at), near(right, at)),
        None => assert_eq!(left.lines().count(), right.lines().count(), "{what}: lengths"),
    }
}

/// The edited `file`: stopped at input check `from`, then whole.
fn stopped_then_whole(file: &Path, (format, flat_format): (&str, &str), from: u64, edit: impl FnOnce(&mut Worker, &Doc)) -> (String, String) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn_dev(&install().expect("LOK"), &dir.path().join("profile")).expect("the worker");
    let doc = worker.load(&crate::file_url(file)).expect("the file loads");
    let at = |name: &str, ext: &str| dir.path().join(format!("{name}.{ext}"));
    edit(&mut worker, &doc);
    let seen = poll_until(&mut worker, &doc, &Events::default(), HANG, modified);
    assert!(modified(&seen), "the edit marks it modified");
    doc.snapshot(&mut worker, &crate::file_url(&at("first", format)), format).expect("a whole snapshot");
    let before = flat(&mut worker, &doc, &at("before", flat_format));
    // Check 1 precedes the export; `from` is inside it.
    let stopped = doc.snapshot_probed(&mut worker, &crate::file_url(&at("stopped", format)), format, from);
    let error = stopped.expect_err("the export reached the probed check");
    assert_eq!(error.kind(), ErrorKind::Interrupted, "{error}");
    assert!(!at("stopped", format).exists(), "a stopped snapshot leaves no file");
    same(&flat(&mut worker, &doc, &at("after", flat_format)), &before, "the stopped snapshot changed the document");
    let later = poll_until(&mut worker, &doc, &seen, Duration::from_secs(1), |_| false);
    let unmarked = later.states.iter().any(|(name, state)| name == ".uno:ModifiedStatus" && state == "false");
    assert!(!unmarked, "the stopped snapshot marked it saved");
    doc.snapshot(&mut worker, &crate::file_url(&at("again", format)), format).expect("the next snapshot");
    let first = reread(&mut worker, &at("first", format), &at("first-read", flat_format));
    let again = reread(&mut worker, &at("again", format), &at("again-read", flat_format));
    same(&again, &first, "the next snapshot is whole");
    (before, again)
}

/// Sections with their own header parts; two tracked changes.
fn big_text(sections: usize) -> String {
    let info = "<office:change-info><dc:creator>A</dc:creator><dc:date>2026-01-01T00:00:00</dc:date></office:change-info>";
    let masters: String = (0..sections)
        .map(|n| format!(r#"<style:master-page style:name="M{n}" style:page-layout-name="L"><style:header><text:p>Header {n}</text:p></style:header></style:master-page>"#))
        .collect();
    let styles: String = (0..sections).map(|n| format!(r#"<style:style style:name="P{n}" style:family="paragraph" style:master-page-name="M{n}"/>"#)).collect();
    let section = |n: usize| {
        let body: String = (0..300).map(|p| format!("<text:p>Paragraph {p} of section {n} keeps it long.</text:p>")).collect();
        format!(r#"<text:p text:style-name="P{n}">Section {n}</text:p>{body}"#)
    };
    let body: String = (0..sections).map(section).collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:dc="http://purl.org/dc/elements/1.1/"
 xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:automatic-styles><style:page-layout style:name="L"/>{styles}</office:automatic-styles>
 <office:master-styles>{masters}</office:master-styles>
 <office:body><office:text><text:tracked-changes text:track-changes="false">
 <text:changed-region text:id="ct1"><text:insertion>{info}</text:insertion></text:changed-region>
 <text:changed-region text:id="ct2"><text:deletion>{info}<text:p>gone</text:p></text:deletion></text:changed-region>
 </text:tracked-changes><text:p>Kept <text:change-start text:change-id="ct1"/>new<text:change-end text:change-id="ct1"/> and <text:change text:change-id="ct2"/>cut.</text:p>{body}</office:text></office:body>
</office:document>"#
    )
}

/// `sheets` sheets, one part each; the first is `name`.
fn big_book(name: &str, sheets: usize) -> String {
    let cell = |n: usize| format!(r#"<table:table-cell office:value-type="float" office:value="{n}"/>"#);
    let row = |r: usize| format!("<table:table-row>{}</table:table-row>", (0..10).map(|c| cell(r * 10 + c)).collect::<String>());
    let rows: String = (0..400).map(row).collect();
    let sheet = |n: usize| {
        let named = if n == 0 { name.to_string() } else { format!("S{n}") };
        format!(r#"<table:table table:name="{named}">{rows}</table:table>"#)
    };
    let sheets: String = (0..sheets).map(sheet).collect();
    let (head, rest) = super::input::FODS.split_once("<office:spreadsheet>").expect("the book's body");
    let (_, tail) = rest.split_once("</office:spreadsheet>").expect("the body's end");
    format!("{head}<office:spreadsheet>{sheets}</office:spreadsheet>{tail}")
}

#[test]
fn a_stopped_writer_snapshot_keeps_the_document_and_its_changes() {
    if install().is_none() {
        return eprintln!("SKIP: no pinned LOK build");
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let text = dir.path().join("long.fodt");
    write(&text, &big_text(200));
    let (before, again) = stopped_then_whole(&text, ("docx", "fodt"), WRITER_CHECK, |worker, doc| type_in(worker, doc, "Typed "));
    let changes = |text: &str| text.matches("<text:changed-region").count();
    assert_eq!(changes(&before), 2, "the insertion and the deletion");
    assert_eq!(changes(&again), changes(&before), "the point keeps every change");
    assert!(again.contains("Typed"), "the point holds the edit");
}

/// About half of each export's input checks (407, 106, 409).
const WRITER_CHECK: u64 = 200;
const CALC_CHECK: u64 = 50;
const IMPRESS_CHECK: u64 = 200;

/// Over Excel's 31 characters: the export shortens it.
const LONG: &str = "A sheet name of forty characters, really";

#[test]
fn a_stopped_calc_snapshot_keeps_long_sheet_names() {
    if install().is_none() {
        return eprintln!("SKIP: no pinned LOK build");
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let book = dir.path().join("long.fods");
    write(&book, &big_book(LONG, 100));
    let (before, again) = stopped_then_whole(&book, ("xlsx", "fods"), CALC_CHECK, |worker, doc| type_in(worker, doc, "7\n"));
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
    write(&deck, &big_deck(400));
    let (before, again) = stopped_then_whole(&deck, ("pptx", "fodp"), IMPRESS_CHECK, |worker, doc| {
        worker.run(doc, "slide.hide", &[]).expect("hides a slide");
    });
    assert!(before.contains("Slide 399 line 15"), "the deck is whole");
    assert!(again.contains("Slide 399 line 15"), "the point holds every slide");
}

#[test]
fn a_worker_without_dev_refuses_the_stop_probe() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let deck = dir.path().join("deck.fodp");
    write(&deck, &big_deck(2));
    let doc = worker.load(&crate::file_url(&deck)).expect("the deck loads");
    let point = dir.path().join("probed.pptx");
    let refused = doc.snapshot_probed(&mut worker, &crate::file_url(&point), "pptx", 1);
    let error = refused.expect_err("only a dev worker takes the probe");
    assert!(error.to_string().contains("dev-only"), "{error}");
    assert!(!point.exists(), "a refused probe writes nothing");
}
