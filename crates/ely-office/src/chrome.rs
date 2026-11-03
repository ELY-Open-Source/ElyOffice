//! The tab strip and the save state (FILE-02).

use ely_gpui_component::{
    feedback::Banner,
    navigation::{EditorTab, EditorTabs},
    primitives::Severity,
};
use gpui::{Context, FocusHandle, SharedString, Window};

use crate::{After, Closing, Engine, Host, Tabs, text};

/// A tab switch: the page behind stops drawing; keys follow.
pub(crate) fn follow_front<H: Host>(host: &mut H, cx: &mut Context<H>) {
    let front = host.core().active;
    if host.core().shown.as_ref().map(|(id, _)| *id) == front {
        return;
    }
    let page = front.and_then(|id| host.tab(id)).map(|tab| <H::Engine as Engine>::focus(tab.view.read(cx)));
    let behind = std::mem::replace(&mut host.core_mut().shown, front.map(|id| (id, page.clone())));
    if let Some(tab) = behind.as_ref().and_then(|(id, _)| host.tab(*id)) {
        <H::Engine as Engine>::hide(&tab.view, cx);
    }
    if let Some(id) = front {
        host.tab_shown(id, cx);
    }
    if let Some(page) = page {
        hand_keys(host, behind.and_then(|(_, held)| held), page, cx);
    }
}

/// The front page takes the keys the page behind held.
fn hand_keys<H: Host>(host: &H, behind: Option<FocusHandle>, page: FocusHandle, cx: &mut Context<H>) {
    let Some(handle) = host.core().window else {
        return;
    };
    let root = host.core().focus.clone();
    let handed = handle.update(cx, |_, window, cx| {
        // Focus the user put elsewhere stays.
        let free = window.focused(cx).is_none_or(|focused| {
            focused == root || focused == page || behind.is_some_and(|old| old.contains_focused(window, cx))
        });
        if free {
            window.focus(&page, cx);
        }
        free
    });
    let slug = H::KIND.app.slug();
    match handed {
        Ok(true) => log::info!("{slug}: the front page has the keys"),
        Ok(false) => log::info!("{slug}: the keys stay where the user put them"),
        Err(error) => log::error!("{slug}: the keys could not follow the front: {error}"),
    }
}

pub trait Chrome: Host {
    fn tab_strip(&self, cx: &mut Context<Self>) -> EditorTabs {
        let this = cx.entity();
        let core = self.core();
        let open = core
            .tabs
            .iter()
            .map(|tab| EditorTab::new(tab.id.to_string(), tab.name.clone()).dirty(tab.dirty(cx)));
        let loading = core
            .loading
            .iter()
            .map(|(id, name)| EditorTab::new(id.to_string(), name.clone()).preview(true));
        let tabs = EditorTabs::new("office-tabs", open.chain(loading).collect::<Vec<_>>())
            .on_select({
                let this = this.clone();
                move |id, window, cx| {
                    let id = id.parse().expect("tab ids are numbers");
                    this.update(cx, |host, cx| host.show_tab(id, window, cx));
                }
            })
            .on_close(move |id, window, cx| {
                let id = id.parse().expect("tab ids are numbers");
                this.update(cx, |host, cx| {
                    host.request_close(id, After::Tab, window, cx)
                });
            });
        match core.active {
            Some(id) => tabs.selected(id.to_string()),
            None => tabs,
        }
    }

    /// The tab the user picked comes forward with the keys.
    fn show_tab(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.core_mut().active = Some(id);
        if let Some(tab) = self.active_tab() {
            log::info!("{}: tab {id} picked", Self::KIND.app.slug());
            window.focus(&<Self::Engine as Engine>::focus(tab.view.read(cx)), cx);
        }
        cx.notify();
    }

    /// The macro warning over a risky document (LOK-11).
    fn risk_banner(&self, cx: &mut Context<Self>) -> Option<Banner> {
        let tab = self.active_tab()?;
        let formats = Self::KIND.save_as.join("/").to_uppercase();
        let key = match tab.risk? {
            crate::Risk::Macros => "office.risk",
            crate::Risk::Signed => "office.signed",
            crate::Risk::Repaired => "office.repaired",
            crate::Risk::Sections => "office.sections",
            crate::Risk::Sealed => "office.sealed",
        };
        let warning = text(cx, key, &[("name", &tab.name), ("formats", &formats)]);
        Some(Banner::new("office-risk", Severity::Warning, warning))
    }

    /// The active tab's save state, or the load.
    fn save_state(&self, cx: &mut Context<Self>) -> SharedString {
        match (self.active_tab(), self.core().loading.first()) {
            (Some(tab), _) => text(cx, tab.state_key(cx), &[]),
            (None, Some((_, name))) => text(cx, "office.loading", &[("name", name)]),
            (None, None) => SharedString::default(),
        }
    }
}

impl<H: Host> Chrome for H {}
