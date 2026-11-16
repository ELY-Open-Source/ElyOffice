//! The engine ended: say so, then reopen (§6.3).

use std::{path::PathBuf, sync::Arc, time::Duration};

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    feedback::{EmptyState, ErrorView},
    primitives::IconName,
};
use gpui::{AnyElement, AppContext as _, Context, IntoElement as _, TaskExt as _};

use crate::{Closing as _, Engine, Host, Opening as _, Recovering as _, recover::clock, text};

/// How long a closed tab's load may keep the engine.
pub(crate) const LET_GO: Duration = Duration::from_secs(2);

/// Why the engine ended, and whether it is coming back.
pub struct Stopped {
    why: String,
    restarting: bool,
}

pub trait Stopping: Host {
    /// After an engine error: asks if it ended.
    fn check_engine(&mut self, cx: &mut Context<Self>) {
        if self.core().shared.get().stopped.is_some() {
            return;
        }
        let worker = Arc::clone(&self.core().worker);
        cx.spawn(async move |this, cx| {
            let ended = cx
                .background_spawn(async move { Self::Engine::ended(&mut worker.lock().expect("the worker lock")) })
                .await;
            let Some(why) = ended else { return };
            let noted = this.update(cx, |host, cx| {
                if host.core().shared.get().stopped.is_none() {
                    log::error!("{}: the engine ended ({why})", Self::KIND.app.slug());
                    host.set_stopped(Some(Stopped { why, restarting: false }), cx);
                }
            });
            if noted.is_err() {
                log::info!("office: the engine ended after its window");
            }
        })
        .detach();
    }

    /// Swaps engines if closed load `id` outlasts `LET_GO`.
    fn watch_let_go(&mut self, id: u64, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(LET_GO).await;
            this.update(cx, |host, cx| {
                if host.core().holding(id) {
                    host.swap_engine(id, cx);
                }
            })
        })
        .detach_and_log_err(cx);
    }

    /// Ends the held engine; a new one reopens the tabs.
    fn swap_engine(&mut self, id: u64, cx: &mut Context<Self>) {
        let slug = Self::KIND.app.slug();
        let (stop, stopped) = {
            let shared = self.core().shared.get();
            (shared.stop.clone(), shared.stopped.is_some())
        };
        let Some(stop) = stop else {
            return log::warn!("{slug}: closed tab {id} still loads; this engine cannot stop");
        };
        if stopped {
            return log::info!("{slug}: closed tab {id} still loads; the engine is already restarting");
        }
        log::warn!("{slug}: closed tab {id} holds the engine after {} s; a new engine takes over", LET_GO.as_secs());
        let why = text(cx, "office.stopped.held", &[]).to_string();
        self.set_stopped(Some(Stopped { why, restarting: true }), cx);
        let shared = self.core().shared.clone();
        cx.spawn(async move |this, cx| {
            // The stuck load fails and frees the lock.
            if let Err(error) = cx.background_executor().spawn(async move { stop() }).await {
                log::error!("{slug}: the held engine did not stop: {error}");
            }
            // Any open window carries the restart on.
            let Some(heir) = shared.heir(&this) else {
                log::info!("{slug}: the engine stopped after every window");
                return Ok(());
            };
            heir.update(cx, |host, cx| host.relaunch_engine(cx))
        })
        .detach_and_log_err(cx);
    }

    /// No document shown: loading, empty or stopped.
    fn empty_body(&self, id: &'static str, icon: IconName, cx: &mut Context<Self>) -> AnyElement {
        let core = self.core();
        // A picked tab still loading takes the keys.
        let picked = core.loading.iter().find(|(of, ..)| Some(*of) == core.active());
        let title = match picked.or(core.loading.first()) {
            Some((_, name, _)) => text(cx, "office.loading", &[("name", name)]),
            None => text(cx, "command.blocked.no_document", &[]),
        };
        let body = self.stopped_page(cx).unwrap_or_else(|| EmptyState::new(id, icon, title).into_any_element());
        match picked {
            Some(_) => crate::hold::around(&self.core().hold, body, cx),
            None => body,
        }
    }

    /// The engine's end and way back, in the page's place.
    fn stopped_page(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (why, restarting, last) = {
            let shared = self.core().shared.get();
            let stopped = shared.stopped.as_ref()?;
            (stopped.why.clone(), stopped.restarting, shared.keeping.as_ref().and_then(|keeping| keeping.last))
        };
        let at = match last {
            Some(at) => clock(at),
            None => text(cx, "office.stopped.never", &[]).to_string(),
        };
        let said = text(cx, "office.stopped", &[("why", &why), ("at", &at)]);
        let page = ErrorView::new("office-stopped", IconName::CircleAlert, text(cx, "office.stopped.title", &[])).body(said);
        if restarting {
            return Some(page.into_any_element());
        }
        let this = cx.entity();
        let reopen = Button::new("office-stopped-reopen", text(cx, "office.stopped.reopen", &[]))
            .variant(ButtonVariant::Primary)
            .on_click(move |_, _, cx| this.update(cx, |host, cx| host.restart_engine(cx)));
        Some(page.action(reopen).into_any_element())
    }

    /// A new worker, then each tab from point or file.
    fn restart_engine(&mut self, cx: &mut Context<Self>) {
        {
            let mut shared = self.core().shared.get_mut();
            // Another window's click restarts it already.
            let Some(stopped) = shared.stopped.as_mut().filter(|stopped| !stopped.restarting) else { return };
            stopped.restarting = true;
        }
        self.notify_windows(cx);
        self.relaunch_engine(cx);
    }

    /// A new worker under the stopped one, then the tabs.
    fn relaunch_engine(&mut self, cx: &mut Context<Self>) {
        let worker = Arc::clone(&self.core().worker);
        let shared = self.core().shared.clone();
        cx.spawn(async move |this, cx| {
            let restarted = cx
                .background_spawn(async move {
                    let mut worker = worker.lock().expect("the worker lock");
                    Self::Engine::restart(Self::KIND.app, &mut worker).map(|()| Self::Engine::stopper(&worker))
                })
                .await;
            let Some(heir) = shared.heir(&this) else {
                return log::info!("office: the engine restarted after every window");
            };
            let reopened = heir.update(cx, |host, cx| match restarted {
                Ok(stop) => {
                    host.core().shared.get_mut().stop = stop;
                    host.set_stopped(None, cx);
                    // One engine: every window reopens on it.
                    let me = cx.entity_id();
                    for other in host.core().shared.hosts() {
                        if other.entity_id() != me {
                            other.update(cx, |other, cx| other.reopen_all(cx));
                        }
                    }
                    host.reopen_all(cx)
                }
                Err(error) => {
                    log::error!("{}: the engine does not restart: {error}", Self::KIND.app.slug());
                    host.set_stopped(Some(Stopped { why: error.to_string(), restarting: false }), cx);
                }
            });
            if reopened.is_err() {
                log::error!("office: the window to reopen in closed meanwhile");
            }
        })
        .detach();
    }

    /// Every window shows the engine's state.
    fn set_stopped(&mut self, stopped: Option<Stopped>, cx: &mut Context<Self>) {
        self.core().shared.get_mut().stopped = stopped;
        self.notify_windows(cx);
    }

    /// Redraws each window of the app.
    fn notify_windows(&self, cx: &mut Context<Self>) {
        for host in self.core().shared.hosts() {
            let app: &mut gpui::App = cx;
            app.notify(host.entity_id());
        }
    }

    /// Unsaved tabs from their newest point, clean ones from file.
    fn reopen_all(&mut self, cx: &mut Context<Self>) {
        let tabs: Vec<(u64, String, Option<PathBuf>, bool)> = self
            .core()
            .tabs
            .iter()
            .map(|tab| (tab.id, tab.name.to_string(), tab.session.path().map(PathBuf::from), tab.dirty(cx)))
            .collect();
        let (mut taken, mut lost) = (Vec::new(), Vec::new());
        for (id, name, path, dirty) in tabs {
            if !dirty {
                self.release_tab(id, cx);
                if let Some(path) = path {
                    self.open_file(path, cx);
                }
                continue;
            }
            match self.take_slot(id, name.clone()) {
                Some(slot) => taken.push(slot),
                None => lost.push(name),
            }
            self.release_tab(id, cx);
        }
        self.reopen_taken(taken, lost, cx);
    }
}

impl<H: Host> Stopping for H {}
