//! What this app keeps, clearing it, and recovery limits (§18.3).

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use ely_gpui_component::{
    buttons::{Button, SegmentedControl},
    feedback::Banner,
    overlays::ConfirmDialog,
    primitives::Severity,
    settings::{SettingsRow, SettingsSection},
};
use ely_platform::{
    folder,
    storage::{self, Kind, Limits},
};
use gpui::{AnyElement, App, Context, IntoElement, SharedString, WeakEntity};

use super::window::SettingsView;
use crate::i18n::text;

/// Each kind's size, and whether a clear waits.
type Usage = Vec<(Kind, u64, bool)>;

/// Last read sizes and limits; a clear waiting or failed.
#[derive(Default)]
pub(crate) struct Storage {
    usage: Option<Result<Usage, SharedString>>,
    limits: Option<Result<Limits, SharedString>>,
    pub(super) folder: Option<Result<folder::State, SharedString>>,
    /// A clear waiting for the user's yes.
    confirming: Option<Kind>,
    pub(super) failed: Option<SharedString>,
}

/// Limit writes outlive the settings window that asked.
#[derive(Default)]
struct Limiting {
    /// The latest commit; older writes are dropped.
    asked: Arc<AtomicU64>,
    /// Picked but not yet written; a read keeps them.
    pending: Option<Limits>,
    /// The last write's failure, until one succeeds.
    failed: Option<SharedString>,
}

impl gpui::Global for Limiting {}

/// An app's own storage rows: reread on each visit.
pub struct AppStorage {
    pub read: std::rc::Rc<dyn Fn(&mut App)>,
    pub rows: std::rc::Rc<dyn Fn(&App) -> Vec<AnyElement>>,
    /// Its yes-or-no before deleting, when asked.
    pub dialog: std::rc::Rc<dyn Fn(&App) -> Option<AnyElement>>,
}

impl gpui::Global for AppStorage {}

/// The app's rows join every settings window's storage page.
pub fn add_rows(storage: AppStorage, cx: &mut App) {
    cx.set_global(storage);
}

impl SettingsView {
    /// Reads sizes and limits off the window thread.
    pub(super) fn read_storage(&mut self, cx: &mut Context<Self>) {
        if let Some(read) = cx.try_global::<AppStorage>().map(|own| own.read.clone()) {
            read(cx);
        }
        let app = self.app;
        let read = cx.background_executor().spawn(async move {
            let usage = storage::usage(app).and_then(|usage| {
                usage.into_iter().map(|(kind, bytes)| Ok((kind, bytes, storage::clearing(app, kind)?))).collect()
            });
            (usage, storage::limits(app), folder::state())
        });
        cx.spawn(async move |this, cx| {
            let (usage, limits, folder) = read.await;
            this.update(cx, |this, cx| {
                this.storage.folder = Some(folder.map_err(|error| error.to_string().into()));
                this.storage.usage = Some(usage.map_err(|error| error.to_string().into()));
                this.storage.limits = Some(match cx.default_global::<Limiting>().pending {
                    Some(wanted) => Ok(wanted),
                    None => limits.map_err(|error| error.to_string().into()),
                });
                cx.notify();
            })
        })
        .detach();
    }

    pub(super) fn storage_page(&self, this: WeakEntity<Self>, cx: &App) -> AnyElement {
        let mut section = SettingsSection::new(text(cx, "settings.storage", &[]));
        for row in self.folder_rows(this.clone(), cx) {
            section = section.row(row);
        }
        match &self.storage.usage {
            None => {}
            Some(Err(error)) => section = section.row(Banner::new("storage-unread", Severity::Danger, error.clone())),
            Some(Ok(usage)) => {
                for (kind, bytes, clearing) in usage {
                    section = section.row(row(*kind, *bytes, *clearing, this.clone(), cx));
                }
            }
        }
        match &self.storage.limits {
            None => {}
            Some(Err(error)) => section = section.row(Banner::new("storage-limits", Severity::Danger, error.clone())),
            Some(Ok(limits)) => {
                let keep = choices("storage-keep", &[10, 20, 50], limits.keep as u64, this.clone(), |limits, n| Limits {
                    keep: n as usize,
                    ..limits
                });
                let space = choices("storage-space", &[32, 64, 256], limits.megabytes, this, |limits, n| Limits {
                    megabytes: n,
                    ..limits
                });
                section = section
                    .row(SettingsRow::new(text(cx, "settings.storage.keep", &[])).control(keep))
                    .row(SettingsRow::new(text(cx, "settings.storage.space", &[])).control(space));
            }
        }
        let unsaved = cx.try_global::<Limiting>().and_then(|limiting| limiting.failed.clone());
        if let Some(failed) = unsaved {
            section = section.row(Banner::new("storage-unsaved", Severity::Danger, failed));
        }
        if let Some(failed) = self.storage.failed.clone() {
            section = section.row(Banner::new("storage-failed", Severity::Danger, failed));
        }
        if let Some(own) = cx.try_global::<AppStorage>() {
            for row in (own.rows)(cx) {
                section = section.row(row);
            }
        }
        section.into_any_element()
    }

    /// The yes-or-no before data is deleted (§4.2).
    pub(super) fn storage_dialog(&self, this: WeakEntity<Self>, cx: &App) -> Option<AnyElement> {
        let Some(kind) = self.storage.confirming else {
            return cx.try_global::<AppStorage>().and_then(|own| (own.dialog)(cx));
        };
        let what = name(kind, cx);
        let message = match kind.at_next_start() {
            true => text(cx, "settings.storage.ask_later", &[("what", &what)]),
            false => text(cx, "settings.storage.ask_now", &[("what", &what)]),
        };
        let close = this.clone();
        let dialog = ConfirmDialog::new("storage-ask", text(cx, "settings.storage.ask_title", &[("what", &what)]), message, move |_, cx| {
            close.update(cx, |this, cx| {
                this.storage.confirming = None;
                cx.notify();
            })
            .ok();
        })
        .confirm(text(cx, "settings.storage.delete", &[]))
        .destructive()
        .on_confirm(move |_, cx| {
            this.update(cx, |this, cx| this.clear_storage(kind, cx)).ok();
        });
        Some(dialog.into_any_element())
    }

    fn clear_storage(&mut self, kind: Kind, cx: &mut Context<Self>) {
        let app = self.app;
        let cleared = cx.background_executor().spawn(async move { storage::clear(app, kind) });
        cx.spawn(async move |this, cx| {
            let done = cleared.await;
            this.update(cx, |this, cx| this.storage_done(done.map(|_| ()), cx))
        })
        .detach();
    }

    fn storage_done(&mut self, done: std::io::Result<()>, cx: &mut Context<Self>) {
        self.storage.failed = done.err().map(|error| text(cx, "settings.storage.failed", &[("error", &error.to_string())]));
        self.read_storage(cx);
    }

    /// Writes the shown limits with one changed; latest wins.
    fn pick_limits(&mut self, change: impl FnOnce(Limits) -> Limits, cx: &mut Context<Self>) {
        let Some(Ok(shown)) = self.storage.limits.clone() else {
            return;
        };
        let wanted = change(shown);
        self.storage.limits = Some(Ok(wanted));
        let limiting = cx.default_global::<Limiting>();
        limiting.pending = Some(wanted);
        let asked = limiting.asked.fetch_add(1, Ordering::SeqCst) + 1;
        let (app, latest) = (self.app, Arc::clone(&limiting.asked));
        let kept = cx
            .background_executor()
            .spawn(async move { storage::set_limits(app, wanted, || latest.load(Ordering::SeqCst) == asked) });
        // The global settles even when this window has closed.
        cx.spawn(async move |this, cx| {
            let done = kept.await;
            let latest = cx.update(|cx| {
                let failed = done.as_ref().err().map(|error| text(cx, "settings.storage.failed", &[("error", &error.to_string())]));
                let limiting = cx.global_mut::<Limiting>();
                let latest = limiting.asked.load(Ordering::SeqCst) == asked;
                if latest {
                    limiting.failed = failed;
                    if done.is_ok() {
                        limiting.pending = None;
                    }
                }
                latest
            });
            if latest {
                this.update(cx, |this, cx| this.read_storage(cx)).ok();
            }
        })
        .detach();
        cx.notify();
    }
}

fn name(kind: Kind, cx: &App) -> SharedString {
    match kind {
        Kind::Log => text(cx, "settings.storage.log", &[]),
        Kind::Recovery => text(cx, "settings.storage.recovery", &[]),
    }
}

/// One kind: its size, cleared after a yes.
fn row(kind: Kind, bytes: u64, clearing: bool, this: WeakEntity<SettingsView>, cx: &App) -> AnyElement {
    let size = text(cx, "settings.storage.size", &[("size", &bytes.div_ceil(1024).to_string())]);
    let row = SettingsRow::new(name(kind, cx)).description(size);
    if clearing {
        let waiting = Button::new(SharedString::from(format!("storage-waiting-{kind:?}")), text(cx, "settings.storage.next_start", &[]));
        return row.control(waiting.disabled(true)).into_any_element();
    }
    let label = match kind.at_next_start() {
        true => text(cx, "settings.storage.clear_later", &[]),
        false => text(cx, "settings.storage.clear", &[]),
    };
    let button = Button::new(SharedString::from(format!("storage-clear-{kind:?}")), label)
        .disabled(bytes == 0)
        .on_click(move |_, _, cx| {
            this.update(cx, |this, cx| {
                this.storage.confirming = Some(kind);
                cx.notify();
            })
            .ok();
        });
    row.control(button).into_any_element()
}

/// A few sizes to pick from; picking writes the limits.
fn choices(
    id: &'static str,
    options: &[u64],
    chosen: u64,
    this: WeakEntity<SettingsView>,
    change: fn(Limits, u64) -> Limits,
) -> SegmentedControl {
    options
        .iter()
        .fold(SegmentedControl::new(id, chosen.to_string()), |control, n| {
            control.segment(n.to_string(), n.to_string(), None)
        })
        .on_change(move |picked, _, cx| {
            let Ok(n) = picked.parse::<u64>() else {
                return log::error!("settings: {id} offered {picked}");
            };
            this.update(cx, |this, cx| this.pick_limits(|limits| change(limits, n), cx)).ok();
        })
}
