//! Signing in again after a refused token (C12.6).

use ely_gpui_component::primitives::Severity;
use ely_mail_engine::{AccountId, Kept};
use gpui::{App, AppContext as _, Context, SharedString, Window};

use crate::{
    adding::Adding,
    mail::{Mail, text},
};

impl Mail {
    /// Opens the sign-in dialog for the account's refused token.
    pub fn sign_in_again(&mut self, id: AccountId, window: &mut Window, cx: &mut Context<Self>) {
        if self.adding.is_some() || self.removing.is_some() {
            return log::info!("mail: sign-in again waits for the open dialog");
        }
        let Some((_, account)) = self.accounts().iter().find(|(each, _)| *each == id) else {
            return log::info!("mail: sign-in again asked of a gone account");
        };
        log::info!("mail: account {} signs in again", id.0);
        let account = account.clone();
        self.adding = Some(Adding::again(id, &account, window, cx));
        cx.notify();
    }

    fn own_address(&self, id: AccountId) -> Option<&str> {
        self.accounts()
            .iter()
            .find(|(each, _)| *each == id)
            .map(|(_, account)| account.address.as_str())
    }

    pub(crate) fn again_title(&self, id: AccountId, cx: &App) -> SharedString {
        match self.own_address(id) {
            Some(address) => text(cx, "mail.again.title", &[("address", address)]),
            None => text(cx, "mail.again.gone", &[]),
        }
    }

    /// The words when `signed` is not the account's address.
    pub(crate) fn not_its_own(&self, id: AccountId, signed: &str, cx: &App) -> Option<SharedString> {
        match self.own_address(id) {
            Some(own) if own.eq_ignore_ascii_case(signed) => None,
            Some(own) => Some(text(cx, "mail.again.other", &[("address", own), ("signed", signed)])),
            None => Some(text(cx, "mail.again.gone", &[])),
        }
    }

    /// Keeps the new refresh token, then syncs.
    pub(crate) fn keep_again(&mut self, id: AccountId, asked: u64, kept: String, cx: &mut Context<Self>) {
        let Some((_, account)) = self.accounts().iter().find(|(each, _)| *each == id) else {
            let words = text(cx, "mail.again.gone", &[]);
            return self.halt(words, cx);
        };
        let (name, address) = (account.secret_name(), account.address.clone());
        let vault = self.vault.clone();
        let held = cx.background_spawn(async move { vault.put(&name, &kept) });
        cx.spawn(async move |this, cx| {
            let held = held.await;
            this.update(cx, |mail, cx| mail.kept_again(id, asked, &address, held, cx))
                .ok();
        })
        .detach();
    }

    fn kept_again(&mut self, id: AccountId, asked: u64, address: &str, held: Kept, cx: &mut Context<Self>) {
        log::info!("mail: account {} signed in again", id.0);
        if self.awaiting(asked).is_some() {
            self.adding = None;
        }
        self.toast(text(cx, "mail.again.done", &[("address", address)]), Severity::Success, cx);
        if let Kept::Session(reason) = held {
            let words = text(cx, "mail.add.session_only", &[("error", &reason)]);
            self.toast(words, Severity::Warning, cx);
        }
        match self.syncs.get_mut(&id) {
            Some(syncing) if syncing.task.is_some() => {
                log::info!("mail: account {} syncs again after this one", id.0);
                syncing.again = true;
            }
            _ => self.sync(id, cx),
        }
        self.notify(cx);
    }
}
