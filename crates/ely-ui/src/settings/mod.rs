//! The shared settings: read at start, followed while running (§18.1).

use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use ely_platform::settings::{Shared, Store};
use futures::{StreamExt, channel::mpsc};
use gpui::{App, AsyncApp, Global, SharedString};
use notify::{RecursiveMode, Watcher as _};
use serde_json::Value;

pub mod switches;
mod diagnostics;
mod frame;
mod storage;
pub(crate) mod panels;
#[cfg(test)]
mod tests;
mod window;

pub use frame::{follow as follow_frame, restore as restored_frame};
pub use storage::{AppStorage, add_rows as add_storage_rows};
pub use window::open;

const SETTLE: Duration = Duration::from_millis(150);

/// The settings in force, why reading failed, why saving failed.
pub struct Current {
    pub shared: Shared,
    pub error: Option<SharedString>,
    /// Each save kind's last failure, cleared by its success.
    pub unsaved: BTreeMap<&'static str, SharedString>,
    /// Why each kind of app setting was unreadable.
    pub app_error: BTreeMap<&'static str, SharedString>,
}

impl Global for Current {}

/// The settings every app follows.
pub fn current(cx: &App) -> &Shared {
    &cx.global::<Current>().shared
}

/// Defaults in force without reading the file, for tests.
#[cfg(any(test, feature = "test-support"))]
pub fn install_defaults(cx: &mut App) {
    cx.set_global(Current { shared: Shared::default(), error: None, unsaved: BTreeMap::new(), app_error: BTreeMap::new() });
    panels::install_defaults(cx);
    switches::install_defaults(cx);
}

/// Reads the shared file and follows later writes to it.
pub fn install(app: ely_platform::AppId, cx: &mut App) {
    let (shared, error) = read();
    cx.set_global(Current { shared, error, unsaved: BTreeMap::new(), app_error: BTreeMap::new() });
    panels::install(app, cx);
    switches::install(app, cx);
    if let Err(error) = follow(cx) {
        log::error!("settings: changes from other apps are not followed: {error}");
    }
}

/// The file's settings; defaults with the reason when unreadable.
fn read() -> (Shared, Option<SharedString>) {
    match Shared::load() {
        Ok(shared) => (shared, None),
        Err(error) => {
            log::error!("settings: unreadable, defaults in force: {error}");
            (Shared::default(), Some(error.to_string().into()))
        }
    }
}

fn follow(cx: &mut App) -> std::io::Result<()> {
    let path = Store::shared()?.path().to_path_buf();
    let dir = path.parent().expect("the settings folder").to_path_buf();
    std::fs::create_dir_all(&dir)?;
    let (sender, mut changes) = mpsc::unbounded();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| match event {
        // Reads, ours included, are no change.
        Ok(event) if crate::watching::only_read(&event.kind) => {}
        Ok(event) if event.paths.iter().any(|changed| changed.file_name() == path.file_name()) => {
            let _ = sender.unbounded_send(());
        }
        Ok(_) => {}
        Err(error) => log::warn!("settings: watching failed: {error}"),
    })
    .map_err(std::io::Error::other)?;
    watcher.watch(&dir, RecursiveMode::NonRecursive).map_err(std::io::Error::other)?;
    cx.spawn(async move |cx: &mut AsyncApp| {
        let _watcher = watcher;
        while changes.next().await.is_some() {
            cx.background_executor().timer(SETTLE).await;
            while changes.try_recv().is_ok() {}
            let read = cx.background_executor().spawn(async { read() }).await;
            cx.update(|cx| changed(read, cx));
        }
    })
    .detach();
    Ok(())
}

fn changed((shared, error): (Shared, Option<SharedString>), cx: &mut App) {
    let current = cx.global::<Current>();
    if current.shared == shared && current.error == error {
        return;
    }
    log::info!("settings: changed on disk");
    let (unsaved, app_error) = (current.unsaved.clone(), current.app_error.clone());
    cx.set_global(Current { shared, error, unsaved, app_error });
    crate::theme::settings_changed(cx);
    cx.refresh_windows();
}

/// Save kinds, each with its own failure.
pub(crate) const SHARED: &str = "shared";
pub(crate) const PANELS: &str = "panels";
pub(crate) const WINDOW: &str = "window";

/// Records and shows how `kind`'s last save went.
pub(crate) fn unsaved(kind: &'static str, failed: Option<SharedString>, cx: &mut App) {
    let current = cx.global_mut::<Current>();
    match failed {
        Some(error) => {
            current.unsaved.insert(kind, error);
        }
        // A good write also mends what could not be read.
        None => {
            current.unsaved.remove(kind);
            current.app_error.remove(kind);
        }
    }
    cx.refresh_windows();
}

/// Writes one shared field in the background; others stay.
pub fn set(key: &'static str, value: Value, cx: &mut App) {
    saved(key, value, None, cx).detach();
}

/// A field's latest commit number, and this commit's.
pub(crate) type Turn = (Arc<AtomicU64>, u64);

/// As `set`; whether written, a later turn skipping it.
pub(crate) fn saved(key: &'static str, value: Value, turn: Option<Turn>, cx: &mut App) -> gpui::Task<bool> {
    let current = turn.clone();
    let write = cx.background_executor().spawn(async move {
        let still = || turn.is_none_or(|(latest, asked)| latest.load(Ordering::SeqCst) == asked);
        Store::shared()?.set_if(key, value, still)
    });
    cx.spawn(async move |cx: &mut AsyncApp| {
        let written = write.await;
        let read = cx.background_executor().spawn(async { read() }).await;
        cx.update(|cx| {
            changed(read, cx);
            let ok = matches!(written, Ok(true));
            // Only the latest choice's outcome shows.
            if current.is_none_or(|(latest, asked)| latest.load(Ordering::SeqCst) == asked) {
                // The reread must not hide this write's failure.
                unsaved(SHARED, written.err().map(|error| {
                    log::error!("settings: {key} not saved: {error}");
                    error.to_string().into()
                }), cx);
            }
            ok
        })
    })
}
