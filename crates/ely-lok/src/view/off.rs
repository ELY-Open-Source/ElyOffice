//! Kernel reads off the UI thread.

use std::{
    any::Any,
    io,
    sync::{Arc, Mutex, Weak},
};

use gpui::Context;

use crate::{Doc, Worker};

type Turn = Arc<futures::lock::Mutex<()>>;

/// Each worker's turn; tests run many workers at once.
static TURNS: Mutex<Vec<(Weak<dyn Any + Send + Sync>, Turn)>> = Mutex::new(Vec::new());

/// The turn background work on `worker` queues for.
fn turn<W: Send + 'static>(worker: &Arc<Mutex<W>>) -> Turn {
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
    let _turn = turn.lock().await;
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
