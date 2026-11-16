//! The tab strip and the save state (FILE-02).

use ely_gpui_component::{
    menus::{ContextMenu, Menu, MenuItem},
    navigation::{EditorTab, EditorTabs},
    primitives::{IconName, Severity},
    shell::StatusBarItem,
};
use gpui::{Context, SharedString, Window, actions};

use crate::{After, Closing, Engine, Host, Tabs, text};

actions!(office, [ShowRisk]);

/// A tab switch: the one behind stops drawing.
pub(crate) fn follow_front<H: Host>(host: &mut H, cx: &mut Context<H>) {
    // A waiting file shown, or asked for, loads now.
    crate::Opening::wake_due(host, cx);
    let front = host.core().active();
    let behind = std::mem::replace(&mut host.core_mut().shown, front);
    if behind == front {
        return;
    }
    if let Some(tab) = behind.and_then(|id| host.tab(id)) {
        <H::Engine as Engine>::hide(&tab.view, cx);
    }
    if let Some(id) = front {
        host.tab_shown(id, cx);
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
            .map(|(id, name, _)| EditorTab::new(id.to_string(), name.clone()).preview(true));
        let tabs = EditorTabs::new("office-tabs", open.chain(loading).collect::<Vec<_>>())
            .on_select({
                let this = this.clone();
                move |id, window, cx| {
                    let id = id.parse().expect("tab ids are numbers");
                    this.update(cx, |host, cx| host.show_tab(id, window, cx));
                }
            })
            .on_close({
                let this = this.clone();
                move |id, window, cx| {
                    let id = id.parse().expect("tab ids are numbers");
                    this.update(cx, |host, cx| host.request_close(id, After::Tab, window, cx));
                }
            });
        // Apps with more windows offer to move a tab.
        let tabs = match ely_ui::windows::allowed(cx) {
            true => {
                let this = this.downgrade();
                tabs.on_context(move |id, at, _, cx| {
                    let Ok(id) = id.parse() else { return log::error!("office: tab {id} is no number") };
                    this.update(cx, |host, cx| {
                        let last = host.core().tab_offer.as_ref();
                        host.core_mut().tab_offer = Some(ely_ui::menu::Offer::new(last, id, (), at));
                        cx.notify();
                    })
                    .ok();
                })
            }
            false => tabs,
        };
        match core.active() {
            Some(id) => tabs.selected(id.to_string()),
            None => tabs,
        }
    }

    /// The tab menu, while one is asked for.
    fn tab_menu(&self, cx: &mut Context<Self>) -> Option<ContextMenu> {
        let offer = self.core().tab_offer.as_ref()?;
        let (this, id) = (cx.entity(), offer.tab);
        let item = MenuItem::new(text(cx, "office.tab.new_window", &[])).on_click(move |_, cx| {
            crate::moving::to_new_window(this.clone(), id, cx);
        });
        Some(offer.host("office-tab-menu", Menu::new().item(item)))
    }

    /// The tab the user picked comes forward with the keys.
    fn show_tab(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.core_mut().set_active(Some(id));
        match self.active_tab() {
            Some(tab) => {
                log::info!("{}: tab {id} picked", Self::KIND.app.slug());
                window.focus(&<Self::Engine as Engine>::focus(tab.view.read(cx)), cx);
                crate::hold::replay(self, id, window, cx);
            }
            None if Self::Engine::PARKS => {
                log::info!("{}: tab {id} picked while it loads; its keys wait", Self::KIND.app.slug());
                self.core().hold.update(cx, |hold, cx| hold.take_keys(id, window, cx));
            }
            None => log::info!("{}: tab {id} picked while it loads", Self::KIND.app.slug()),
        }
        cx.notify();
    }

    /// What saving tab `id` in place would lose (LOK-11).
    fn risk_words(&self, id: u64, cx: &mut Context<Self>) -> Option<SharedString> {
        let tab = self.tab(id)?;
        let formats = Self::KIND.save_as.join("/").to_uppercase();
        let key = match tab.risk? {
            crate::Risk::Macros => "office.risk",
            crate::Risk::Signed => "office.signed",
            crate::Risk::Repaired => "office.repaired",
            crate::Risk::Sections => "office.sections",
            crate::Risk::Sealed => "office.sealed",
            crate::Risk::Links => "office.unlinked",
            crate::Risk::Unkept => "office.unkept",
        };
        Some(text(cx, key, &[("name", &tab.name), ("formats", &formats)]))
    }

    /// Asks before tab `id` is saved over its file.
    fn warn_risk(&mut self, id: u64, then: Option<After>, cx: &mut Context<Self>) {
        if self.core().dialog.is_some() {
            log::warn!("{}: tab {id}'s risk is not shown behind a dialog", Self::KIND.app.slug());
            if then.is_some() {
                self.close_stopped(cx);
            }
            return;
        }
        let Some(said) = self.risk_words(id, cx) else {
            return log::error!("{}: tab {id} has no risk to show", Self::KIND.app.slug());
        };
        log::info!("{}: tab {id}'s risk is shown", Self::KIND.app.slug());
        self.core_mut().dialog = Some(crate::Dialog::Risk { tab: id, then, said });
        cx.notify();
    }

    /// The menu's risk warning, on the active tab.
    fn on_show_risk(&mut self, _: &ShowRisk, _: &mut Window, cx: &mut Context<Self>) {
        match self.active_tab().map(|tab| (tab.id, tab.risk.is_some(), tab.name.clone())) {
            Some((id, true, _)) => self.warn_risk(id, None, cx),
            Some((_, false, name)) => self.toast(text(cx, "office.risk.none", &[("name", &name)]), Severity::Info, cx),
            None => self.toast(text(cx, "command.blocked.no_document", &[]), Severity::Warning, cx),
        }
    }

    /// The save state; a risky tab's opens its warning.
    fn state_item(&self, id: &'static str, cx: &mut Context<Self>) -> StatusBarItem {
        let item = StatusBarItem::new(id).label(self.save_state(cx));
        let Some(tab) = self.active_tab().filter(|tab| tab.risk.is_some()).map(|tab| tab.id) else {
            return item;
        };
        let this = cx.entity();
        item.icon(IconName::TriangleAlert)
            .on_click(move |_, _, cx| this.update(cx, |host, cx| host.warn_risk(tab, None, cx)))
    }

    /// The active tab's save state, or the load.
    fn save_state(&self, cx: &mut Context<Self>) -> SharedString {
        match (self.active_tab(), self.core().loading.first()) {
            (Some(tab), _) => text(cx, tab.state_key(cx), &[]),
            (None, Some((_, name, _))) => text(cx, "office.loading", &[("name", name)]),
            (None, None) => SharedString::default(),
        }
    }
}

impl<H: Host> Chrome for H {}
