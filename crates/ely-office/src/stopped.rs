//! The engine ended: say so, then reopen (§6.3).

use std::{path::PathBuf, sync::Arc, time::Duration};

use ely_gpui_component::{
    feedback::{Banner, EmptyState},
    primitives::{IconName, Severity},
};
use gpui::{
    AnyElement, AppContext as _, Context, IntoElement as _, ParentElement as _, Styled as _, TaskExt as _,
    div,
};

use crate::{Closing as _, Dialog, Engine, Host, Opening as _, Recovering as _, recover::clock, text};

/// How long a load runs before the stop is offered.
pub(crate) const SLOW: Duration = Duration::from_secs(30);

/// Why the engine ended, and whether it is coming back.
pub struct Stopped {
    why: String,
    restarting: bool,
}

pub trait Stopping: Host {
    /// After an engine error: asks if it ended.
    fn check_engine(&mut self, cx: &mut Context<Self>) {
        if self.core().stopped.is_some() {
            return;
        }
        let worker = Arc::clone(&self.core().worker);
        cx.spawn(async move |this, cx| {
            let ended = cx
                .background_spawn(async move { Self::Engine::ended(&mut worker.lock().expect("the worker lock")) })
                .await;
            let Some(why) = ended else { return };
            let noted = this.update(cx, |host, cx| {
                if host.core().stopped.is_none() {
                    log::error!("{}: the engine ended ({why})", Self::KIND.app.slug());
                    host.core_mut().stopped = Some(Stopped { why, restarting: false });
                    cx.notify();
                }
            });
            if noted.is_err() {
                log::info!("office: the engine ended after its window");
            }
        })
        .detach();
    }

    /// Offers the stop if load `id` still runs after `SLOW`.
    fn watch_slow(&mut self, id: u64, cx: &mut Context<Self>) {
        if self.core().stop.is_none() {
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SLOW).await;
            this.update(cx, |host, cx| host.slow_load(id, cx))
        })
        .detach_and_log_err(cx);
    }

    /// No document shown: loading or empty, under the engine's banner.
    fn empty_body(&self, id: &'static str, icon: IconName, cx: &mut Context<Self>) -> AnyElement {
        let title = match self.core().loading.first() {
            Some((_, name)) => text(cx, "office.loading", &[("name", name)]),
            None => text(cx, "command.blocked.no_document", &[]),
        };
        let empty = EmptyState::new(id, icon, title);
        let Some(banner) = self.stopped_banner(cx) else {
            return empty.into_any_element();
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(banner)
            .child(div().flex_1().min_h_0().child(empty))
            .into_any_element()
    }

    /// A load still running after `SLOW`: the stop is offered.
    fn slow_load(&mut self, id: u64, cx: &mut Context<Self>) {
        if !self.core().loading.iter().any(|(loading, _)| *loading == id) {
            return;
        }
        log::warn!("{}: tab {id} has not opened after {} s", Self::KIND.app.slug(), SLOW.as_secs());
        self.core_mut().slow = Some(id);
        cx.notify();
    }

    /// Ends the worker under a stuck load, which then fails.
    fn stop_engine(&mut self, cx: &mut Context<Self>) {
        self.core_mut().slow = None;
        cx.notify();
        let Some(stop) = self.core().stop.clone() else { return };
        log::warn!("{}: the user stops the engine under a slow load", Self::KIND.app.slug());
        // The stuck load then fails, which checks the engine.
        if let Err(error) = stop() {
            crate::Tabs::engine_failed(self, &error.to_string(), cx);
        }
    }

    /// A slow load and its stop.
    fn slow_banner(&self, cx: &mut Context<Self>) -> Option<Banner> {
        let id = self.core().slow?;
        let (_, name) = self.core().loading.iter().find(|(loading, _)| *loading == id)?;
        let seconds = SLOW.as_secs().to_string();
        let said = text(cx, "office.slow", &[("name", name), ("seconds", &seconds)]);
        let this = cx.entity();
        let stop = text(cx, "office.slow.stop", &[]);
        Some(Banner::new("office-slow", Severity::Warning, said).action(stop, move |_, cx| {
            this.update(cx, |host, cx| host.stop_engine(cx))
        }))
    }

    /// The engine's end and way back, or a slow load.
    fn stopped_banner(&self, cx: &mut Context<Self>) -> Option<Banner> {
        let Some(stopped) = self.core().stopped.as_ref() else {
            return self.slow_banner(cx);
        };
        let at = match self.core().keeping.as_ref().and_then(|keeping| keeping.last) {
            Some(at) => clock(at),
            None => text(cx, "office.stopped.never", &[]).to_string(),
        };
        let said = text(cx, "office.stopped", &[("why", &stopped.why), ("at", &at)]);
        let banner = Banner::new("office-stopped", Severity::Danger, said);
        if stopped.restarting {
            return Some(banner);
        }
        let this = cx.entity();
        Some(banner.action(text(cx, "office.stopped.reopen", &[]), move |_, cx| {
            this.update(cx, |host, cx| host.restart_engine(cx))
        }))
    }

    /// A new worker, then each tab from point or file.
    fn restart_engine(&mut self, cx: &mut Context<Self>) {
        let Some(stopped) = self.core_mut().stopped.as_mut() else { return };
        stopped.restarting = true;
        cx.notify();
        let worker = Arc::clone(&self.core().worker);
        cx.spawn(async move |this, cx| {
            let restarted = cx
                .background_spawn(async move {
                    let mut worker = worker.lock().expect("the worker lock");
                    Self::Engine::restart(Self::KIND.app, &mut worker).map(|()| Self::Engine::stopper(&worker))
                })
                .await;
            let reopened = this.update(cx, |host, cx| match restarted {
                Ok(stop) => {
                    host.core_mut().stop = stop;
                    host.reopen_all(cx)
                }
                Err(error) => {
                    log::error!("{}: the engine does not restart: {error}", Self::KIND.app.slug());
                    host.core_mut().stopped = Some(Stopped { why: error.to_string(), restarting: false });
                    cx.notify();
                }
            });
            if reopened.is_err() {
                log::info!("office: the engine restarted after its window");
            }
        })
        .detach();
    }

    /// Unsaved tabs from their newest point, clean ones from file.
    fn reopen_all(&mut self, cx: &mut Context<Self>) {
        self.core_mut().stopped = None;
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
        // Questions about the released tabs have no answer now.
        if matches!(self.core().dialog, Some(Dialog::Unsaved { .. } | Dialog::Conflict { .. })) {
            self.core_mut().dialog = None;
        }
        self.reopen_taken(taken, lost, cx);
    }
}

impl<H: Host> Stopping for H {}
