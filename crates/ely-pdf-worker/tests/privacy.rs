//! Worker records reach the app log; document text never.

use std::{
    path::Path,
    sync::{Mutex, OnceLock},
    time::Duration,
};

use ely_pdf_engine::{LinkChange, Target, Worker, sample::pdf};

/// Every record the app would log, kept for the test.
struct Kept;

fn kept() -> &'static Mutex<Vec<String>> {
    static KEPT: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    KEPT.get_or_init(Mutex::default)
}

impl log::Log for Kept {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        kept().lock().unwrap().push(format!("{} {} {}", record.level(), record.target(), record.args()));
    }

    fn flush(&self) {}
}

#[test]
fn a_broken_stream_keeps_its_text_out_of_the_log() {
    log::set_boxed_logger(Box::new(Kept)).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    // The worker logs at debug; the relay filters.
    unsafe { std::env::set_var("ELY_LOG", "debug") };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("broken.pdf");
    // An unknown operator: MuPDF warns, quoting it.
    let line = "ZqxPrivate42) Tj ZqxPrivate42 (tail";
    std::fs::write(&path, pdf(&[(612.0, 792.0, &[line])], &[])).unwrap();
    let mut worker = Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).unwrap();
    let opened = worker.open(&path).unwrap();
    let _ = worker.text(opened.doc, 0);
    // A refused address holds a credential, unquoted.
    let secret = Target::Web("ftp://ana:ZqxPrivate42@files.example/document".into());
    let at = worker.now(opened.doc).unwrap();
    assert!(worker.change_link(at, 0, &LinkChange::Add { rect: [72.0, 300.0, 200.0, 320.0], to: secret }).is_err());
    worker.close(opened.doc).unwrap();
    drop(worker);
    std::thread::sleep(Duration::from_millis(500));
    let records = kept().lock().unwrap().clone();
    assert!(records.iter().any(|r| r.contains("native output withheld")), "MuPDF spoke: {records:#?}");
    assert!(records.iter().any(|r| r.contains("ely_pdf_worker") || r.contains("pdf-worker")), "the worker's own records pass");
    let leaked: Vec<_> = records.iter().filter(|r| r.contains("ZqxPrivate42")).collect();
    assert!(leaked.is_empty(), "document text reached the log: {leaked:#?}");
}
