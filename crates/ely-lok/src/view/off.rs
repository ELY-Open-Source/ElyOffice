//! Kernel reads off the UI thread.

use std::{
    collections::HashMap,
    io,
    sync::{Arc, LazyLock, Mutex, PoisonError},
};

use gpui::Context;

use crate::{Doc, Worker};

/// Each worker's turn, by its lock's address.
static TURNS: LazyLock<Mutex<HashMap<usize, Arc<Mutex<()>>>>> = LazyLock::new(Mutex::default);

/// Runs `work` once no other background work holds `worker`'s turn.
pub fn behind<W, T>(worker: &Mutex<W>, work: impl FnOnce(&mut W) -> T) -> T {
    let turn = {
        // An insert never leaves the map half changed.
        let mut turns = TURNS.lock().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(turns.entry(std::ptr::from_ref(worker).addr()).or_default())
    };
    let _turn = turn.lock().expect("the background turn");
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
            .spawn(async move { behind(&worker, |worker| read(worker, &doc)) })
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
