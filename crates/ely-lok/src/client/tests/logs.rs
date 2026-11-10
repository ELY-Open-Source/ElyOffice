//! Kernel patch logs reach the app's log (R15.64).

use std::sync::{Mutex, Once};

use super::analysis::{cells, skip};
use super::linked::{rates, sheets, update};
use super::ole_links::worksheet;
use super::sheet::open_book;
use super::*;

/// What the workers' relays logged: level and message.
static HEARD: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct Heard;

impl log::Log for Heard {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.target() == "lok-worker"
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
