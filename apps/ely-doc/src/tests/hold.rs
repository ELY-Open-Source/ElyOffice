//! The worker lock held until the test lets it go.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

/// Past this the holder lets go: something waited on it.
const STUCK: Duration = Duration::from_secs(10);

pub(crate) struct Held {
    release: mpsc::Sender<()>,
    gave_up: Arc<AtomicBool>,
    holder: JoinHandle<()>,
}

/// Takes `worker`'s lock on another thread; returns once held.
pub(crate) fn hold<W: Send + 'static>(worker: &Arc<Mutex<W>>) -> Held {
    let (held, holding) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    let gave_up = Arc::new(AtomicBool::new(false));
    let (worker, stuck) = (Arc::clone(worker), Arc::clone(&gave_up));
    let holder = std::thread::spawn(move || {
        let _guard = worker.lock().expect("the worker lock");
        held.send(()).expect("the test waits");
        if released.recv_timeout(STUCK).is_err() {
            stuck.store(true, Ordering::SeqCst);
        }
    });
    holding.recv().expect("the lock is held");
    Held { release, gave_up, holder }
}

impl Held {
    /// Lets go; fails when `what` outwaited the hold.
    #[track_caller]
    pub(crate) fn release(self, what: &str) {
        let waited = self.gave_up.load(Ordering::SeqCst);
        // Sent before the check fails, so the holder ends.
        let _ = self.release.send(());
        self.holder.join().expect("the holder ends");
        assert!(!waited, "{what} waited for the worker lock");
    }
}
