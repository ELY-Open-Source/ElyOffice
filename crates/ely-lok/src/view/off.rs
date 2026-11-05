//! Kernel reads off the UI thread.

use std::{
    any::Any,
    io,
    sync::{
        Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use gpui::Context;

use crate::{Doc, Worker};

/// An undrawn edit holds background work this long at most.
const HELD_MOST: Duration = Duration::from_secs(1);

/// One worker's background queue and the edits it yields to.
#[derive(Default)]
struct Turn {
    queue: futures::lock::Mutex<()>,
    /// Each holding view and when its newest undrawn edit was sent.
    holds: Mutex<Vec<(u64, Instant)>>,
    clear: Condvar,
}

impl Turn {
    /// Plain data: a panic leaves it whole.
    fn holds(&self) -> MutexGuard<'_, Vec<(u64, Instant)>> {
        self.holds.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Waits while an undrawn edit is under a second old.
    fn yield_to_edits(&self) {
        let began = Instant::now();
        let mut holds = self.holds();
        loop {
            let now = Instant::now();
            let until = holds.iter().map(|(_, since)| *since + HELD_MOST).filter(|until| *until > now).max();
            let Some(until) = until else { break };
            holds = self.clear.wait_timeout(holds, until - now).unwrap_or_else(PoisonError::into_inner).0;
        }
        let stuck = !holds.is_empty();
        drop(holds);
        let waited = began.elapsed();
        if stuck && !waited.is_zero() {
            log::warn!("lok: background work held {} ms by an undrawn edit; it goes", waited.as_millis());
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

/// Holds a worker's background work while an edit is undrawn.
pub(super) struct Ahead {
    turn: Arc<Turn>,
    id: u64,
    since: Option<Instant>,
}

impl Ahead {
    pub(super) fn of<W: Send + 'static>(worker: &Arc<Mutex<W>>) -> Self {
        Ahead { turn: turn(worker), id: AHEADS.fetch_add(1, Ordering::Relaxed), since: None }
    }

    /// Holds background work for the edit sent at `since`; None lets it go.
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

/// Runs `work` once no other background work holds the turn.
pub async fn behind<W: Send + 'static, T>(worker: &Arc<Mutex<W>>, work: impl FnOnce(&mut W) -> T) -> T {
    let turn = turn(worker);
    // Awaited: a queued task holds no pool thread.
    let _turn = turn.queue.lock().await;
    // The key's own tiles go first (PERF-10).
    turn.yield_to_edits();
    work(&mut worker.lock().expect("the worker lock"))
}

/// Runs `read` on a background thread; `then` takes its result.
pub fn read_off<E: 'static, T: Send + 'static>(
    worker: &Arc<Mutex<Worker>>,
    doc: &Doc,
    read: impl FnOnce(&mut Worker, &Doc) -> io::Result<T> + Send + 'static,
    then: impl FnOnce(&mut E, io::Result<T>, &mut Context<E>) + 'static,
    cx: &mut Context<E>,
) {
    let (worker, doc) = (Arc::clone(worker), doc.clone());
    cx.spawn(async move |this, cx| {
        let read = cx
            .background_executor()
            .spawn(async move { behind(&worker, |worker| read(worker, &doc)).await })
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
