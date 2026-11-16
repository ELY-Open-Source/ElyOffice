//! What every window of one app shares (W.1a).

use std::{
    cell::{Ref, RefCell, RefMut},
    io,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};

use ely_files::Sessions;
use gpui::{App, Global, WeakEntity};

use crate::{Host, Stop, WorkerOf, recover::Keeping, stopped::Stopped};

/// One engine, one registry, one recovery for the app.
pub struct Shared<H: Host> {
    pub worker: Arc<Mutex<WorkerOf<H>>>,
    /// Ends a stuck worker; None when the engine cannot.
    pub stop: Option<Stop>,
    /// The engine ended under the open tabs (§6.3).
    pub stopped: Option<Stopped>,
    pub sessions: Sessions,
    /// Recovery points and crash return; none when off or failed.
    pub keeping: Option<Keeping>,
    /// Why recovery could not start, shown once.
    pub recovery_off: Option<String>,
    /// The first window started recovery.
    pub recovering: bool,
    pub work: PathBuf,
    /// The next tab id, across every window.
    pub next: u64,
    pub untitled: u32,
    /// Each window's documents, oldest first.
    pub hosts: Vec<WeakEntity<H>>,
}

/// The app's shared state, once a window holds it.
struct Slot<H: Host>(Rc<RefCell<Shared<H>>>);

impl<H: Host> Global for Slot<H> {}

/// A handle every window of the app holds.
pub struct Handle<H: Host>(Rc<RefCell<Shared<H>>>);

impl<H: Host> Clone for Handle<H> {
    fn clone(&self) -> Self {
        Handle(Rc::clone(&self.0))
    }
}

impl<H: Host> Handle<H> {
    /// The first window's: its engine, folder and stopper.
    pub(crate) fn first(worker: Arc<Mutex<WorkerOf<H>>>, stop: Option<Stop>, work: PathBuf, cx: &mut App) -> Self {
        let shared = Shared {
            worker,
            stop,
            stopped: None,
            sessions: Sessions::default(),
            keeping: None,
            recovery_off: None,
            recovering: false,
            work,
            next: 0,
            untitled: 0,
            hosts: Vec::new(),
        };
        let shared = Rc::new(RefCell::new(shared));
        cx.set_global(Slot(Rc::clone(&shared)));
        Handle(shared)
    }

    /// A later window's: what the first one started.
    pub fn joined(cx: &App) -> io::Result<Self> {
        let slot = cx.try_global::<Slot<H>>().ok_or_else(|| io::Error::other("no window of the app holds its documents"))?;
        Ok(Handle(Rc::clone(&slot.0)))
    }

    pub fn get(&self) -> Ref<'_, Shared<H>> {
        self.0.borrow()
    }

    pub fn get_mut(&self) -> RefMut<'_, Shared<H>> {
        self.0.borrow_mut()
    }

    /// This window, or the oldest still open.
    pub fn heir(&self, this: &WeakEntity<H>) -> Option<WeakEntity<H>> {
        if this.upgrade().is_some() {
            return Some(this.clone());
        }
        self.hosts().first().map(gpui::Entity::downgrade)
    }

    /// The host in `window`, else in the front window.
    pub fn front(&self, window: Option<gpui::AnyWindowHandle>, cx: &App) -> Option<gpui::Entity<H>> {
        let hosts = self.hosts();
        let front = window.or_else(|| ely_ui::tray::main(cx));
        let shown = hosts.iter().find(|host| front.is_some() && host.read(cx).core().window == front);
        shown.or(hosts.last()).cloned()
    }

    /// The windows still open, oldest first.
    pub fn hosts(&self) -> Vec<gpui::Entity<H>> {
        let mut shared = self.0.borrow_mut();
        shared.hosts.retain(|host| host.upgrade().is_some());
        shared.hosts.iter().filter_map(WeakEntity::upgrade).collect()
    }
}

#[cfg(test)]
mod tests;
