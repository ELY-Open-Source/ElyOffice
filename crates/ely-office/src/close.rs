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
        match self.core().active {
            Some(id) => self.save(id, None, window, cx),
            None => self.no_document(cx),
        }
    }

    fn on_save_as(&mut self, _: &SaveAs, _: &mut Window, cx: &mut Context<Self>) {
        match self.core().active {
            Some(id) => self.save_as(id, None, cx),
            None => self.no_document(cx),
        }
    }

    fn on_close_tab(&mut self, _: &CloseTab, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.core().active {
            self.request_close(id, After::Tab, window, cx);
        }
    }

    /// Takes `window` as home; quit anywhere asks here first.
    fn own_window(&mut self, window: &Window, cx: &mut Context<Self>) {
        let handle = window.window_handle();
        self.core_mut().window = Some(handle);
        let host = cx.entity().downgrade();
        ely_ui::commands::route_quit(cx, move |cx| {
            let host = host.clone();
            // The asking window is busy until its action ends.
            cx.defer(move |cx| {
                let asked = handle.update(cx, |_, window, cx| {
                    // The question shows in front, with the keys.
                    window.activate_window();
                    host.update(cx, |host, cx| {
                        host.quitting(cx);
                        host.proceed(After::Quit, window, cx);
                    })
                });
                match asked {
                    Ok(Ok(())) => {}
                    // No home window is left to ask in.
                    _ if handle.update(cx, |_, _, _| ()).is_err() => cx.quit(),
                    Ok(Err(error)) | Err(error) => log::error!("office: the quit was not asked: {error}"),
                }
            });
        });
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
        cx.spawn(async move |this, cx| {
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
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        log::info!("office: closing tab {id} asked, then {after:?}");
        if !self.settled(id, cx) {
            return self.when_settled(id, move |host, window, cx| {
                host.request_close(id, after, window, cx)
            });
        }
        let Some(tab) = self.tab(id) else {
            return;
        };
        if tab.dirty(cx) {
            return self.ask_unsaved(id, after, cx);
        }
        // The undo history decides; it is read in the background.
        let edited = Self::Engine::edited(tab.view.read(cx));
        self.read_in(
            id,
            Self::Engine::history,
            move |host, read, window, cx| {
                let Some(tab) = host.tab(id) else {
                    return;
                };
                if Self::Engine::edited(tab.view.read(cx)) != edited {
                    return host.request_close(id, after, window, cx);
                }
                let moved = match read {
                    Ok(history) => history != tab.mark,
                    Err(error) => {
                        log::error!("office: the undo history of tab {id} is unreadable: {error}");
                        true
                    }
                };
                if moved || tab.dirty(cx) {
                    return host.ask_unsaved(id, after, cx);
                }
                host.close_tab(id, cx);
                host.proceed(after, window, cx);
            },
            cx,
        );
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
            After::Quit => cx.quit(),
            After::Tab => {}
        }
    }

    /// The close button: tabs close in turn, off the window.
    fn should_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.core().tabs.is_empty() {
            return true;
        }
        // Each tab's history is read in the background.
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
                .background_spawn(async move {
                    Self::Engine::close(&mut worker.lock().expect("the worker lock"), doc)
                })
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
        let core = self.core_mut();
        core.waiting.retain(|(waiting, _)| *waiting != id);
        if let Some(registry) = tab.registry {
            core.sessions.close(registry);
        }
        if core.active == Some(id) {
            core.active = core.tabs.last().map(|tab| tab.id);
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
