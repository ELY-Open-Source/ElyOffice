//! A live tab moves to another window (W.1c).

use ely_gpui_component::primitives::Severity;
use gpui::{Context, Window};

use crate::{Engine, Host, Opening as _, Tab, Tabs as _};

/// A tab between windows: its kernel document, file and points.
pub struct Moved<H: Host>(Tab<H::Extra, H::Engine>);

/// Why a tab stays where it is; an i18n key.
pub type Stays = &'static str;

pub trait Moving: Host {
    /// Takes tab `id` out of this window, unless busy.
    fn move_out(&mut self, id: u64, cx: &mut Context<Self>) -> Result<Moved<Self>, Stays> {
        let slug = Self::KIND.app.slug();
        let core = self.core();
        let Some(at) = core.tabs.iter().position(|tab| tab.id == id) else {
            log::info!("{slug}: tab {id} still loads; it stays");
            return Err("office.move.loading");
        };
        let ours = |of: &u64| *of == id;
        let waits = core.queued_saves.iter().chain(&core.waiting).any(|(of, _)| ours(of));
        let asks = core.saving.iter().chain(&core.asked).any(|(of, _)| ours(of));
        let tab = &core.tabs[at];
        let point = core.shared.get().keeping.as_ref().is_some_and(|keeping| keeping.writes(id));
        if tab.exporting || tab.pausing > 0 || waits || asks || point {
            log::info!("{slug}: tab {id} is saving; it stays");
            return Err("office.move.saving");
        }
        // Work started here ends here; the tab waits.
        if std::rc::Rc::strong_count(&tab.busy) > 1 || core.shared.get().stopped.is_some() {
            log::info!("{slug}: tab {id} has work running; it stays");
            return Err("office.move.busy");
        }
        if core.dialog.as_ref().and_then(crate::Dialog::tab) == Some(id) || self.asks_about(id) {
            log::info!("{slug}: tab {id} asks a question; it stays");
            return Err("office.move.dialog");
        }
        self.tab_leaving(id, cx);
        crate::hold::forget(self, id, cx);
        let core = self.core_mut();
        let mut tab = core.tabs.remove(at);
        core.quiet.remove(&id);
        core.closed_front(id);
        // Its old watch ends; the new window makes its own.
        tab.watching.clear();
        // Its images belong to this window's atlas.
        let mut images = Self::Engine::retire(&tab.view, cx);
        cx.spawn(async move |this, cx| {
            let freed = crate::in_window(&this, cx, |_, window, _| {
                window.on_next_frame(move |window, _| images.release_all(window));
            });
            if freed.is_err() {
                log::info!("office: the window went, and the moved tab's images with it");
            }
        })
        .detach();
        log::info!("{slug}: tab {id} leaves, {} tabs left", self.core().tabs.len());
        cx.notify();
        Ok(Moved(tab))
    }

    /// Takes in a moved tab, in front, with the keys.
    fn move_in(&mut self, moved: Moved<Self>, window: &mut Window, cx: &mut Context<Self>) {
        let Moved(mut tab) = moved;
        let id = tab.id;
        tab.watching = self.watch(id, &tab.view, cx);
        let at = self.core().tabs.partition_point(|open| open.id < id);
        self.core_mut().tabs.insert(at, tab);
        self.core_mut().set_active(Some(id));
        if let Some(tab) = self.tab(id) {
            window.focus(&Self::Engine::focus(tab.view.read(cx)), cx);
            // Its tiles draw again in this window.
            tab.view.update(cx, |_, cx| cx.notify());
        }
        // Shown, not opened: it keeps its place and state.
        self.tab_shown(id, cx);
        log::info!("{}: tab {id} arrives, {} tabs open", Self::KIND.app.slug(), self.core().tabs.len());
        cx.notify();
    }
}

impl<H: Host> Moving for H {}

/// Moves tab `id` to a new window; empty windows close.
pub fn to_new_window<H: Moving>(host: gpui::Entity<H>, id: u64, cx: &mut gpui::App) {
    // The asking window is busy until its action ends.
    cx.defer(move |cx| {
        let moved = match host.update(cx, |host, cx| host.move_out(id, cx)) {
            Ok(moved) => moved,
            Err(stays) => return host.update(cx, |host, cx| host.toast(crate::text(cx, stays, &[]), Severity::Warning, cx)),
        };
        let source = host.read(cx).core().window;
        let opened = ely_ui::windows::open(false, cx).map_err(|error| error.to_string());
        let target = opened.as_ref().ok().and_then(|window| crate::shared::Handle::<H>::joined(cx).ok()?.front(Some(*window), cx));
        let (window, home) = match (opened, target) {
            (Ok(window), Some(target)) => (window, target),
            (opened, _) => {
                let why = opened.err().unwrap_or_else(|| "the new window holds no documents".into());
                log::error!("{}: tab {id} stays; no new window: {why}", H::KIND.app.slug());
                let Some(window) = source else {
                    return log::error!("{}: tab {id} has no window to go back to", H::KIND.app.slug());
                };
                return back(window, host, moved, &why, cx);
            }
        };
        let landed = window.update(cx, |_, window, cx| home.update(cx, |home, cx| home.move_in(moved, window, cx)));
        if let Err(error) = landed {
            return log::error!("{}: the new window closed before tab {id} arrived: {error}", H::KIND.app.slug());
        }
        let Some(source) = source else { return };
        let core = host.read(cx).core();
        if core.tabs.is_empty() && core.loading.is_empty() {
            log::info!("{}: the window tab {id} left is empty; it closes", H::KIND.app.slug());
            source.update(cx, |_, window, _| window.remove_window()).ok();
            return;
        }
        // The tab left in front takes the keys.
        let focused = source.update(cx, |_, window, cx| {
            let focus = host.read(cx).active_tab().map(|tab| H::Engine::focus(tab.view.read(cx)));
            if let Some(focus) = focus {
                window.focus(&focus, cx);
            }
        });
        if let Err(error) = focused {
            log::warn!("{}: the window tab {id} left is gone: {error}", H::KIND.app.slug());
        }
    });
}

/// The tab goes back; the failure shows there.
fn back<H: Moving>(window: gpui::AnyWindowHandle, host: gpui::Entity<H>, moved: Moved<H>, why: &str, cx: &mut gpui::App) {
    let returned = window.update(cx, |_, window, cx| {
        host.update(cx, |host, cx| {
            host.move_in(moved, window, cx);
            host.toast(crate::text(cx, "office.move.failed", &[("error", why)]), Severity::Danger, cx);
        })
    });
    if let Err(error) = returned {
        log::error!("office: the tab's own window went too: {error}");
    }
}
