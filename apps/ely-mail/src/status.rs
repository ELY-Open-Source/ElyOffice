//! The status bar: sync and a read-only store.

use ely_gpui_component::{
    overlays::AlertDialog,
    primitives::{IconName, Severity},
    shell::{StatusBar, StatusBarItem},
};
use gpui::{AnyElement, App, Context, IntoElement as _, Window};

use crate::{
    boxes::Open,
    mail::{Mail, text},
};

impl Mail {
    pub fn status_bar(&self, cx: &mut Context<Self>) -> StatusBar {
        let bar = StatusBar::new().left(self.sync_item(cx));
        match self.read_only_item(cx) {
            Some(item) => bar.left(item),
            None => bar,
        }
    }

    /// Paused, failed, syncing or up to date.
    fn sync_item(&self, cx: &mut Context<Self>) -> StatusBarItem {
        let item = StatusBarItem::new("mail-sync");
        let this = cx.weak_entity();
        if self.paused {
            return item
                .icon(IconName::CirclePause)
                .label(text(cx, "mail.status.paused", &[]))
                .tooltip(text(cx, "mail.sync.paused", &[]))
                .on_click(move |_, _, cx| {
                    this.update(cx, |mail, cx| mail.set_paused(false, cx)).ok();
                });
        }
        let failed = self.syncs.iter().find_map(|(id, each)| Some((*id, each.failed.as_ref()?)));
        if let Some((id, error)) = failed {
            return item
                .icon(IconName::CloudAlert)
                .label(text(cx, "mail.status.failed", &[]))
                .tooltip(text(cx, "mail.sync.failed", &[("error", error)]))
                .on_click(move |_, _, cx| {
                    this.update(cx, |mail, cx| mail.open_box(Open::Account(id), cx)).ok();
                });
        }
        if self.syncs.values().any(|each| each.task.is_some()) {
            return item.icon(IconName::RefreshCw).label(text(cx, "mail.sync.running", &[]));
        }
        item.icon(IconName::CloudCheck).label(text(cx, "mail.status.synced", &[]))
    }

    fn read_only_item(&self, cx: &mut Context<Self>) -> Option<StatusBarItem> {
        let reason = self.read_only.as_ref()?;
        let this = cx.weak_entity();
        let item = StatusBarItem::new("mail-read-only")
            .icon(IconName::TriangleAlert)
            .label(text(cx, "mail.status.read_only", &[]))
            .tooltip(text(cx, "mail.store.read_only", &[("error", reason)]))
            .on_click(move |_, _, cx| {
                this.update(cx, |mail, cx| {
                    mail.read_only_shown = true;
                    cx.notify();
                })
                .ok();
            });
        Some(item)
    }

    /// Said once at start; the bar keeps the mark.
    pub fn read_only_dialog(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let reason = self.read_only.as_ref().filter(|_| self.read_only_shown)?;
        let this = cx.weak_entity();
        let close = move |_: &mut Window, cx: &mut App| {
            this.update(cx, |mail, cx| {
                mail.read_only_shown = false;
                cx.notify();
            })
            .ok();
        };
        let dialog = AlertDialog::new(
            "mail-read-only",
            Severity::Warning,
            text(cx, "mail.store.read_only.title", &[]),
            text(cx, "mail.store.read_only", &[("error", reason)]),
            close,
        )
        .label(text(cx, "mail.ok", &[]));
        Some(dialog.into_any_element())
    }
}
