//! The shell document apps share (§6), over any engine.

pub mod chart;
mod autosave;
mod chrome;
mod close;
mod dialogs;
pub mod engine;
pub mod files;
mod find;
pub mod lok;
pub mod menu;
mod open;
mod recover;
pub mod remote;
pub mod panel;
mod save;
mod stopped;
mod undo;
mod zoom;

use std::{
    io,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use ely_files::{SaveState, Session, Sessions};
use ely_gpui_component::primitives::Severity;
use ely_platform::AppId;
use ely_ui::Root;
use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, SharedString, TaskExt as _, WeakEntity,
    Window,
};

pub use autosave::Autosaving;
pub use chrome::Chrome;
pub use close::{After, Closing};
pub use dialogs::{Dialog, Dialogs, cancel};
pub use engine::{Engine, Load, Stop};
pub use find::{FindReplace, Finder, Finding};
pub use lok::{Commands, Lok, start};
pub use open::{Opening, Source};
pub use recover::{Keeping, Recovering};
pub use remote::Remote;
pub use stopped::{Stopped, Stopping};
pub use save::{Saving, folder};
pub use undo::{Redo, Undo, Undoing, bind_undo_keys, undo_able, undo_items};
pub use zoom::{ActualSize, ZoomIn, ZoomOut, Zooming, bind_zoom_keys, zoom_items};

/// Every shared trait, for `use ely_office::prelude::*`.
pub mod prelude {
    pub use crate::{Chrome, Closing, Commands, Dialogs, Finding, Host, Opening, Remote, Saving, Stopping, Tabs, Undoing, Zooming};
}

/// What one app opens and writes.
pub struct Kind {
    pub app: AppId,
    /// Formats a save keeps, as the engine names them.
    pub kept: &'static [&'static str],
    /// Formats Save As converts to; the first is the default.
    pub save_as: &'static [&'static str],
    /// Extensions that always carry macros; None for macro-free formats.
    pub macros: Option<&'static [&'static str]>,
}

/// An app built on the shared shell.
pub trait Host: Sized + 'static {
    /// What the app keeps per tab.
    type Extra: Default;
    type Engine: Engine;
    const KIND: Kind;
    fn core(&self) -> &Core<Self>;
    fn core_mut(&mut self) -> &mut Core<Self>;
    /// Runs when the kernel reports on tab `id`.
    fn observed(&mut self, id: u64, view: &Entity<View<Self>>, cx: &mut Context<Self>);
    /// Runs before a search first moves the cursor.
    fn search_leaves(&mut self, _: &mut Context<Self>) {}
    /// Runs as a quit starts, before any tab closes.
    fn quitting(&mut self, _: &mut Context<Self>) {}
    /// Runs when a close stops: question or save cancelled.
    fn close_cancelled(&mut self, _: &mut Context<Self>) {}
    /// An outside change met a save; true if shown.
    fn conflicted(&mut self, _id: u64, _: &mut Context<Self>) -> bool {
        false
    }
    /// Runs as tab `tab` leaves, saved, clean or discarded.
    fn tab_closed(&mut self, _tab: &Tab<Self::Extra, Self::Engine>, _: &mut Context<Self>) {}

    /// Tab `tab`'s document as last seen; reads and numbered commands.
    fn read_at(&self, _tab: u64, _cx: &App) -> Option<<Self::Engine as Engine>::Doc> {
        None
    }
}

/// The view type of host `H`'s engine.
pub type View<H> = <<H as Host>::Engine as Engine>::View;
/// The worker type of host `H`'s engine.
pub type WorkerOf<H> = <<H as Host>::Engine as Engine>::Worker;
/// The document type of host `H`'s engine.
pub type DocOf<H> = <<H as Host>::Engine as Engine>::Doc;

/// Why saving in place would lose something.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Risk {
    /// Macros Ely will not round-trip (LOK-11).
    Macros,
    /// Signatures an edit breaks (§11.4).
    Signed,
    /// A damaged file opened through a repaired copy (PDF-14).
    Repaired,
    /// PPTX slide sections the kernel drops (D-02).
    Sections,
    /// An encrypted PPTX whose sections Ely cannot see.
    Sealed,
}

/// One open document.
pub struct Tab<E, G: Engine> {
    pub id: u64,
    pub view: Entity<G::View>,
    pub doc: G::Doc,
    pub session: Session,
    /// Its id in the same-file registry; untitled tabs have none.
    pub registry: Option<usize>,
    pub name: SharedString,
    /// The kernel says it is modified.
    pub modified: bool,
    /// Why Save writes a copy, if it does.
    pub risk: Option<Risk>,
    /// The kernel's undo history at the last load or save.
    pub mark: String,
    pub work: PathBuf,
    /// A save's export runs in the background.
    pub exporting: bool,
    /// Reopen from the file once memory matches it.
    pub reopen: bool,
    pub extra: E,
}

impl<E, G: Engine> Tab<E, G> {
    pub fn dirty(&self, cx: &App) -> bool {
        self.modified
            || self.session.state() != SaveState::Saved
            || G::edit_pending(self.view.read(cx))
    }

    /// The status line for this tab (FILE-02).
    pub fn state_key(&self, cx: &App) -> &'static str {
        match self.session.state() {
            _ if self.risk == Some(Risk::Macros) => "office.state.risk",
            _ if self.risk == Some(Risk::Signed) => "office.state.signed",
            _ if self.risk == Some(Risk::Repaired) => "office.state.repaired",
            _ if self.risk == Some(Risk::Sections) => "office.state.sections",
            _ if self.risk == Some(Risk::Sealed) => "office.state.sealed",
            _ if self.exporting => "office.state.saving",
            SaveState::Saving => "office.state.saving",
            SaveState::ExternalModified => "office.state.external",
            SaveState::SaveFailed => "office.state.failed",
            _ if self.dirty(cx) => "office.state.dirty",
            _ => "office.state.saved",
        }
    }
}

/// A continuation run once a tab's input lands.
pub type Waiting<H> = Box<dyn FnOnce(&mut H, &mut Window, &mut Context<H>)>;

/// The shared state of one window.
pub struct Core<H: Host> {
    pub worker: Arc<Mutex<WorkerOf<H>>>,
    pub sessions: Sessions,
    pub tabs: Vec<Tab<H::Extra, H::Engine>>,
    pub active: Option<u64>,
    /// Loads in flight: tab id and name.
    pub loading: Vec<(u64, SharedString)>,
    pub focus: FocusHandle,
    pub root: WeakEntity<Root>,
    pub work: PathBuf,
    pub next: u64,
    pub untitled: u32,
    pub dialog: Option<Dialog>,
    /// Locked files waiting: path, a password failed, a copy.
    pub locked: std::collections::VecDeque<(PathBuf, bool, bool)>,
    /// The window that async results update.
    pub window: Option<gpui::AnyWindowHandle>,
    /// Work that waits for a tab's queued input.
    pub waiting: Vec<(u64, Waiting<H>)>,
    /// Saves that wait for a tab's running export.
    pub queued_saves: Vec<(u64, Waiting<H>)>,
    /// The open find box.
    pub finder: Option<Finder>,
    /// Recovery points and crash return; none when off or failed.
    pub keeping: Option<Keeping>,
    /// Why recovery could not start, shown once.
    pub recovery_off: Option<String>,
    /// The engine ended under the open tabs (§6.3).
    pub stopped: Option<Stopped>,
    /// Ends a stuck worker; None when the engine cannot.
    pub stop: Option<Stop>,
    /// A load still unanswered after `stopped::SLOW`.
    pub slow: Option<u64>,
    /// Edit counts at the last autosave look, per tab.
    pub quiet: std::collections::HashMap<u64, u64>,
    /// Other programs' saves, answered as each ends.
    pub saving: Vec<(u64, ely_ui::Ask)>,
}

impl<H: Host> Core<H> {
    pub fn new(worker: Arc<Mutex<WorkerOf<H>>>, work: PathBuf, cx: &mut Context<H>) -> Self {
        // A worker just made holds no lock yet.
        let stop = H::Engine::stopper(&worker.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
        Core {
            worker,
            sessions: Sessions::default(),
            tabs: Vec::new(),
            active: None,
            loading: Vec::new(),
            focus: cx.focus_handle(),
            root: WeakEntity::new_invalid(),
            work,
            next: 0,
            untitled: 0,
            dialog: None,
            locked: Default::default(),
            window: None,
            waiting: Vec::new(),
            queued_saves: Vec::new(),
            finder: None,
            keeping: None,
            recovery_off: None,
            stopped: None,
            stop,
            slow: None,
            quiet: Default::default(),
            saving: Vec::new(),
        }
    }
}

/// An emptied scratch root under an app's data folder.
pub fn fresh_work(data: &std::path::Path) -> io::Result<PathBuf> {
    let work = data.join("work");
    if work.try_exists()? {
        std::fs::remove_dir_all(&work)?;
    }
    std::fs::create_dir_all(&work)?;
    Ok(work)
}

/// Runs `f` on the host inside its window.
pub fn in_window<H: Host, R>(
    this: &WeakEntity<H>,
    cx: &mut gpui::AsyncApp,
    f: impl FnOnce(&mut H, &mut Window, &mut Context<H>) -> R,
) -> anyhow::Result<R> {
    let handle = this
        .read_with(cx, |host, _| host.core().window)?
        .ok_or_else(|| anyhow::anyhow!("the documents have no window"))?;
    cx.update_window(handle, |_, window, cx| {
        this.update(cx, |host, cx| f(host, window, cx))
    })?
}

/// Opens the print panel after GPUI releases the app.
pub fn print_later<H: Tabs>(pdf: std::path::PathBuf, cx: &mut Context<H>) {
    let sides = ely_ui::settings::current(cx).two_sided;
    // The panel's modal loop calls back into GPUI.
    cx.spawn(async move |this, cx| {
        let printed = ely_platform::print::print_pdf(&pdf, sides).await;
        // The copy may hold what a password guarded.
        if let Err(error) = std::fs::remove_file(&pdf) {
            log::error!("office: the print copy {} stays: {error}", pdf.display());
        }
        match printed {
            Ok(printed) => log::info!("office: print done, printed {printed}"),
            Err(error) => {
                if this
                    .update(cx, |host, cx| host.print_failed(&error.to_string(), cx))
                    .is_err()
                {
                    log::error!("office: printing failed after its window closed: {error}");
                }
            }
        }
    })
    .detach();
}

/// Addresses a click may open outside: web and mail.
pub fn outward(uri: &str) -> bool {
    // Schemes ignore case.
    ["https://", "http://", "mailto:"]
        .iter()
        .any(|scheme| uri.get(..scheme.len()).is_some_and(|head| head.eq_ignore_ascii_case(scheme)))
}

/// A localized message.
pub fn text(cx: &App, key: &str, args: &[(&str, &str)]) -> SharedString {
    ely_ui::i18n::text(cx, key, args)
}

/// Tabs, toasts and queued input.
pub trait Tabs: Host {
    fn tab(&self, id: u64) -> Option<&Tab<Self::Extra, Self::Engine>> {
        self.core().tabs.iter().find(|tab| tab.id == id)
    }

    fn tab_mut(&mut self, id: u64) -> Option<&mut Tab<Self::Extra, Self::Engine>> {
        self.core_mut().tabs.iter_mut().find(|tab| tab.id == id)
    }

    fn active_tab(&self) -> Option<&Tab<Self::Extra, Self::Engine>> {
        self.core().active.and_then(|id| self.tab(id))
    }

    fn toast(&self, text: SharedString, severity: Severity, cx: &mut App) {
        log::warn!("{}: {text}", Self::KIND.app.slug());
        self.toast_unlogged(text, severity, cx);
    }

    fn print_failed(&self, error: &str, cx: &mut App) {
        let text = text(cx, "office.print.failed", &[("error", error)]);
        self.toast(text, Severity::Danger, cx);
    }

    /// A success toast with a button that opens `path`.
    fn toast_output(&self, text: SharedString, path: std::path::PathBuf, cx: &mut App) {
        log::info!("{}: {text}", Self::KIND.app.slug());
        if let Some(root) = self.core().root.upgrade() {
            root.update(cx, |root, cx| root.toast_output(text, Severity::Success, path, cx));
        }
    }

    /// A toast quoting document text; the caller logs a summary.
    fn toast_unlogged(&self, text: SharedString, severity: Severity, cx: &mut App) {
        if let Some(root) = self.core().root.upgrade() {
            root.update(cx, |root, cx| root.toast(text, severity, cx));
        }
    }

    /// A kernel failure as a toast.
    fn engine_failed(&self, error: &str, cx: &mut App) {
        let text = text(cx, "office.error.engine", &[("error", error)]);
        self.toast(text, Severity::Danger, cx);
    }

    /// Whether every input sent to `id` reached the kernel.
    fn settled(&self, id: u64, cx: &App) -> bool {
        self.tab(id)
            .is_none_or(|tab| Self::Engine::settled(tab.view.read(cx)))
    }

    /// Runs `then` once the queued input of `id` lands.
    fn when_settled(
        &mut self,
        id: u64,
        then: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) {
        log::info!("{}: tab {id} waits for queued input", Self::KIND.app.slug());
        self.core_mut().waiting.push((id, Box::new(then)));
    }

    /// Reads tab `id` in the background; `then` gets the window.
    fn read_in<T: Send + 'static>(
        &self,
        id: u64,
        read: impl FnOnce(&mut WorkerOf<Self>, &DocOf<Self>) -> io::Result<T> + Send + 'static,
        then: impl FnOnce(&mut Self, io::Result<T>, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tab(id) else {
            return;
        };
        let doc = self.read_at(id, cx).unwrap_or_else(|| tab.doc.clone());
        let worker = Arc::clone(&self.core().worker);
        cx.spawn(async move |this, cx| {
            let read = cx
                .background_executor()
                .spawn(async move { read(&mut worker.lock().expect("the worker lock"), &doc) })
                .await;
            in_window(&this, cx, |host, window, cx| then(host, read, window, cx))
        })
        .detach_and_log_err(cx);
    }

    /// A dialog's read; dropped once tab `id` is hidden.
    fn ask_in<T: Send + 'static>(
        &self,
        id: u64,
        read: impl FnOnce(&mut WorkerOf<Self>, &DocOf<Self>) -> io::Result<T> + Send + 'static,
        then: impl FnOnce(&mut Self, io::Result<T>, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        self.read_in(
            id,
            read,
            move |host, read, window, cx| {
                if host.core().active != Some(id) {
                    log::info!(
                        "{}: tab {id} left; its dialog is dropped",
                        Self::KIND.app.slug()
                    );
                    return;
                }
                then(host, read, window, cx)
            },
            cx,
        );
    }
}

impl<H: Host> Tabs for H {}

#[cfg(test)]
mod outward_tests {
    #[test]
    fn web_and_mail_go_out_in_any_case() {
        for uri in ["https://a.example/", "HTTPS://a.example/", "Http://a.example/", "MAILTO:x@a.example"] {
            assert!(super::outward(uri), "{uri}");
        }
        for uri in ["ftp://a.example/", "file:///etc/passwd", "javascript:alert(1)", "httpsx://a", "ht", "\u{e9}https://"] {
            assert!(!super::outward(uri), "{uri}");
        }
    }
}
