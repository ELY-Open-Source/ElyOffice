//! Kernel patch logs reach the app's log (R15.64).

use std::sync::{Mutex, Once};

use super::analysis::{cells, skip};
use super::fallback::{chinese, ASKED, KINDS, WHY};
use super::linked::{rates, sheets, update};
use super::ole_links::worksheet;
use super::sheet::open_book;
use super::show::DECK;
use super::*;

mod printers;

/// What the workers' relays logged: level and message.
static HEARD: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct Heard;

impl log::Log for Heard {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        matches!(metadata.target(), "lok-worker" | "ely_lok::worker::stages")
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            HEARD.lock().expect("the heard log").push(format!("{} {}", record.level(), record.args()));
        }
    }

    fn flush(&self) {}
}

/// The logged line that holds every one of `parts`.
fn heard(parts: &[&str]) -> Option<String> {
    let deadline = std::time::Instant::now() + HANG;
    loop {
        let found = HEARD.lock().expect("the heard log").iter().find(|line| parts.iter().all(|part| line.contains(part))).cloned();
        if found.is_some() || std::time::Instant::now() > deadline {
            return found;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn listen() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        log::set_logger(&Heard).expect("no other logger in these tests");
        log::set_max_level(log::LevelFilter::Info);
    });
}

#[test]
fn a_source_read_and_a_failed_read_are_both_logged() {
    if skip() {
        return;
    }
    listen();
    let dir = tempfile::tempdir().expect("a temp dir");
    let install = install().expect("the pinned LOK build");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("logged-source.xlsx");
    worksheet(dir.path(), &mut worker, &source, 1);
    let flat = dir.path().join("reader.fods");
    rates(&flat, 0);
    let reader = worker.load(&crate::file_url(&flat)).expect("loads");
    let formula = format!("='{}/[logged-source.xlsx]S1'!A1", dir.path().display());
    worker.run(&reader, "cell.goto", &["H1"]).expect("H1");
    worker.run(&reader, "cell.enter", &[&formula]).expect("the link");
    assert_eq!(cells(&mut worker, &reader, "H1"), "1");
    let folder = dir.path().file_name().and_then(|name| name.to_str()).expect("a folder name");
    let read = heard(&["INFO sc.ui (externalrefmgr.cxx:", "external source read into memory", folder, "logged-source.xlsx", " bytes"]);
    assert!(read.is_some(), "the read is not in the app's log");
    // A linked sheet whose file is gone.
    let missing = crate::file_url(&dir.path().join("logged-missing.ods"));
    let sheet = format!(r#"<table:table-source xlink:type="simple" xlink:href="{missing}" table:filter-name="calc8" table:table-name="S1" table:mode="copy-all"/>"#);
    let (_book_dir, mut other, book) = open_book(&sheets(&sheet, r#"<table:table-cell office:value-type="float" office:value="5"/>"#));
    assert_eq!(update(&mut other, &book), [missing], "the missing source is named");
    let failed = heard(&["WARN sc.ui (docsh4.cxx:", "linked source not read", folder, "logged-missing.ods"]);
    assert!(failed.is_some(), "the failed read is not in the app's log");
}

#[test]
fn a_worker_logs_each_start_stage_with_its_counters() {
    if skip() {
        return;
    }
    listen();
    let dir = tempfile::tempdir().expect("a temp dir");
    let install = install().expect("the pinned LOK build");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let flat = dir.path().join("first.fods");
    rates(&flat, 0);
    worker.load(&crate::file_url(&flat)).expect("loads");
    for stage in ["kernel hooked", "self-check loaded", "self-check painted", "first document loaded"] {
        let said = heard(&["INFO lok-worker: start ", stage, " ms, "]);
        let said = said.unwrap_or_else(|| panic!("no start line for {stage}"));
        assert!(!said.contains("counters"), "{stage} has no counts: {said}");
        assert!(!said.contains(&*dir.path().to_string_lossy()), "{stage} names a path: {said}");
    }
}

/// A class name no document of ours holds elsewhere.
const CLASS: &str = "ElyPrivateOleClass.1";

#[test]
fn a_linked_object_s_class_name_stays_out_of_the_log() {
    if skip() {
        return;
    }
    listen();
    let dir = tempfile::tempdir().expect("a temp dir");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office/ole-link-other.docx");
    let renamed = dir.path().join("renamed.docx");
    // The link's ProgID is text from the file.
    let script = "import re, sys, zipfile\nsource, target, name = sys.argv[1:4]\nwith zipfile.ZipFile(source) as before, zipfile.ZipFile(target, 'w', zipfile.ZIP_DEFLATED) as after:\n    \
                  for entry in before.infolist():\n        data = before.read(entry.filename)\n        \
                  if entry.filename == 'word/document.xml':\n            text, count = re.subn(r'ProgID=\"[^\"]*\"', 'ProgID=\"' + name + '\"', data.decode())\n            \
                  assert count == 1, count\n            data = text.encode()\n        after.writestr(entry, data)";
    let made = std::process::Command::new("python3").args(["-I", "-c", script]).arg(&fixture).arg(&renamed).arg(CLASS).output().expect("python3 runs");
    assert!(made.status.success(), "{}", String::from_utf8_lossy(&made.stderr));
    let mut worker = Worker::spawn(&install().expect("the pinned LOK build"), &dir.path().join("profile")).expect("the worker");
    let doc = worker.load(&crate::file_url(&renamed)).expect("loads");
    worker.close_doc(doc).expect("closes");
    // Every line the worker wrote is in by now.
    worker.shutdown().expect("the worker ends");
    // Its own lines: other tests log the same warning.
    let said = worker.stderr.said().lines;
    let warned = said.iter().any(|line| line.starts_with("warn:ely.writerfilter:") && line.ends_with("stays a picture"));
    assert!(warned, "this worker did not warn of the unlisted class: {said:?}");
    assert!(!said.iter().any(|line| line.contains(CLASS)), "the worker wrote the class name: {said:?}");
    let leaked: Vec<String> = HEARD.lock().expect("the heard log").iter().filter(|line| line.contains(CLASS)).cloned().collect();
    assert!(leaked.is_empty(), "text from the document reached the app's log: {leaked:?}");
}

/// SF.18: copies a zip, swapping one text per part.
fn edited(source: &std::path::Path, target: &std::path::Path, swaps: &[(&str, &str, &str)], added: Option<(&str, &str)>) {
    let script = r#"import json, sys, zipfile
source, target, swaps, added = sys.argv[1], sys.argv[2], json.loads(sys.argv[3]), json.loads(sys.argv[4])
left = {name: (old, new) for name, old, new in swaps}
with zipfile.ZipFile(source) as before, zipfile.ZipFile(target, 'w', zipfile.ZIP_DEFLATED) as after:
    for entry in before.infolist():
        data = before.read(entry.filename)
        if entry.filename in left:
            old, new = left.pop(entry.filename)
            assert data.count(old.encode()) == 1, entry.filename
            data = data.replace(old.encode(), new.encode())
        after.writestr(entry, data)
    assert not left, left
    if added:
        after.writestr(added[0], added[1])"#;
    let swaps = serde_json::to_string(swaps).expect("json");
    let added = serde_json::to_string(&added).expect("json");
    let made = std::process::Command::new("python3").args(["-I", "-c", script]).arg(source).arg(target).arg(swaps).arg(added).output().expect("python3 runs");
    assert!(made.status.success(), "{}", String::from_utf8_lossy(&made.stderr));
}

/// Why loading `path` made a new worker ask the spooler.
fn asked(dir: &std::path::Path, path: &std::path::Path) -> Option<String> {
    let mut worker = Worker::spawn(&install().expect("the pinned LOK build"), &dir.join("profile")).expect("the worker");
    let doc = worker.load(&crate::file_url(path)).expect("loads");
    worker.close_doc(doc).expect("closes");
    // Every line the worker wrote is in by now.
    worker.shutdown().expect("the worker ends");
    let said = worker.stderr.said().lines;
    let why = said.iter().find_map(|line| line.strip_prefix("info:ely.vcl.print:")?.strip_suffix("; asking the spooler"));
    why.map(|why| why.rsplit(':').next().expect("a reason").trim().to_owned())
}

#[test]
fn only_documents_needing_a_printer_ask_the_spooler() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let word = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office/word.docx");
    let metrics = dir.path().join("metrics.docx");
    let settings = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:compat><w:usePrinterMetrics/></w:compat></w:settings>"#;
    edited(&word, &metrics, &[
        ("[Content_Types].xml", "</Types>", r#"<Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/></Types>"#),
        ("word/_rels/document.xml.rels", "</Relationships>", r#"<Relationship Id="rIdEly1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/></Relationships>"#),
    ], Some(("word/settings.xml", settings)));
    // Flat decks drop settings; zipped ones keep them.
    let (flat, deck) = (dir.path().join("deck.fodp"), dir.path().join("plain.odp"));
    let (metrics_deck, named_deck) = (dir.path().join("metrics.odp"), dir.path().join("named.odp"));
    std::fs::write(&flat, DECK).expect("writes");
    let mut worker = Worker::spawn(&install().expect("the pinned LOK build"), &dir.path().join("saver")).expect("the worker");
    let doc = worker.load(&crate::file_url(&flat)).expect("loads");
    doc.save_as(&mut worker, &crate::file_url(&deck), "odp").expect("saves");
    worker.close_doc(doc).expect("closes");
    let layout = r#"PrinterIndependentLayout" config:type="string">"#;
    edited(&deck, &metrics_deck, &[("settings.xml", &format!("{layout}low-resolution"), &format!("{layout}disabled"))], None);
    let name = r#"<config:config-item config:name="PrinterName" config:type="string">Ely Absent Printer</config:config-item>"#;
    edited(&deck, &named_deck, &[("settings.xml", printers::UNNAMED, name)], None);
    let (metrics_why, named_why) = (Some("a document lays out by printer metrics"), Some("a document names a printer"));
    for (path, why) in [(&word, None), (&deck, None), (&metrics, metrics_why), (&metrics_deck, metrics_why), (&named_deck, named_why)] {
        assert_eq!(asked(dir.path(), path).as_deref(), why, "{} asked the spooler", path.display());
    }
}

#[test]
fn a_refused_animation_logs_why() {
    if skip() {
        return;
    }
    listen();
    let dir = tempfile::tempdir().expect("a temp dir");
    let flat = dir.path().join("deck.fodp");
    std::fs::write(&flat, DECK).expect("writes");
    let mut worker = Worker::spawn(&install().expect("the pinned LOK build"), &dir.path().join("profile")).expect("the worker");
    let doc = worker.load(&crate::file_url(&flat)).expect("loads");
    let add = r#"{"preset":"ooo-entrance-fade-in"}"#;
    assert!(worker.run(&doc, "animation.add", &[add]).is_err(), "nothing is marked");
    let why = heard(&["WARN sd.view (drviewskit.cxx:", "AddAnimation refused", "preset known", "marked 0"]);
    assert!(why.is_some(), "the refusal names its reason");
    worker.close_doc(doc).expect("closes");
}

// Linux's fontconfig answers every font name itself.
#[cfg(any(windows, target_os = "macos"))]
#[test]
fn the_start_check_reads_every_font_s_other_names() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let read: Vec<_> = [crate::Probe::Text, crate::Probe::Sheet, crate::Probe::Slides].into_iter().map(|probe| {
        let dir = tempfile::tempdir().expect("a temp dir");
        let mut worker = Worker::launch(&install, &dir.path().join("profile"), probe).expect("the worker");
        worker.ready().unwrap_or_else(|error| panic!("{probe:?} paints no red: {error}"));
        worker.shutdown().expect("the worker ends");
        (probe, worker.stderr.said().lines.iter().any(|line| line.contains("other names of")))
    }).collect();
    // Else a hot open pays it in its first tile.
    assert!(read.iter().all(|(_, read)| *read), "{read:?}");
}

#[test]
fn a_missing_font_logs_what_drew_it_and_why() {
    if skip() {
        return;
    }
    listen();
    let (_dir, _worker, _doc) = super::input::open(&chinese());
    for ((asked, kind), why) in ASKED.iter().zip(KINDS).zip(WHY) {
        let line = heard(&["vcl.fonts", &format!("font \"{asked}\" missing: took \"")]).unwrap_or_else(|| panic!("{asked}'s stand-in is not logged"));
        let (took, reason) = line.split_once(" missing: took \"").and_then(|(_, rest)| rest.split_once("\" (")).expect("the line's shape");
        assert!(kind.contains(&took), "{asked} logged {took:?}, not one of {kind:?}");
        assert_eq!(reason, format!("{why})"), "{asked}: why {took} was taken");
    }
}
