use super::*;

/// Queued background work lets a view's input go after one.
#[test]
fn input_waits_for_one_background_request_not_all() {
    use std::{sync::atomic::{AtomicUsize, Ordering}, time::Duration};
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let shared = Arc::new(Mutex::new(crate::Worker::spawn(&install, &dir.path().join("profile")).expect("the worker")));
    let ran = Arc::new(AtomicUsize::new(0));
    let held = shared.lock().expect("the worker lock");
    let queued: Vec<_> = (0..12)
        .map(|_| {
            let (shared, ran) = (Arc::clone(&shared), Arc::clone(&ran));
            std::thread::spawn(move || {
                futures::executor::block_on(crate::behind(&shared, |_| {
                    std::thread::sleep(Duration::from_millis(20));
                    ran.fetch_add(1, Ordering::SeqCst);
                }))
            })
        })
        .collect();
    std::thread::sleep(Duration::from_millis(300));
    // The lane locks the worker itself, as here.
    let input = {
        let (shared, ran) = (Arc::clone(&shared), Arc::clone(&ran));
        std::thread::spawn(move || {
            let _engine = shared.lock().expect("the worker lock");
            ran.load(Ordering::SeqCst)
        })
    };
    std::thread::sleep(Duration::from_millis(300));
    drop(held);
    let before = input.join().expect("the input lands");
    queued.into_iter().for_each(|read| read.join().expect("each read lands"));
    assert_eq!(ran.load(Ordering::SeqCst), 12, "every read ran");
    assert!(before <= 2, "the input waited for {before} of 12 background reads");
}

/// Work waiting for its turn holds no thread.
#[test]
fn waiting_for_the_turn_parks_no_thread() {
    use futures::task::LocalSpawnExt as _;
    use std::{sync::mpsc::channel, time::Duration};
    let worker = Arc::new(Mutex::new(()));
    let (held, release) = (channel(), channel::<()>());
    let holder = {
        let worker = Arc::clone(&worker);
        std::thread::spawn(move || {
            futures::executor::block_on(crate::behind(&worker, |_| {
                held.0.send(()).expect("the test waits");
                release.1.recv().expect("the test lets go");
            }))
        })
    };
    held.1.recv().expect("the turn is held");
    let (ran, seen) = channel();
    // One thread: a blocked wait would stop everything on it.
    let pool = std::thread::spawn(move || {
        let mut pool = futures::executor::LocalPool::new();
        let spawner = pool.spawner();
        let waiting: Vec<_> = (0..8)
            .map(|_| {
                let worker = Arc::clone(&worker);
                spawner.spawn_local_with_handle(async move { crate::behind(&worker, |_| ()).await }).expect("spawns")
            })
            .collect();
        spawner.spawn_local(async move { ran.send(()).expect("the test waits") }).expect("spawns");
        pool.run_until(futures::future::join_all(waiting));
    });
    let free = seen.recv_timeout(Duration::from_secs(2)).is_ok();
    release.0.send(()).expect("the holder lets go");
    holder.join().expect("the holder ends");
    pool.join().expect("every waiter ran");
    assert!(free, "a task waited behind work queued for the turn");
}

/// One worker's turn never holds back another's.
#[test]
fn each_worker_has_its_own_turn() {
    use std::{sync::mpsc::channel, time::Duration};
    let (one, other) = (Arc::new(Mutex::new(())), Arc::new(Mutex::new(())));
    let (held, release) = (channel(), channel::<()>());
    let holder = std::thread::spawn(move || {
        futures::executor::block_on(crate::behind(&one, |_| {
            held.0.send(()).expect("the test waits");
            release.1.recv().expect("the test lets go");
        }))
    });
    held.1.recv().expect("the turn is held");
    let (ran, seen) = channel();
    let waiter = std::thread::spawn(move || futures::executor::block_on(crate::behind(&other, |_| ran.send(()).expect("the test waits"))));
    let free = seen.recv_timeout(Duration::from_secs(2)).is_ok();
    release.0.send(()).expect("the holder lets go");
    holder.join().expect("the holder ends");
    waiter.join().expect("the other worker's work ends");
    assert!(free, "a worker waited for another worker's turn");
}

/// A busy engine never stalls the window opening a view.
#[test]
fn a_view_opens_while_the_engine_is_busy() {
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = crate::Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("note.fodt");
    std::fs::write(&source, crate::client::tests::FODT).expect("the fixture writes");
    let doc = worker.load(&crate::file_url(&source)).expect("the document loads");
    let laid = super::super::layout(&mut worker, &doc).expect("the layout reads");
    let feed = worker.feed();
    let shared = Arc::new(Mutex::new(worker));
    let (held, release) = (std::sync::mpsc::channel(), std::sync::mpsc::channel::<()>());
    let holder = {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || {
            let _engine = shared.lock().expect("the worker lock");
            held.0.send(()).expect("the test waits");
            let _ = release.1.recv_timeout(std::time::Duration::from_secs(5));
        })
    };
    held.1.recv().expect("the engine is held");
    let began = std::time::Instant::now();
    let cx = gpui::TestAppContext::single();
    cx.update(|app| drop(LokView::open(Arc::clone(&shared), doc, laid, feed, app)));
    let took = began.elapsed();
    release.0.send(()).expect("the holder lets go");
    holder.join().expect("the holder ends");
    assert!(took < std::time::Duration::from_secs(1), "opening waited {took:?} for the engine");
}

/// Queued keys share one hold of the worker.
#[test]
fn queued_keys_share_one_hold_of_the_worker() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let cx = gpui::TestAppContext::single();
    let (view, _) = cx.update(|app| live_view(app, &install));
    let settle = || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !cx.update(|app| view.read(app).settled()) {
            assert!(std::time::Instant::now() < deadline, "the input never landed");
            cx.executor().tick();
        }
    };
    settle();
    let (worker, doc) = cx.update(|app| (Arc::clone(&view.read(app).worker), view.read(app).doc.clone()));
    let rev = || worker.lock().expect("the worker lock").now(&doc).expect("the state").rev;
    let start = rev();
    let stop = Arc::new(AtomicBool::new(false));
    // Background work, always waiting for the worker.
    let others: Vec<_> = (0..4)
        .map(|_| {
            let (worker, doc, stop) = (Arc::clone(&worker), doc.clone(), Arc::clone(&stop));
            std::thread::spawn(move || {
                let mut seen = Vec::new();
                while !stop.load(Ordering::SeqCst) {
                    seen.push(worker.lock().expect("the worker lock").now(&doc).expect("the state").rev);
                }
                seen
            })
        })
        .collect();
    cx.update(|app| view.update(app, |view, _| (0..6).for_each(|_| view.commit(None, "x"))));
    settle();
    stop.store(true, Ordering::SeqCst);
    let end = rev();
    let seen: Vec<u64> = others.into_iter().flat_map(|other| other.join().expect("the reader ends")).collect();
    assert!(end > start, "the keys edited");
    let between: Vec<_> = seen.iter().filter(|at| **at > start && **at < end).collect();
    assert!(between.is_empty(), "other work ran between queued keys at {between:?}, from {start} to {end}");
}
