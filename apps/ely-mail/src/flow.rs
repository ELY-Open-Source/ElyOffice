//! Checking, signing in and adding an account (MAIL-01, 02).

use ely_gpui_component::primitives::Severity;
use ely_mail_engine::{Account, Checked, Error, Kept, Secret, oauth};
use gpui::{Context, SharedString, Window};

use crate::{
    adding::{Adding, Step},
    kinds::Kind,
    mail::{Mail, text},
};

impl Mail {
    pub fn start_adding(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.adding.is_none() && self.removing.is_none() {
            self.adding = Some(Adding::new(Kind::Gmail, window, cx));
        }
        cx.notify();
    }

    pub fn close_adding(&mut self, cx: &mut Context<Self>) {
        self.adding = None;
        cx.notify();
    }

    /// A field changed: an earlier check no longer holds.
    pub fn edited(&mut self, cx: &mut Context<Self>) {
        let Some(adding) = &mut self.adding else {
            return;
        };
        if !matches!(adding.step, Step::Editing | Step::Saving) {
            adding.stop();
            adding.step = Step::Editing;
            cx.notify();
        }
    }

    /// Starts a request for the open dialog.
    pub(crate) fn ask(&mut self) -> Option<u64> {
        let adding = self.adding.as_mut()?;
        adding.stop();
        self.asked += 1;
        adding.asked = Some(self.asked);
        Some(self.asked)
    }

    /// The open dialog, if it awaits `asked`.
    pub(crate) fn awaiting(&mut self, asked: u64) -> Option<&mut Adding> {
        self.adding
            .as_mut()
            .filter(|adding| adding.asked == Some(asked))
    }

    pub(crate) fn halt(&mut self, words: SharedString, cx: &mut Context<Self>) {
        if let Some(adding) = &mut self.adding {
            adding.stop();
            adding.step = Step::Failed(words);
        }
        cx.notify();
    }

    /// The words when the address is already here.
    fn taken(&self, address: &str, cx: &Context<Self>) -> Option<SharedString> {
        if let Some(id) = self.adding.as_ref().and_then(|adding| adding.again) {
            return self.not_its_own(id, address, cx);
        }
        self.accounts()
            .iter()
            .any(|(_, account)| account.address.eq_ignore_ascii_case(address))
            .then(|| text(cx, "mail.add.taken", &[("address", address)]))
    }

    /// Checks a typed-in password with both servers.
    pub fn check(&mut self, cx: &mut Context<Self>) {
        let Some(adding) = &self.adding else { return };
        let ready = adding
            .account(None, cx)
            .and_then(|account| Ok((account, adding.password(cx)?)));
        match ready {
            Ok((account, password)) => {
                self.probe(account, password.clone(), Secret::Password(password), cx)
            }
            Err(words) => self.halt(words, cx),
        }
    }

    /// Logs in with `secret`; `kept` goes to the vault after.
    fn probe(&mut self, account: Account, kept: String, secret: Secret, cx: &mut Context<Self>) {
        if let Some(words) = self.taken(&account.address, cx) {
            return self.halt(words, cx);
        }
        let tls = match &self.tls {
            Ok(tls) => tls.clone(),
            Err(error) => return self.halt(text(cx, "mail.error.tls", &[("error", error)]), cx),
        };
        let Some(asked) = self.ask() else { return };
        if let Some(adding) = &mut self.adding {
            adding.step = Step::Checking;
        }
        log::info!("mail: checking a {:?} account", account.protocol);
        let probe = account.clone();
        let work = async move { probe.check(secret, &tls).await };
        let task = self.on_runtime(cx, work, move |mail, done, cx| {
            mail.checked(asked, account, kept, done, cx)
        });
        if let Some(adding) = &mut self.adding {
            adding.task = Some(task);
        }
        cx.notify();
    }

    fn checked(
        &mut self,
        asked: u64,
        account: Account,
        kept: String,
        done: Result<Checked, String>,
        cx: &mut Context<Self>,
    ) {
        let Some(adding) = self.awaiting(asked) else {
            return log::info!("mail: a stale check dropped");
        };
        adding.task = None;
        // Signing in again keeps a passing token at once.
        let keep = adding.again.is_some() && done.as_ref().is_ok_and(Checked::passed);
        adding.step = match done {
            Ok(checked) => Step::Checked {
                account,
                kept,
                checked,
            },
            Err(error) => Step::Failed(text(cx, "mail.add.check_failed", &[("error", &error)])),
        };
        if keep {
            self.save(cx);
        }
    }

    /// Opens the provider's page, then checks what it grants.
    pub fn sign_in(&mut self, cx: &mut Context<Self>) {
        let Some(adding) = &self.adding else { return };
        let provider = match (self.providers)(adding.auth()) {
            Some(Ok(provider)) => provider,
            Some(Err(error)) => return self.halt(unsigned(&error, cx), cx),
            None => return log::error!("mail: sign-in asked of a password account"),
        };
        if let Err(words) = adding.servers(cx) {
            return self.halt(words, cx);
        }
        let hint = adding.fields.address.read(cx).text().trim().to_owned();
        let Some(asked) = self.ask() else { return };
        if let Some(adding) = &mut self.adding {
            adding.step = Step::Signing;
        }
        let work = async move { oauth::SignIn::start(provider, &hint).await };
        let task = self.on_runtime(cx, work, move |mail, started, cx| {
            if mail.awaiting(asked).is_none() {
                return log::info!("mail: a stale sign-in dropped");
            }
            match started.map_err(|error| text(cx, "mail.add.check_failed", &[("error", &error)])) {
                Ok(Ok(sign_in)) => mail.wait_for_browser(asked, sign_in, cx),
                Ok(Err(error)) => mail.halt(unsigned(&error, cx), cx),
                Err(words) => mail.halt(words, cx),
            }
        });
        if let Some(adding) = &mut self.adding {
            adding.task = Some(task);
        }
        cx.notify();
    }

    fn wait_for_browser(&mut self, asked: u64, sign_in: oauth::SignIn, cx: &mut Context<Self>) {
        cx.open_url(sign_in.url());
        log::info!("mail: the browser has the sign-in page");
        let work = async move {
            let tokens = sign_in.finish().await?;
            let email = tokens
                .email
                .clone()
                .ok_or_else(|| Error::SignIn("the provider named no address".into()))?;
            let refresh = tokens
                .refresh
                .clone()
                .ok_or_else(|| Error::SignIn("no refresh token granted".into()))?;
            Ok::<_, Error>((tokens.access, refresh, email))
        };
        let task = self.on_runtime(cx, work, move |mail, done, cx| {
            if mail.awaiting(asked).is_none() {
                return log::info!("mail: a stale sign-in dropped");
            }
            let account = match done {
                Ok(Ok((access, refresh, email))) => mail.adding.as_ref().map(|adding| {
                    adding
                        .account(Some(&email), cx)
                        .map(|account| (account, access, refresh))
                }),
                Ok(Err(error)) => Some(Err(unsigned(&error, cx))),
                Err(error) => Some(Err(text(cx, "mail.add.check_failed", &[("error", &error)]))),
            };
            match account {
                Some(Ok((account, access, refresh))) => {
                    mail.probe(account, refresh, Secret::Token(access), cx)
                }
                Some(Err(words)) => mail.halt(words, cx),
                None => {}
            }
        });
        if let Some(adding) = &mut self.adding {
            adding.task = Some(task);
        }
    }

    /// Stores the checked account, then its secret.
    pub fn save(&mut self, cx: &mut Context<Self>) {
        let Some(Adding {
            step:
                Step::Checked {
                    account,
                    kept,
                    checked,
                },
            ..
        }) = &self.adding
        else {
            return;
        };
        if !checked.passed() {
            return;
        }
        if let Some(words) = self.taken(&account.address, cx) {
            return self.halt(words, cx);
        }
        let (account, kept) = (account.clone(), kept.clone());
        let Some(asked) = self.ask() else { return };
        let again = match &mut self.adding {
            Some(adding) => {
                adding.step = Step::Saving;
                adding.again
            }
            None => None,
        };
        if let Some(id) = again {
            return self.keep_again(id, asked, kept, cx);
        }
        let vault = self.vault.clone();
        let work = move |store: &mut ely_mail_engine::Store| {
            let id = match store.add_account(&account) {
                Ok(id) => id,
                Err(Error::Taken(address)) => return Ok(Err(address)),
                Err(error) => return Err(error),
            };
            let held = vault.put(&account.secret_name(), &kept);
            Ok(Ok((id, account, held)))
        };
        self.with_store(cx, work, move |mail, saved, cx| match saved {
            Ok(Err(address)) => {
                let words = text(cx, "mail.add.taken", &[("address", &address)]);
                match mail.awaiting(asked) {
                    Some(_) => mail.halt(words, cx),
                    None => mail.toast(words, Severity::Danger, cx),
                }
            }
            Ok(Ok((id, account, held))) => {
                log::info!("mail: account {} added", id.0);
                let words = text(cx, "mail.add.added", &[("address", &account.address)]);
                if let Some(Ok(accounts)) = &mut mail.accounts {
                    accounts.push((id, account));
                }
                mail.open_box(crate::boxes::Open::Account(id), cx);
                mail.sync(id, cx);
                if mail.awaiting(asked).is_some() {
                    mail.adding = None;
                }
                mail.toast(words, Severity::Success, cx);
                if let Kept::Session(reason) = held {
                    mail.toast(
                        text(cx, "mail.add.session_only", &[("error", &reason)]),
                        Severity::Warning,
                        cx,
                    );
                }
            }
            Err(error) => {
                let words = text(cx, "mail.add.save_failed", &[("error", &error)]);
                match mail.awaiting(asked) {
                    Some(_) => mail.halt(words, cx),
                    None => mail.toast(words, Severity::Danger, cx),
                }
            }
        });
    }
}

/// Sign-in failures, with this build's missing client named.
fn unsigned(error: &Error, cx: &gpui::App) -> SharedString {
    match error {
        Error::Unavailable(name) => text(cx, &format!("mail.add.unavailable.{name}"), &[]),
        other => text(
            cx,
            "mail.add.sign_in_failed",
            &[("error", &other.to_string())],
        ),
    }
}
