//! Points refused for size: measured at opening and again.

use gpui::{AppContext as _, AsyncApp, Context, WeakEntity};

use super::{Recovering, disk};
use crate::{Engine as _, Tabs as _};

/// Refused slots that fit the cap now take points again.
pub(super) fn recheck<H: Recovering>(host: &mut H, cx: &mut Context<H>) {
    let (refused, dir) = {
        let shared = host.core().shared.get();
        let Some(keeping) = shared.keeping.as_ref() else { return };
        let slots = keeping.slots.iter();
        let refused: Vec<_> = slots.filter_map(|(id, slot)| Some((*id, keeping.folder(&slot.kept.slot), slot.kept.sealed, slot.refused?))).collect();
        (refused, keeping.dir.clone())
    };
    if refused.is_empty() {
        return;
    }
    let task = cx.background_spawn(async move { disk::fitting(&dir, refused) });
    cx.spawn(async move |this: WeakEntity<H>, cx: &mut AsyncApp| {
        let fitting = task.await;
        this.update(cx, |host, _| {
            let mut shared = host.core().shared.get_mut();
            let Some(keeping) = shared.keeping.as_mut() else { return };
            // A save meanwhile measures again next tick.
            for (id, size) in fitting {
                if let Some(slot) = keeping.slots.get_mut(&id).filter(|slot| slot.refused == Some(size)) {
                    log::info!("recovery: tab {id} fits the cap again");
                    slot.refused = None;
                }
            }
        })
        .ok();
    })
    .detach();
}

/// A file opened over its cap takes no snapshot.
pub(super) fn opened<H: Recovering>(host: &mut H, id: u64, size: u64, cx: &mut Context<H>) {
    let (dir, folder, sealed) = {
        let shared = host.core().shared.get();
        let Some(keeping) = shared.keeping.as_ref() else { return };
        let Some(slot) = keeping.slots.get(&id) else { return };
        (keeping.dir.clone(), keeping.folder(&slot.kept.slot), slot.kept.sealed)
    };
    let task = cx.background_spawn(async move { disk::recovery_in(&dir, folder, sealed)?.fits(size) });
    cx.spawn(async move |this: WeakEntity<H>, cx: &mut AsyncApp| {
        let Err(error) = task.await else { return };
        this.update(cx, |host, cx| {
            let Some(tab) = host.tab(id) else { return };
            let edited = H::Engine::edited(tab.view.read(cx));
            // A snapshot in flight measures itself.
            let writing = host.core().shared.get().keeping.as_ref().and_then(|keeping| keeping.slots.get(&id)).is_none_or(|slot| slot.writing);
            if writing {
                return;
            }
            log::info!("recovery: tab {id} opened over its cap");
            host.point_written(id, (edited, Some(size)), Err(error), cx);
        })
        .ok();
    })
    .detach();
}
