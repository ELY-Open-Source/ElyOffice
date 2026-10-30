//! What the window draws.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    feedback::{Banner, EmptyState, ErrorView},
    layout::ScrollArea,
    overlays::ConfirmDialog,
    primitives::{IconName, Severity},
    typography::format,
};
use ely_mail_engine::{AccountId, Auth, LocalFolder, Protocol, Security, Server, sync};
use ely_platform::AppId;
use ely_ui::{Shell, commands};
use gpui::{
    AnyElement, App, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, Window, div,
};

use crate::{
    boxes::Open,
    mail::{Mail, text},
    menu,
};

fn server(server: &Server, cx: &App) -> SharedString {
    let security = match server.security {
        Security::Tls => "TLS",
        Security::StartTls => "STARTTLS",
    };
    text(
        cx,
        "mail.account.server",
        &[
            ("host", &server.host),
            ("port", &server.port.to_string()),
            ("security", security),
        ],
    )
}

pub fn auth(auth: Auth, cx: &App) -> SharedString {
    let key = match auth {
        Auth::Password => "mail.auth.password",
        Auth::Google => "mail.auth.google",
        Auth::Microsoft => "mail.auth.microsoft",
    };
    text(cx, key, &[])
}

impl Mail {
    fn add_button(&self, cx: &App) -> Button {
        Button::new("mail-add", text(cx, "mail.add.open", &[]))
            .variant(ButtonVariant::Primary)
            .on_click(|_, window, cx| window.dispatch_action(Box::new(menu::AddAccount), cx))
    }

    fn body(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let error = |key: &str, detail: &str, cx: &App| {
            ErrorView::new("mail-error", IconName::TriangleAlert, text(cx, key, &[]))
                .detail(detail.to_owned())
                .into_any_element()
        };
        if let Err(detail) = &self.store {
            return error("mail.store.unopened", detail, cx);
        }
        let accounts = match &self.accounts {
            None => return div().into_any_element(),
            Some(Err(detail)) => return error("mail.store.unread", detail, cx),
            Some(Ok(accounts)) => accounts,
        };
        if self.open == Open::Contacts {
            return self.contacts_page(window, cx);
        }
        if accounts.is_empty() {
            return EmptyState::new("mail-empty", IconName::Mail, text(cx, "mail.empty", &[]))
                .action(self.add_button(cx))
                .into_any_element();
        }
        let Open::Account(open) = self.open else {
            return self.panes(cx);
        };
        let Some((id, account)) = accounts.iter().find(|(id, _)| *id == open) else {
            log::error!("mail: account {} open but not listed", open.0);
            return error("mail.store.unread", "", cx);
        };
        let label = |key: &str| text(cx, key, &[]);
        let incoming = format!(
            "{} · {}",
            protocol_name(account.protocol),
            server(&account.incoming, cx)
        );
        let details = DescriptionList::new()
            .text(label("mail.account.address"), account.address.clone())
            .text(label("mail.account.user"), account.user.clone())
            .text(label("mail.account.incoming"), incoming)
            .text(
                label("mail.account.outgoing"),
                server(&account.outgoing, cx),
            )
            .text(label("mail.account.auth"), auth(account.auth, cx));
        let (id, protocol) = (*id, account.protocol);
        let (details, failed) = self.sync_rows(id, protocol, details, cx);
        let remove = Button::new("mail-remove", label("mail.remove.open"))
            .on_click(|_, window, cx| window.dispatch_action(Box::new(menu::RemoveAccount), cx));
        let page = div()
            .p_6()
            .flex()
            .flex_col()
            .gap_4()
            .max_w_128()
            .children(self.paused_banner(cx))
            .children(failed)
            .child(details)
            .children(self.identity_editor(id, window, cx))
            .children(self.rules_editor(id, protocol, window, cx))
            .child(self.sync_buttons(id, cx))
            .children(self.offline_switches(id, protocol, cx))
            .child(self.import_button(id, cx))
            .children(self.drafts_switch(id, protocol, cx))
            .children(self.sent_copy_switch(id, account, cx))
            .child(div().child(remove));
        ScrollArea::new("mail-account-scroll")
            .vertical()
            .size_full()
            .child(page)
            .into_any_element()
    }

    /// When it last synced, what is here, why it failed.
    fn sync_rows(
        &self,
        id: AccountId,
        protocol: Protocol,
        details: DescriptionList,
        cx: &App,
    ) -> (DescriptionList, Option<Banner>) {
        let Some(syncing) = self.syncs.get(&id) else {
            return (details, None);
        };
        let label = |key: &str| text(cx, key, &[]);
        let last = match (&syncing.task, syncing.synced.and_then(|synced| synced.at)) {
            (Some(_), _) => label("mail.sync.running"),
            (None, Some(at)) => local_time(at).into(),
            (None, None) => label("mail.sync.never"),
        };
        let mut details = details.text(label("mail.sync.last"), last);
        if let Protocol::Pop { keep } = protocol {
            let server = label(if keep {
                "mail.sync.kept"
            } else {
                "mail.sync.deleted"
            });
            details = details
                .text(label("mail.sync.range"), label("mail.sync.range_pop"))
                .text(label("mail.sync.server"), server);
        } else {
            for (key, value) in imap_range(&syncing.folders, cx) {
                details = details.text(key, value);
            }
        }
        if let (Some(synced), Protocol::Imap) = (syncing.synced, protocol) {
            for (key, n) in [
                ("mail.sync.uncached", synced.uncached),
                ("mail.sync.partial", synced.partial),
            ] {
                let value = format!("{key}_value");
                let words = ely_ui::i18n::counted(cx, &value, n, &[("n", &n.to_string())]);
                details = details.text(label(key), words);
            }
            let waiting = match synced.waiting {
                0 => label("mail.sync.waiting_none"),
                n => {
                    let count = n.to_string();
                    ely_ui::i18n::counted(cx, "mail.sync.waiting_value", n, &[("n", &count)])
                }
            };
            details = details.text(label("mail.sync.waiting"), waiting);
        }
        let failed = syncing.failed.as_ref().map(|error| {
            Banner::new(
                "mail-sync-failed",
                Severity::Danger,
                text(cx, "mail.sync.failed", &[("error", error)]),
            )
        });
        (details, failed)
    }

    fn remove_dialog(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let removing = self.removing.as_ref()?;
        let this = cx.weak_entity();
        let dismiss = {
            let this = this.clone();
            move |_: &mut Window, cx: &mut App| {
                this.update(cx, |mail, cx| {
                    mail.removing = None;
                    cx.notify();
                })
                .ok();
            }
        };
        let size = format::file_size(removing.cached.bytes, false);
        let count = removing.cached.messages;
        let args = [
            ("address", removing.address.as_str()),
            ("n", &count.to_string()),
            ("size", &size),
        ];
        let message = ely_ui::i18n::counted(cx, "mail.remove.message", count, &args);
        let dialog = ConfirmDialog::new(
            "mail-remove",
            text(cx, "mail.remove.title", &args),
            message,
            dismiss,
        )
        .confirm(text(cx, "mail.remove.go", &[]))
        .destructive()
        .on_confirm({
            let removing = removing.clone();
            move |_, cx| {
                let removing = removing.clone();
                this.update(cx, |mail, cx| mail.remove(removing, cx)).ok();
            }
        });
        Some(dialog.into_any_element())
    }
}

/// What the rest download, then folders kept whole.
pub fn imap_range(folders: &[LocalFolder], cx: &App) -> Vec<(SharedString, SharedString)> {
    let synced = folders.iter().filter(|folder| !folder.local);
    let (whole, rest): (Vec<_>, Vec<_>) = synced.partition(|folder| folder.offline);
    let label = |key: &str| text(cx, key, &[]);
    let days = sync::BODY_DAYS.to_string();
    let mut rows = Vec::new();
    if whole.is_empty() || !rest.is_empty() {
        let key = match whole.is_empty() {
            true => "mail.sync.range_value",
            false => "mail.sync.range_rest",
        };
        rows.push((label("mail.sync.range"), text(cx, key, &[("days", &days)])));
    }
    if !whole.is_empty() {
        let sep = label("mail.sync.whole_sep");
        let names: Vec<&str> = whole.iter().map(|folder| folder.shown.as_str()).collect();
        let value = text(cx, "mail.sync.whole_value", &[("names", &names.join(&sep))]);
        rows.push((label("mail.sync.whole"), value));
    }
    rows
}

pub fn protocol_name(protocol: Protocol) -> &'static str {
    match protocol {
        Protocol::Imap => "IMAP",
        Protocol::Pop { .. } => "POP3",
    }
}

impl Render for Mail {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Contacts need no account, so the side shows once read.
        let left = matches!(self.accounts, Some(Ok(_))).then(|| self.side.clone().into());
        let read_only = self.read_only.as_ref().map(|reason| {
            Banner::new(
                "mail-read-only",
                Severity::Warning,
                text(cx, "mail.store.read_only", &[("error", reason)]),
            )
        });
        let shell = Shell::new(AppId::Mail.name(), &self.focus)
            .left(left)
            .children(read_only)
            .child(self.body(window, cx));
        div()
            .size_full()
            .key_context(menu::CONTEXT)
            .track_focus(&self.focus)
            .on_action(
                cx.listener(|mail, _: &menu::AddAccount, window, cx| mail.start_adding(window, cx)),
            )
            .on_action(cx.listener(|mail, _: &menu::RemoveAccount, _, cx| mail.ask_remove(cx)))
            .on_action(cx.listener(|mail, _: &menu::SyncAll, _, cx| mail.sync_all(cx)))
            .on_action(cx.listener(|mail, _: &commands::New, _, cx| mail.write_new(cx)))
            .on_action(cx.listener(|mail, _: &commands::Open, _, cx| mail.pick_eml(cx)))
            .child(shell)
            .children(self.add_dialog(cx))
            .children(self.remove_dialog(cx))
            .children(self.delete_dialog(cx))
            .children(self.erase_dialog(cx))
            .children(self.unimport_dialog(cx))
            .children(self.quit_dialog(cx))
    }
}

/// Local time to the minute.
pub fn local_time(at: i64) -> String {
    match jiff::Timestamp::from_second(at) {
        Ok(at) => at
            .to_zoned(jiff::tz::TimeZone::system())
            .strftime("%Y-%m-%d %H:%M")
            .to_string(),
        Err(error) => {
            log::error!("mail: a sync time out of range: {error}");
            at.to_string()
        }
    }
}
