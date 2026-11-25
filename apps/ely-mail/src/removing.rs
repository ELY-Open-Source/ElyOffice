//! Removing an account: its local data and secret only.

use ely_gpui_component::primitives::Severity;
use ely_mail_engine::{AccountId, Cached, Error, Kept};
use gpui::Context;

use crate::mail::{Mail, text};

#[derive(Clone)]
pub struct Removing {
    pub id: AccountId,
    pub address: String,
    pub cached: Cached,
}

impl Mail {
    /// Measures what goes, then asks.
    pub fn ask_remove(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.open.account() else {
            let words = text(cx, "mail.remove.none", &[]);
            return self.toast(words, Severity::Info, cx);
        };
        self.with_store(
            cx,
            move |store| store.cached(id),
            move |mail, cached, cx| {
                let address = mail
                    .accounts()
                    .iter()
                    .find(|(each, _)| *each == id)
                    .map(|(_, account)| account.address.clone());
                match (cached, address) {
                    (Ok(cached), Some(address))
                        if mail.adding.is_none() && mail.removing.is_none() =>
                    {
                        mail.removing = Some(Removing {
                            id,
                            address,
                            cached,
                        });
                    }
                    (Ok(_), _) => {
                        log::info!("mail: removal asked of a gone account or over a dialog")
                    }
                    (Err(error), _) => mail.failed("mail.remove.failed", &error, cx),
                }
            },
        );
    }

    /// The secret goes first, and back if the rest fails.
    pub fn remove(&mut self, removing: Removing, cx: &mut Context<Self>) {
        let Some((_, account)) = self
            .accounts()
            .iter()
            .find(|(each, _)| *each == removing.id)
        else {
            return log::info!("mail: the account to remove is already gone");
        };
        let (id, name, vault) = (removing.id, account.secret.clone(), self.vault.clone());
        self.stop_sync(id);
        let address = removing.address;
        self.with_store(
            cx,
            move |store| {
                let secret = vault.get(&name)?;
                vault.remove(&name)?;
                store.remove_account(id).map_err(|error| {
                    match secret.map(|secret| vault.put(&name, &secret)) {
                        Some(Kept::Session(reason)) => Error::Vault(format!(
                            "{error}; its secret is back for this session only: {reason}"
                        )),
                        _ => error,
                    }
                })
            },
            move |mail, removed, cx| {
                match removed {
                    Ok(()) => {
                        log::info!("mail: account {} removed", id.0);
                        mail.toast(
                            text(cx, "mail.remove.done", &[("address", &address)]),
                            Severity::Success,
                            cx,
                        );
                    }
                    Err(error) => {
                        mail.failed("mail.remove.failed", &error, cx);
                        mail.sync(id, cx);
                    }
                }
                mail.read_accounts(cx);
            },
        );
        cx.notify();
    }
}
