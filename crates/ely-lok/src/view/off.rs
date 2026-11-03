//! Kernel reads off the UI thread.

use std::{
    io,
    sync::{Arc, Mutex},
};

use gpui::Context;

use crate::{Doc, Worker};

/// Background work queues here; views lock the worker directly.
static TURN: Mutex<()> = Mutex::new(());

/// Runs `work` once no other background work holds the turn.
pub fn behind<W, T>(worker: &Mutex<W>, work: impl FnOnce(&mut W) -> T) -> T {
    let _turn = TURN.lock().expect("the background turn");
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
