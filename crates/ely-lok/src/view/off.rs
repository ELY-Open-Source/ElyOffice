//! Kernel reads off the UI thread.

use std::{
    any::Any,
    io,
    sync::{Arc, Condvar, Mutex, PoisonError, Weak},
    time::{Duration, Instant},
};

use gpui::Context;

use crate::{Doc, Worker};

/// How long an input's tile goes before background work.
const KEEN: Duration = Duration::from_millis(100);

/// Where the view's last input is.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Keen {
    Free,
    /// Running on the worker.
    Running,
    /// Ran; its tile is due by then.
    Until(Instant),
}

/// Background work on one worker, after the view's input.
#[derive(Debug)]
pub(crate) struct Order {
    lock: futures::lock::Mutex<()>,
    keen: Mutex<Keen>,
    moved: Condvar,
}

impl Default for Order {
    fn default() -> Self {
        Order { lock: Default::default(), keen: Mutex::new(Keen::Free), moved: Condvar::new() }
    }
}

impl Order {
    fn set(&self, keen: Keen) {
        // A plain value: a panicked holder leaves it whole.
        *self.keen.lock().unwrap_or_else(PoisonError::into_inner) = keen;
        self.moved.notify_all();
    }

    /// An input starts: background work waits.
    pub(crate) fn input(&self) {
        self.set(Keen::Running);
    }

    /// The input ran; its tile comes next.
    pub(crate) fn ran(&self) {
        self.set(Keen::Until(Instant::now() + KEEN));
    }

    /// A tile is painted: background work may go.
    pub(crate) fn painted(&self) {
        self.set(Keen::Free);
    }

    /// Blocks while an input or its tile is due.
    fn wait(&self) -> Duration {
        let began = Instant::now();
        let mut keen = self.keen.lock().unwrap_or_else(PoisonError::into_inner);
        loop {
            keen = match *keen {
                Keen::Free => break,
                Keen::Running => self.moved.wait(keen).unwrap_or_else(PoisonError::into_inner),
                Keen::Until(until) => match until.checked_duration_since(Instant::now()) {
                    Some(left) if !left.is_zero() => self.moved.wait_timeout(keen, left).unwrap_or_else(PoisonError::into_inner).0,
                    _ => break,
                },
            };
        }
        began.elapsed()
    }
}

type Turn = Arc<Order>;

/// Each worker's turn; tests run many workers at once.
static TURNS: Mutex<Vec<(Weak<dyn Any + Send + Sync>, Turn)>> = Mutex::new(Vec::new());

/// The turn background work on `worker` queues for.
pub(crate) fn turn<W: Send + 'static>(worker: &Arc<Mutex<W>>) -> Turn {
    let mut turns = TURNS.lock().expect("the turns");
    turns.retain(|(of, _)| of.strong_count() > 0);
    let any: Arc<dyn Any + Send + Sync> = worker.clone();
    let of = Arc::downgrade(&any);
    if let Some((_, turn)) = turns.iter().find(|(seen, _)| seen.ptr_eq(&of)) {
        return Arc::clone(turn);
    }
    let turn = Turn::default();
    turns.push((of, Arc::clone(&turn)));
    turn
}

/// Runs `work` once no other background work holds the turn.
pub async fn behind<W: Send + 'static, T>(worker: &Arc<Mutex<W>>, work: impl FnOnce(&mut W) -> T) -> T {
    let turn = turn(worker);
    // Awaited: a queued task holds no pool thread.
    let _turn = turn.lock.lock().await;
    // The view's input and its tile go first (R15.57).
    let waited = turn.wait();
    if !waited.is_zero() {
        log::debug!("lok: background work waited {} ms for an input", waited.as_millis());
    }
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
