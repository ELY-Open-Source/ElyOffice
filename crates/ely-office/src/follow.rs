//! Reads of tab state that follow their tab between windows.

use std::io;

use gpui::Context;

use crate::{DocOf, Host, Tabs as _, WorkerOf};

pub trait Following: Host {
    /// Reads tab `id`; `then` runs where the tab is.
    #[track_caller]
    fn read_for_tab<T: Send + 'static>(
        &self,
        id: u64,
        doc: &DocOf<Self>,
        read: impl FnOnce(&mut WorkerOf<Self>, &DocOf<Self>) -> io::Result<T> + Send + 'static,
        then: impl FnOnce(&mut Self, io::Result<T>, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        let doc = doc.clone();
        // Built here: the worker's hold log names the caller.
        let read = ely_lok::behind(&self.core().worker, move |worker| read(worker, &doc));
        let shared = self.core().shared.clone();
        cx.spawn(async move |_, cx| {
            // Queued from the task, as `read_off` does.
            let read = cx.background_executor().spawn(read).await;
            cx.update(|cx| {
                let Some(host) = shared.hosts().into_iter().find(|host| host.read(cx).tab(id).is_some()) else {
                    return log::info!("{}: tab {id} closed before its read landed", Self::KIND.app.slug());
                };
                host.update(cx, |host, cx| then(host, read, cx));
            });
        })
        .detach();
    }
}

impl<H: Host> Following for H {}
