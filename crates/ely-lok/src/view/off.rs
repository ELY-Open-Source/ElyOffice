//! Kernel reads off the UI thread.

use std::{
    any::Any,
    io,
    sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak},
    time::{Duration, Instant},
};

use gpui::Context;

use crate::{Doc, Worker};

/// Background work holds back this long at most.
const HELD_MOST: Duration = Duration::from_secs(1);

/// One worker's background queue and the edits it yields to.
#[derive(Default)]
struct Turn {
    queue: futures::lock::Mutex<()>,
    /// Views whose edit is not yet on screen.
    ahead: Mutex<usize>,
    clear: Condvar,
}

impl Turn {
    /// A bare count: a panic leaves it whole.
    fn ahead(&self) -> MutexGuard<'_, usize> {
        self.ahead.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Waits until no edit is undrawn, or a second.
    fn yield_to_edits(&self) {
        let began = Instant::now();
        let held = self.clear.wait_timeout_while(self.ahead(), HELD_MOST, |ahead| *ahead > 0);
        let (ahead, waited) = held.unwrap_or_else(PoisonError::into_inner);
        drop(ahead);
        if waited.timed_out() {
            log::warn!("lok: background work held {} ms by an undrawn edit; it goes", began.elapsed().as_millis());
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

/// Holds a worker's background work while an edit is undrawn.
pub(super) struct Ahead {
    turn: Arc<Turn>,
    held: bool,
}

impl Ahead {
    pub(super) fn of<W: Send + 'static>(worker: &Arc<Mutex<W>>) -> Self {
        Ahead { turn: turn(worker), held: false }
    }

    /// Holds background work back while `held`.
    pub(super) fn hold(&mut self, held: bool) {
        if held == self.held {
            return;
        }
        self.held = held;
        let mut ahead = self.turn.ahead();
        if held {
            *ahead += 1;
        } else {
            *ahead -= 1;
            if *ahead == 0 {
                self.turn.clear.notify_all();
            }
        }
    }
}

impl Drop for Ahead {
    fn drop(&mut self) {
        self.hold(false);
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
