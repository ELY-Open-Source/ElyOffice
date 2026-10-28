//! The tab strip and the save state (FILE-02).

use ely_gpui_component::{
    feedback::Banner,
    navigation::{EditorTab, EditorTabs},
    primitives::Severity,
};
use gpui::{Context, SharedString};

use crate::{After, Closing, Host, Tabs, text};

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
                move |id, _, cx| {
                    let id = id.parse().expect("tab ids are numbers");
                    this.update(cx, |host, cx| {
                        host.core_mut().active = Some(id);
                        cx.notify();
                    });
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

    /// The macro warning over a risky document (LOK-11).
    fn risk_banner(&self, cx: &mut Context<Self>) -> Option<Banner> {
        let tab = self.active_tab()?;
        let formats = Self::KIND.save_as.join("/").to_uppercase();
        let key = match tab.risk? {
            crate::Risk::Macros => "office.risk",
            crate::Risk::Signed => "office.signed",
            crate::Risk::Repaired => "office.repaired",
            crate::Risk::Sections => "office.sections",
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
