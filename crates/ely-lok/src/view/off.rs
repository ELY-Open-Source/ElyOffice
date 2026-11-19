//! Kernel reads off the UI thread.

use std::{
    any::Any,
    panic::Location,
    io,
    sync::{
        Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use gpui::Context;

use crate::{Doc, Worker};

/// Undrawn work holds background work this long at most.
const HELD_MOST: Duration = Duration::from_secs(1);

/// Background work longer than this is logged.
const HELD_LONG: Duration = Duration::from_millis(100);

/// One worker's background queue and the edits it yields to.
#[derive(Default)]
struct Turn {
    queue: futures::lock::Mutex<()>,
    /// Each holding view and its newest undrawn edit's send time.
    holds: Mutex<Vec<(u64, Instant)>>,
    /// Inputs and tiles waiting for the worker lock.
    fronts: AtomicUsize,
    clear: Condvar,
}

impl Turn {
    /// Plain data: a panic leaves it whole.
    fn holds(&self) -> MutexGuard<'_, Vec<(u64, Instant)>> {
        self.holds.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Waits while undrawn work is under a second old.
    fn yield_to_edits(&self) {
        let began = Instant::now();
        let mut holds = self.holds();
        loop {
            let now = Instant::now();
            let until = holds.iter().map(|(_, since)| *since + HELD_MOST).filter(|until| *until > now).max();
            let Some(until) = until else { break };
            holds = self.clear.wait_timeout(holds, until - now).unwrap_or_else(PoisonError::into_inner).0;
        }
        let waited = began.elapsed();
        if !holds.is_empty() && !waited.is_zero() {
            log::warn!("lok: background work held {} ms by undrawn work; it goes", waited.as_millis());
        }
        // The lock is unfair on Windows: waiting input goes first.
        while self.fronts.load(Ordering::SeqCst) > 0 {
            holds = self.clear.wait(holds).unwrap_or_else(PoisonError::into_inner);
        }
    }
}

/// A worker, weakly held, and its turn.
type Seen = (Weak<dyn Any + Send + Sync>, Arc<Turn>);

/// Each worker's turn; tests run many workers at once.
static TURNS: Mutex<Vec<Seen>> = Mutex::new(Vec::new());

/// The turn background work on `worker` queues for.
fn turn<W: Send + 'static>(worker: &Arc<Mutex<W>>) -> Arc<Turn> {
    let mut turns = TURNS.lock().expect("the turns");
    turns.retain(|(of, _)| of.strong_count() > 0);
    let any: Arc<dyn Any + Send + Sync> = worker.clone();
    let of = Arc::downgrade(&any);
    if let Some((_, turn)) = turns.iter().find(|(seen, _)| seen.ptr_eq(&of)) {
        return Arc::clone(turn);
    }
    let turn = Arc::new(Turn::default());
    turns.push((of, Arc::clone(&turn)));
    turn
}

/// Tells holding views apart.
static AHEADS: AtomicU64 = AtomicU64::new(0);

/// Holds a worker's background work while work is undrawn.
pub(super) struct Ahead {
    turn: Arc<Turn>,
    id: u64,
    since: Option<Instant>,
}

impl Ahead {
    pub(super) fn of<W: Send + 'static>(worker: &Arc<Mutex<W>>) -> Self {
        Ahead { turn: turn(worker), id: AHEADS.fetch_add(1, Ordering::Relaxed), since: None }
    }

    /// Holds for the edit sent at `since`; None releases.
    pub(super) fn hold(&mut self, since: Option<Instant>) {
        if since == self.since {
            return;
        }
        self.since = since;
        let mut holds = self.turn.holds();
        holds.retain(|(id, _)| *id != self.id);
        holds.extend(since.map(|since| (self.id, since)));
        self.turn.clear.notify_all();
    }
}

impl Drop for Ahead {
    fn drop(&mut self) {
        self.hold(None);
    }
}

/// A view's front: background work waits while it waits.
pub(super) struct Front(Arc<Turn>);

/// Counts one front wait; dropped, it lets background go.
struct Waiting<'a>(&'a Turn);

impl Drop for Waiting<'_> {
    fn drop(&mut self) {
        self.0.fronts.fetch_sub(1, Ordering::SeqCst);
        let _holds = self.0.holds();
        self.0.clear.notify_all();
    }
}

impl Front {
    pub(super) fn of<W: Send + 'static>(worker: &Arc<Mutex<W>>) -> Self {
        Front(turn(worker))
    }

    /// Takes the worker ahead of queued background work.
    pub(super) fn take<G>(&self, take: impl FnOnce() -> G) -> G {
        self.0.fronts.fetch_add(1, Ordering::SeqCst);
        let _waiting = Waiting(&self.0);
        take()
    }
}

/// Runs `work` once no other background work holds the turn.
#[track_caller]
pub fn behind<W: Send + 'static, T, F: FnOnce(&mut W) -> T>(worker: &Arc<Mutex<W>>, work: F) -> impl Future<Output = T> + use<W, T, F> {
    queued(Arc::clone(worker), work, Location::caller())
}

async fn queued<W: Send + 'static, T>(worker: Arc<Mutex<W>>, work: impl FnOnce(&mut W) -> T, at: &'static Location<'static>) -> T {
    let turn = turn(&worker);
    // Awaited: a queued task holds no pool thread.
    let _turn = turn.queue.lock().await;
    // The key's own tiles go first (PERF-10).
    turn.yield_to_edits();
    let mut held = worker.lock().expect("the worker lock");
    let began = Instant::now();
    let done = work(&mut held);
    let took = began.elapsed();
    if took > HELD_LONG {
        log::warn!("lok: background work from {at} held the worker {} ms", took.as_millis());
    }
    done
}

/// Runs `read` on a background thread; `then` takes its result.
#[track_caller]
pub fn read_off<E: 'static, T: Send + 'static>(
    worker: &Arc<Mutex<Worker>>,
    doc: &Doc,
    read: impl FnOnce(&mut Worker, &Doc) -> io::Result<T> + Send + 'static,
    then: impl FnOnce(&mut E, io::Result<T>, &mut Context<E>) + 'static,
    cx: &mut Context<E>,
) {
    let (worker, doc, at) = (Arc::clone(worker), doc.clone(), Location::caller());
    cx.spawn(async move |this, cx| {
        let read = cx
            .background_executor()
            .spawn(async move { queued(worker, |worker| read(worker, &doc), at).await })
            .await;
        if this
            .update(cx, |entity, cx| then(entity, read, cx))
            .is_err()
        {
            log::info!("lok: a read outlived its reader");
        }
    })
    .detach();
}
