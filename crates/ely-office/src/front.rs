//! The front tab; files behind load when shown (R15.57).

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::{Core, Engine, Host, Source};

/// A load's source, kept while it waits.
type Kept<H> = Source<<<H as Host>::Engine as Engine>::Import>;

/// Loads not run because their tab is behind.
pub struct Parking<H: Host> {
    /// Each running load's flag, read once it has the engine.
    flags: Vec<(u64, Arc<AtomicBool>, bool)>,
    /// Loads that found themselves behind, with their source.
    parked: Vec<(u64, Kept<H>)>,
    /// Loads another program asked for.
    woken: Vec<u64>,
    /// Those programs' asks, put again once loaded.
    asks: Vec<(u64, ely_ui::Ask)>,
}

impl<H: Host> Default for Parking<H> {
    fn default() -> Self {
        Parking { flags: Vec::new(), parked: Vec::new(), woken: Vec::new(), asks: Vec::new() }
    }
}

impl<H: Host> Core<H> {
    /// The tab in front.
    pub fn active(&self) -> Option<u64> {
        self.active
    }

    /// Puts `tab` in front; pending loads stay behind.
    pub fn set_active(&mut self, tab: Option<u64>) {
        self.active = tab;
        self.front = None;
        self.refresh();
    }

    /// Load `id` landed in front; a newer load still may.
    pub(crate) fn landed_front(&mut self, id: u64) {
        self.active = Some(id);
        if self.front == Some(id) {
            self.front = None;
        }
        self.refresh();
    }

    /// Load `id` was asked last; it lands in front.
    pub(crate) fn asked_front(&mut self, id: u64) {
        self.front = Some(id);
        self.refresh();
    }

    /// Load `id` failed; true when the active tab moved.
    pub(crate) fn failed_front(&mut self, id: u64) -> bool {
        let chosen = self.active == Some(id);
        if chosen {
            self.active = self.tabs.last().map(|tab| tab.id);
        }
        if chosen || self.front == Some(id) {
            self.front = self.loading.last().map(|(id, ..)| *id);
        }
        self.refresh();
        chosen
    }

    /// Tab `id` closed; else the newest waiting tab comes.
    pub(crate) fn closed_front(&mut self, id: u64) {
        if self.active == Some(id) {
            self.active = self.tabs.last().map(|tab| tab.id);
        }
        if self.front == Some(id) {
            self.front = None;
        }
        if self.active.is_none() && self.front.is_none() {
            self.front = self.loading.last().map(|(id, ..)| *id);
        }
        self.refresh();
    }

    /// Whether load `id` takes the front when it lands.
    pub(crate) fn takes_front(&self, id: u64) -> bool {
        self.front == Some(id) || self.active.is_none() || self.active == Some(id)
    }

    /// Whether load `id` runs now rather than wait.
    fn wanted(&self, id: u64) -> bool {
        self.front == Some(id) || self.active == Some(id) || self.park.woken.contains(&id)
    }

    fn refresh(&self) {
        for (id, flag, parks) in &self.park.flags {
            flag.store(!parks || self.wanted(*id), Ordering::Release);
        }
    }

    /// The flag load `id` reads; only user files wait.
    pub fn flag(&mut self, id: u64, source: &Kept<H>) -> Arc<AtomicBool> {
        let parks = H::Engine::PARKS && matches!(source, Source::File(..));
        let flag = Arc::new(AtomicBool::new(false));
        self.park.flags.push((id, Arc::clone(&flag), parks));
        self.refresh();
        flag
    }

    /// Whether `flag` is still load `id`'s, not a later one's.
    pub(crate) fn running(&self, id: u64, flag: &Arc<AtomicBool>) -> bool {
        self.park.flags.iter().any(|(of, held, _)| *of == id && Arc::ptr_eq(held, flag))
    }

    /// Load `id` found its tab behind; it waits.
    pub(crate) fn park(&mut self, id: u64, source: Kept<H>) {
        self.park.flags.retain(|(of, ..)| *of != id);
        self.park.parked.push((id, source));
    }

    /// Waiting loads now wanted, with their sources.
    pub(crate) fn due(&mut self) -> Vec<(u64, Kept<H>)> {
        let (due, rest) = std::mem::take(&mut self.park.parked).into_iter().partition(|(id, _)| self.wanted(*id));
        self.park.parked = rest;
        due
    }

    /// Whether load `id` waits until shown.
    pub fn parked(&self, id: u64) -> bool {
        self.park.parked.iter().any(|(of, _)| *of == id)
    }

    /// The files waiting until shown.
    pub(crate) fn parked_files(&self) -> impl Iterator<Item = &std::path::Path> {
        self.park.parked.iter().filter_map(|(_, source)| match source {
            Source::File(path, ..) => Some(path.as_path()),
            _ => None,
        })
    }

    /// Another program asks for loading tab `id`.
    pub(crate) fn wake(&mut self, id: u64, ask: ely_ui::Ask) {
        if !self.park.woken.contains(&id) {
            self.park.woken.push(id);
        }
        self.park.asks.push((id, ask));
        self.refresh();
    }

    /// Load `id` ended; the asks waiting for it.
    pub(crate) fn ended(&mut self, id: u64) -> Vec<ely_ui::Ask> {
        self.park.flags.retain(|(of, ..)| *of != id);
        self.park.woken.retain(|of| *of != id);
        let (asks, rest) = std::mem::take(&mut self.park.asks).into_iter().partition(|(of, _)| *of == id);
        self.park.asks = rest;
        asks.into_iter().map(|(_, ask)| ask).collect()
    }

    /// Drops waiting load `id`; its registry id, if any.
    pub(crate) fn unpark(&mut self, id: u64) -> Option<Option<usize>> {
        let at = self.park.parked.iter().position(|(of, _)| *of == id)?;
        self.park.parked.remove(at);
        let at = self.loading.iter().position(|(of, ..)| *of == id)?;
        Some(self.loading.remove(at).2)
    }
}
