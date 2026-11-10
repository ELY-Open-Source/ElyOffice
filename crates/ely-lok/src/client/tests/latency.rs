use super::input::FODS;
use super::*;

/// Keys never queue behind a view's event watch.
#[test]
fn keys_never_wait_behind_the_event_watch() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).unwrap();
    let source = dir.path().join("in.fods");
    write(&source, FODS);
    let doc = worker
        .load(&crate::file_url(&source))
        .unwrap();
    let feed = worker.feed();
    let shared = std::sync::Arc::new(std::sync::Mutex::new(worker));
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let poller = {
        let (feed, doc, stop) = (feed.clone(), doc.clone(), stop.clone());
        std::thread::spawn(move || {
            let mut known = 0;
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                known = feed.wait(&doc, known, 50).unwrap().seq;
            }
        })
    };
    let mut waits = Vec::new();
    let mut calls = Vec::new();
    for c in "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz".chars() {
        let start = std::time::Instant::now();
        let mut worker = shared.lock().unwrap();
        let locked = start.elapsed();
        worker.post_key(&doc, c as u32, 0).unwrap();
        drop(worker);
        waits.push(locked.as_secs_f64() * 1000.0);
        calls.push((start.elapsed() - locked).as_secs_f64() * 1000.0);
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    poller.join().unwrap();
    let stat = |v: &mut Vec<f64>| {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        (v[v.len() / 2], v[v.len() * 95 / 100], v[v.len() - 1])
    };
    let (waited, called) = (stat(&mut waits), stat(&mut calls));
    log::info!("keys: lock wait {waited:?} ms, call {called:?} ms");
    assert!(waited.1 < 2.0, "keys wait for the lock: {waited:?} ms");
    // Queued behind the 50 ms watch, every key would be slow.
    assert!(called.0 < 30.0, "a key round trip is slow: {called:?} ms");
}
