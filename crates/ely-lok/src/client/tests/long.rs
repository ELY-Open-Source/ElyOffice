//! #111: a long Writer document loads before its layout ends.

use std::time::{Duration, Instant};

use super::*;

/// No resize this long: the idle layout has ended.
const SETTLED: Duration = Duration::from_secs(2);

/// Flowing text over hundreds of pages, no page breaks.
fn long_fodt() -> String {
    let line = "Ely lays out the first page now, the rest while idle. ".repeat(8);
    let body: String = (0..4000).map(|n| format!("<text:p>{n} {line}</text:p>")).collect();
    FODT.replace("<text:p>Hello 世界</text:p>", &body)
}

#[test]
fn a_long_document_loads_before_its_layout_ends() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("long.fodt");
    write(&path, &long_fodt());
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let doc = worker.load(&crate::file_url(&path)).expect("the document loads");
    let early = doc.pages(&mut worker).expect("pages at load").len();
    let deadline = Instant::now() + HANG;
    let mut seen = Events::default();
    let mut resized_at = Instant::now();
    while resized_at.elapsed() < SETTLED {
        assert!(Instant::now() < deadline, "the idle layout never settles");
        let batch = worker.poll_events(&doc, seen.seq, 250).expect("the poll returns");
        if batch.resized != seen.resized {
            resized_at = Instant::now();
        }
        seen = batch;
    }
    let laid = doc.pages(&mut worker).expect("pages once idle").len();
    assert!(laid > 100, "the idle layout reached {laid} pages");
    assert!(early * 10 < laid, "the load laid out {early} of {laid} pages");
}
