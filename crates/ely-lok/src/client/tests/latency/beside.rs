use super::super::input::FODS;
use super::super::*;

/// A key waits out another window's load (W.1b).
#[test]
#[ignore = "a measurement: run with --ignored --nocapture"]
fn a_key_waits_while_another_window_loads() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let big = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../engines/bench/sheet.xlsx");
    if !big.exists() {
        return eprintln!("SKIP: no {}; run tests/perf/samples.py", big.display());
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("in.fods");
    write(&source, FODS);
    let doc = worker.load(&crate::file_url(&source)).expect("the small sheet");
    let shared = std::sync::Arc::new(std::sync::Mutex::new(worker));
    let key = |shared: &std::sync::Mutex<Worker>| {
        let start = std::time::Instant::now();
        shared.lock().expect("the worker").post_key(&doc, 'a' as u32, 0).expect("a key");
        start.elapsed().as_secs_f64() * 1000.0
    };
    let idle = key(&shared);
    let (held, holding) = std::sync::mpsc::channel();
    let loader = {
        let (shared, url) = (shared.clone(), crate::file_url(&big));
        std::thread::spawn(move || {
            let mut worker = shared.lock().expect("the worker");
            held.send(()).expect("told");
            let start = std::time::Instant::now();
            worker.load(&url).expect("the big sheet");
            start.elapsed().as_secs_f64() * 1000.0
        })
    };
    holding.recv().expect("the load holds the engine");
    let waited = key(&shared);
    let load = loader.join().expect("loaded");
    println!("beside: key idle {idle:.1} ms, during the other load {waited:.1} ms, load {load:.1} ms");
    assert!(waited >= load * 0.5, "the key should wait out the load: {waited:.1} of {load:.1} ms");
}
