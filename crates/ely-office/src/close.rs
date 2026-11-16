//! File actions, the system file panel, close and quit (FILE-03).

use std::path::PathBuf;

use ely_gpui_component::primitives::Severity;
use ely_ui::commands::{CloseTab, Open, Save, SaveAs};
use gpui::{App, AppContext as _, Context, PathPromptOptions, TaskExt as _, Window};

use crate::{Dialog, Engine, Host, Opening, Saving, Tabs, in_window, text};

/// What a finished close or save goes on to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum After {
    Tab,
    Window,
    Quit,
}

pub trait Closing: Host {
    fn on_open(&mut self, _: &Open, _: &mut Window, cx: &mut Context<Self>) {
        self.pick(false, cx, |host, path, cx| host.open_file(path, cx));
    }

    fn on_save(&mut self, _: &Save, window: &mut Window, cx: &mut Context<Self>) {
        match self.core().active() {
            Some(id) => self.save(id, None, window, cx),
            None => self.no_document(cx),
        }
    }

    fn on_save_as(&mut self, _: &SaveAs, window: &mut Window, cx: &mut Context<Self>) {
        match self.core().active() {
            Some(id) => self.save_as(id, None, window, cx),
            None => self.no_document(cx),
        }
    }

    fn on_close_tab(&mut self, _: &CloseTab, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.core().active() {
            self.request_close(id, After::Tab, window, cx);
        }
    }

    /// Takes `window` as home; quit asks each window in turn.
    fn own_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let handle = window.window_handle();
        self.core_mut().window = Some(handle);
        // Keys that fall to the root go to the front.
        let root = self.core().focus.clone();
        cx.on_focus(&root, window, |host, window, cx| {
            if let Some(tab) = host.active_tab() {
                log::info!("{}: the window had the keys; tab {} takes them", Self::KIND.app.slug(), tab.id);
                window.focus(&Self::Engine::focus(tab.view.read(cx)), cx);
            }
        })
        .detach();
        ely_ui::commands::route_quit(cx, ask_quit::<Self>);
    }

    /// A close or quit stopped: no update installs on it.
    fn close_stopped(&mut self, cx: &mut Context<Self>) {
        ely_ui::updates::quit_stopped(cx);
        self.close_cancelled(cx);
    }

    fn no_document(&self, cx: &mut Context<Self>) {
        let text = text(cx, "command.blocked.no_document", &[]);
        self.toast(text, Severity::Warning, cx);
    }

    /// A failed system panel is a failure, never a cancel.
    fn panel_failed(&self, error: &str, cx: &mut App) {
        let text = text(cx, "office.error.panel", &[("error", error)]);
        self.toast(text, Severity::Danger, cx);
    }

    /// Asks for one file, or one folder when `folder`.
    fn pick(
        &mut self,
        folder: bool,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, PathBuf, &mut Context<Self>) + 'static,
    ) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: !folder,
            directories: folder,
            multiple: false,
            prompt: None,
        });
        // The pick may serve the front tab: it stays.
        let busy = self.core().active().and_then(|id| self.hold(id));
        cx.spawn(async move |this, cx| {
            let _busy = busy;
            let picked = match picked.await {
                Ok(Ok(paths)) => Ok(paths.and_then(|paths| paths.into_iter().next())),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            in_window(&this, cx, |host, _, cx| match picked {
                Ok(Some(path)) => then(host, path, cx),
                Ok(None) => log::info!("office: the file panel was cancelled"),
                Err(error) => host.panel_failed(&error, cx),
            })
        })
        .detach_and_log_err(cx);
    }

    /// Closes `id`, asking first when it holds unsaved changes.
    fn request_close(
        &mut self,
        id: u64,
        after: After,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        log::info!("office: closing tab {id} asked, then {after:?}");
        let Some(tab) = self.tab(id) else {
            self.close_loading(id, cx);
            return self.proceed(after, window, cx);
        };
        // The app's own record decides; the engine is not asked.
        if tab.dirty(cx) {
            return self.ask_unsaved(id, after, cx);
        }
        self.close_tab(id, cx);
        self.proceed(after, window, cx);
    }

    fn ask_unsaved(&mut self, id: u64, after: After, cx: &mut Context<Self>) {
        self.core_mut().dialog = Some(Dialog::Unsaved { tab: id, after });
        cx.notify();
    }

    /// Window and quit close each tab, asking for dirty ones.
    fn proceed(&mut self, after: After, window: &mut Window, cx: &mut Context<Self>) {
        if after == After::Tab {
            return;
        }
        if let Some(id) = self.core().tabs.first().map(|tab| tab.id) {
            return self.request_close(id, after, window, cx);
        }
        match after {
            After::Window => window.remove_window(),
            // This window is done; the next one asks.
            After::Quit => ask_quit::<Self>(cx),
            After::Tab => {}
        }
    }

    /// The close button: tabs close in turn, off the window.
    fn should_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.core().tabs.is_empty() {
            return true;
        }
        // Unsaved tabs ask first.
        self.proceed(After::Window, window, cx);
        false
    }

    /// Releases the kernel document, registry entry and scratch folder.
    fn close_tab(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(doc) = self.release_tab(id, cx) else {
            return;
        };
        // The kernel lets go in the background.
        let worker = std::sync::Arc::clone(&self.core().worker);
        cx.spawn(async move |this, cx| {
            let closed = cx
                .background_spawn(async move { ely_lok::behind(&worker, |worker| Self::Engine::close(worker, doc)).await })
                .await;
            if let Err(error) = closed
                && this
                    .update(cx, |host, cx| host.engine_failed(&error.to_string(), cx))
                    .is_err()
            {
                log::error!("office: closing a document failed after its window: {error}");
            }
        })
        .detach();
    }

    /// Drops loading tab `id`; a stuck load swaps engines.
    fn close_loading(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some((registry, running, asks)) = self.core_mut().drop_load(id) else {
            return log::info!("office: tab {id} is not open nor loading");
        };
        for ask in asks {
            ask.answer(Err(format!("tab {id} closed while it loaded")));
        }
        if running {
            crate::Stopping::watch_let_go(self, id, cx);
        }
        crate::hold::forget(self, id, cx);
        let core = self.core_mut();
        core.closed_front(id);
        if let Some(registry) = registry {
            core.shared.get_mut().sessions.close(registry);
        }
        log::info!("office: closed loading tab {id}, {} tabs open, {} waiting", core.tabs.len(), core.loading.len());
        cx.notify();
    }

    /// Drops `id` from the window; its kernel document is returned.
    fn release_tab(&mut self, id: u64, cx: &mut Context<Self>) -> Option<crate::DocOf<Self>> {
        let core = self.core_mut();
        let at = core.tabs.iter().position(|tab| tab.id == id)?;
        let tab = core.tabs.remove(at);
        self.tab_closed(&tab, cx);
        // Its images leave GPUI's atlas next frame (PERF-08).
        let mut images = Self::Engine::retire(&tab.view, cx);
        cx.spawn(async move |this, cx| {
            let freed = crate::in_window(&this, cx, |_, window, _| {
                window.on_next_frame(move |window, _| images.release_all(window));
            });
            if freed.is_err() {
                log::info!("office: the window went, and its images with it");
            }
        })
        .detach();
        crate::Recovering::forget_tab(self, id, cx);
        crate::hold::forget(self, id, cx);
        let core = self.core_mut();
        core.waiting.retain(|(waiting, _)| *waiting != id);
        let (asks, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut core.asked).into_iter().partition(|(tab, _)| *tab == id);
        core.asked = rest;
        for (_, ask) in asks {
            ask.answer(Err(format!("tab {id} closed before its save")));
        }
        if let Some(registry) = tab.registry {
            core.shared.get_mut().sessions.close(registry);
        }
        core.closed_front(id);
        if core.dialog.as_ref().and_then(Dialog::tab) == Some(id) {
            log::info!("office: tab {id}'s question closes with it");
            core.dialog = None;
            // A locked file waiting behind it asks now.
            cx.spawn(async move |this, cx| in_window(&this, cx, |host, window, cx| {
                if host.core().dialog.is_none() {
                    crate::Dialogs::dialog_done(host, window, cx);
                }
            }))
            .detach_and_log_err(cx);
        }
        if let Err(error) = std::fs::remove_dir_all(&tab.work) {
            log::warn!("office: scratch {} stays: {error}", tab.work.display());
        }
        log::info!("office: closed {}, {} tabs open", tab.name, self.core().tabs.len());
        cx.notify();
        Some(tab.doc)
    }
}

impl<H: Host> Closing for H {}

/// The next window with tabs asks; else the app quits.
fn ask_quit<H: Host>(cx: &mut App) {
    // The asking window is busy until its action ends.
    cx.defer(|cx| {
        let Ok(shared) = crate::shared::Handle::<H>::joined(cx) else { return cx.quit() };
        let next = shared.hosts().into_iter().find_map(|host| {
            let core = host.read(cx).core();
            core.window.filter(|_| !core.tabs.is_empty()).map(|handle| (host.clone(), handle))
        });
        let Some((host, handle)) = next else {
            log::info!("{}: every window answered; quitting", H::KIND.app.slug());
            return cx.quit();
        };
        let asked = handle.update(cx, |_, window, cx| {
            // The question shows in front, with the keys.
            window.activate_window();
            host.update(cx, |host, cx| {
                host.quitting(cx);
                host.proceed(After::Quit, window, cx);
            })
        });
        if let Err(error) = asked {
            // Its window is gone, and its tabs with it.
            log::error!("office: the quit was not asked: {error}");
            cx.quit();
        }
    });
}
