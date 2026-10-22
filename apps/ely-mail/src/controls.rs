//! Sync now, pause, and folders kept whole.

use ely_gpui_component::{buttons::Button, feedback::Banner, forms::Switch, primitives::Severity};
use ely_mail_engine::{AccountId, LocalFolder, Protocol};
use gpui::{
    AnyElement, App, Context, InteractiveElement as _, IntoElement, ParentElement as _, SharedString, Styled as _,
    div,
};

use crate::mail::{Mail, text};

impl Mail {
    pub fn sync_buttons(&self, id: AccountId, cx: &mut Context<Self>) -> AnyElement {
        let running = self.syncs.get(&id).is_some_and(|each| each.task.is_some());
        let this = cx.weak_entity();
        let now = Button::new("mail-sync-now", text(cx, "mail.sync.now", &[]))
            .disabled(running || self.paused)
            .on_click({
                let this = this.clone();
                move |_, _, cx| {
                    this.update(cx, |mail, cx| mail.sync(id, cx)).ok();
                }
            });
        let key = if self.paused {
            "mail.sync.resume"
        } else {
            "mail.sync.pause"
        };
        let pause = Button::new("mail-sync-pause", text(cx, key, &[])).on_click(move |_, _, cx| {
            this.update(cx, |mail, cx| mail.set_paused(!mail.paused, cx))
                .ok();
        });
        div()
            .flex()
            .gap_2()
            .child(now)
            .child(pause)
            .into_any_element()
    }

    pub fn paused_banner(&self, cx: &Context<Self>) -> Option<Banner> {
        self.paused.then(|| {
            Banner::new(
                "mail-sync-paused",
                Severity::Info,
                text(cx, "mail.sync.paused", &[]),
            )
        })
    }

    /// One switch per IMAP folder.
    pub fn offline_switches(
        &self,
        id: AccountId,
        protocol: Protocol,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if protocol != Protocol::Imap {
            return None;
        }
        // Imported folders have no server to sync.
        let folders: Vec<_> = self.syncs.get(&id)?.folders.iter().filter(|folder| !folder.local).collect();
        if folders.is_empty() {
            return None;
        }
        let this = cx.weak_entity();
        let switches = folders.into_iter().map(|folder| {
            let mailbox = folder.id;
            let this = this.clone();
            let label = offline_label(folder, cx);
            let switch = Switch::new(("mail-offline", mailbox.0 as u64), folder.offline)
                .label(label)
                .on_change(move |on, _, cx| {
                    this.update(cx, |mail, cx| mail.set_offline(id, mailbox, on, cx))
                        .ok();
                });
            div()
                .debug_selector(move || format!("mail-offline-{}", mailbox.0))
                .child(switch)
        });
        Some(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .children(switches)
                .into_any_element(),
        )
    }
}

pub(crate) fn offline_label(folder: &LocalFolder, cx: &App) -> SharedString {
    text(cx, "mail.sync.offline", &[("name", &folder.shown)])
}
