//! The recovery index on disk, written one at a time.

use gpui::{AppContext as _, AsyncApp, Context, WeakEntity};
use ely_gpui_component::primitives::Severity;

use super::{Recovering, write_index};
use crate::{Tabs as _, text};

/// Writes the index in the background, one at a time.
pub(super) fn flush<H: Recovering>(host: &mut H, cx: &mut Context<H>) {
    let shared = host.core().shared.clone();
    let mut held = shared.get_mut();
    let Some(keeping) = held.keeping.as_mut() else { return };
    if keeping.flushing {
        return keeping.flush_again = true;
    }
    let (path, bytes) = match keeping.index() {
        Ok(index) => index,
        Err(error) => {
            drop(held);
            return host.index_failed(error.to_string(), cx);
        }
    };
    keeping.flushing = true;
    let (turn, turns) = (keeping.turns.take(), keeping.turns.clone());
    drop(held);
    let task = cx.background_spawn(async move { turns.write(turn, || write_index(&path, &bytes)) });
    let shared = host.core().shared.clone();
    cx.spawn(async move |this: WeakEntity<H>, cx: &mut AsyncApp| {
        let written = task.await;
        // The record ends it, not the writing window.
        let again = {
            let mut held = shared.get_mut();
            let Some(keeping) = held.keeping.as_mut() else { return };
            keeping.flushing = false;
            if let Some(Ok(())) = written {
                keeping.behind = None;
            }
            std::mem::take(&mut keeping.flush_again)
        };
        // A later write, the quit's, holds the newest.
        let failed = written.and_then(Result::err).map(|error| error.to_string());
        if failed.is_none() && !again {
            return;
        }
        let Some(heir) = shared.heir(&this) else {
            return log::warn!("recovery: no window is left to write the index again");
        };
        heir.update(cx, |host, cx| {
            if let Some(error) = failed {
                host.index_failed(error, cx);
            }
            if again {
                host.flush_index(cx);
            }
        })
        .ok();
    })
    .detach();
}

/// An index failure shows once and retries.
pub(super) fn failed<H: Recovering>(host: &mut H, error: String, cx: &mut Context<H>) {
    {
        let mut shared = host.core().shared.get_mut();
        let Some(keeping) = shared.keeping.as_mut() else { return };
        if keeping.behind.as_ref() == Some(&error) {
            return;
        }
        log::error!("recovery: the index is not saved: {error}");
        keeping.behind = Some(error.clone());
    }
    host.toast_unlogged(text(cx, "office.recovery.index", &[("error", &error)]), Severity::Danger, cx);
}
