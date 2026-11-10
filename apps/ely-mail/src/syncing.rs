//! Accounts sync on open, on demand and on a timer.

use std::{
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ely_mail_engine::{AccountId, Auth, Error, LocalFolder, MailboxId, Protocol, Synced, sync};
use gpui::Context;
use tokio::{sync::mpsc::UnboundedReceiver, task::AbortHandle};

use crate::mail::Mail;

/// How often every account syncs while the window is open.
pub const EVERY: Duration = Duration::from_secs(10 * 60);

/// One account's sync as the window shows it.
#[derive(Default)]
pub struct Syncing {
    pub task: Option<AbortHandle>,
    /// Which sync `task` is; older completions are stale.
    pub run: u64,
    /// A folder changed mid-sync: sync once more.
    pub again: bool,
    /// A change came mid-run: send the queue after.
    pub push: bool,
    pub synced: Option<Synced>,
    pub failed: Option<String>,
    /// The provider refused the token: sign in again.
    pub signed_out: bool,
    pub folders: Vec<LocalFolder>,
}

pub type Syncs = HashMap<AccountId, Syncing>;

impl Mail {
    pub fn sync(&mut self, id: AccountId, cx: &mut Context<Self>) {
        self.run_sync(id, false, cx);
    }

    /// Sends queued changes; a running sync sends them after.
    pub fn push(&mut self, id: AccountId, cx: &mut Context<Self>) {
        match self.syncs.get_mut(&id) {
            Some(syncing) if syncing.task.is_some() => {
                log::info!("mail: account {} sends its changes after this run", id.0);
                syncing.push = true;
            }
            _ => self.run_sync(id, true, cx),
        }
    }

    /// `only_changes` sends the queue, no mail comes down.
    fn run_sync(&mut self, id: AccountId, only_changes: bool, cx: &mut Context<Self>) {
        if self.paused {
            return log::info!("mail: account {} not synced: paused", id.0);
        }
        let Some((_, account)) = self.accounts().iter().find(|(each, _)| *each == id) else {
            return log::info!("mail: sync asked of a gone account");
        };
        if self.syncs.get(&id).is_some_and(|each| each.task.is_some()) {
            return log::info!("mail: account {} is already syncing", id.0);
        }
        let Ok(store) = self.store.clone() else {
            return log::error!("mail: no store to sync with");
        };
        let tls = match self.tls.clone() {
            Ok(tls) => tls,
            Err(error) => {
                log::warn!("mail: account {} cannot sync without TLS", id.0);
                let tls_failure = crate::mail::text(cx, "mail.error.tls", &[("error", &error)]).to_string();
                let syncing = self.syncs.entry(id).or_default();
                syncing.failed = Some(tls_failure);
                syncing.signed_out = false;
                return self.read_synced(id, cx);
            }
        };
        let (account, vault) = (account.clone(), self.vault.clone());
        let auth = account.auth;
        self.runs += 1;
        let run = self.runs;
        self.syncs.entry(id).or_default().run = run;
        let (told, steps) = tokio::sync::mpsc::unbounded_channel();
        self.follow_steps(id, run, steps, cx);
        let providers = self.providers.clone();
        let work = async move {
            let step = move || {
                if told.send(()).is_err() {
                    log::debug!("mail: a sync step after the window went");
                }
            };
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|err| ely_mail_engine::Error::Store(format!("the clock: {err}")))?
                .as_secs() as i64;
            // The first sync brings old mail, none of it new.
            let first = sync::lock(&store)?.synced(id)?.at.is_none();
            let secret = sync::secret(&account, providers(account.auth), &vault).await?;
            match (account.protocol, only_changes) {
                (Protocol::Imap, true) => sync::changes(&account, id, secret, &tls, &store)
                    .await
                    .map(|_| ())?,
                (Protocol::Imap, false) => {
                    sync::imap(&account, id, secret, &tls, &store, now, &step)
                        .await
                        .map(|_| ())?
                }
                // POP3 changes stay here.
                (Protocol::Pop { .. }, true) => (),
                (Protocol::Pop { .. }, false) => {
                    sync::pop(&account, id, secret, &tls, &store, now, &step)
                        .await
                        .map(|_| ())?
                }
            }
            let mut store = sync::lock(&store)?;
            let arrivals = match (only_changes, first) {
                (true, _) => None,
                (false, true) => store.skip_arrivals(id).map(|()| None)?,
                // Kept until handled, across failed syncs.
                (false, false) => Some(store.handle_arrivals(id)?),
            };
            Ok::<_, ely_mail_engine::Error>(arrivals)
        };
        let task = self.on_runtime(cx, work, move |mail, done, cx| {
            // Let go changes are told whatever broke after.
            mail.tell_dropped(id, cx);
            // Gmail's OAuth is gone; Microsoft signs in again.
            let signed_out = auth == Auth::Microsoft && matches!(done, Ok(Err(Error::SignInAgain(_))));
            let failed = match done {
                Ok(Ok(None)) => None,
                Ok(Ok(Some(arrivals))) => {
                    if !arrivals.refused.is_empty() {
                        mail.failed("mail.rules.refused", &arrivals.refused.join("; "), cx);
                    }
                    mail.announce(id, arrivals.arrived, cx);
                    // Rules' changes go to the server now.
                    if arrivals.ruled > 0
                        && let Some(syncing) = mail.syncs.get_mut(&id)
                    {
                        syncing.push = true;
                    }
                    None
                }
                // Worded as the sign-in check words it.
                Ok(Err(error)) => Some(crate::add_view::failure(&error, cx).to_string()),
                Err(error) => Some(error),
            };
            if failed.is_some() {
                log::warn!("mail: account {} did not sync", id.0);
            }
            // §18.1: what came down fits the current cap.
            if failed.is_none()
                && !only_changes
                && let Some(cap) = mail.cap_bytes()
            {
                mail.trim_to(cap, cx);
            }
            let Some(syncing) = mail.syncs.get_mut(&id) else {
                return log::info!("mail: a sync ended for a removed account");
            };
            if syncing.run != run {
                return log::info!("mail: account {} sync {run} ended after a newer one", id.0);
            }
            syncing.task = None;
            syncing.failed = failed;
            syncing.signed_out = signed_out;
            let again = std::mem::take(&mut syncing.again);
            let push = std::mem::take(&mut syncing.push);
            mail.read_synced(id, cx);
            if again {
                log::info!("mail: account {} syncs again for a changed folder", id.0);
                mail.sync(id, cx);
            } else if push {
                mail.push(id, cx);
            }
        });
        self.syncs.entry(id).or_default().task = Some(task);
        self.read_synced(id, cx);
    }

    /// Folders and mail show as the sync stores them.
    pub(crate) fn follow_steps(
        &self,
        id: AccountId,
        run: u64,
        mut steps: UnboundedReceiver<()>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            while steps.recv().await.is_some() {
                let shown = this.update(cx, |mail, cx| {
                    if mail.syncs.get(&id).is_some_and(|each| each.run == run) {
                        mail.read_synced(id, cx);
                    }
                });
                if shown.is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    pub fn sync_all(&mut self, cx: &mut Context<Self>) {
        // Sending is the user's own; pausing leaves it.
        self.run_outbox(true, cx);
        if self.paused {
            return log::info!("mail: sync is paused");
        }
        let ids: Vec<_> = self.accounts().iter().map(|(id, _)| *id).collect();
        for id in ids {
            self.sync(id, cx);
        }
    }

    /// Pausing stops running syncs; resuming syncs every account.
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        log::info!("mail: sync paused: {paused}");
        self.paused = paused;
        if paused {
            for syncing in self.syncs.values_mut() {
                syncing.again = false;
                syncing.push = false;
                if let Some(task) = syncing.task.take() {
                    task.abort();
                }
            }
        } else {
            self.sync_all(cx);
        }
        self.notify(cx);
    }

    /// Syncs every account each `EVERY` until the window goes.
    pub fn sync_on_a_timer(cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(EVERY).await;
                if this.update(cx, |mail, cx| mail.sync_all(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    /// Takes the folder's mail whole, then syncs to fetch it.
    pub fn set_offline(
        &mut self,
        id: AccountId,
        mailbox: MailboxId,
        offline: bool,
        cx: &mut Context<Self>,
    ) {
        self.with_store(
            cx,
            move |store| store.set_offline(mailbox, offline),
            move |mail, done, cx| match done {
                Ok(()) => {
                    mail.read_synced(id, cx);
                    match mail.syncs.get_mut(&id) {
                        Some(syncing) if offline && syncing.task.is_some() => {
                            log::info!("mail: account {} syncs again after this one", id.0);
                            syncing.again = true;
                        }
                        _ if offline => mail.sync(id, cx),
                        _ => {}
                    }
                }
                Err(error) => mail.failed("mail.sync.offline_failed", &error, cx),
            },
        );
    }

    /// Stops the account's sync and forgets it.
    pub fn stop_sync(&mut self, id: AccountId) {
        if let Some(task) = self.syncs.remove(&id).and_then(|each| each.task) {
            task.abort();
        }
    }

    pub fn read_synced(&mut self, id: AccountId, cx: &mut Context<Self>) {
        self.with_store(
            cx,
            move |store| Ok((store.synced(id)?, store.folders(id)?)),
            move |mail, read, cx| match (read, mail.syncs.get_mut(&id)) {
                (Ok((synced, folders)), Some(syncing)) => {
                    syncing.synced = Some(synced);
                    syncing.folders = folders;
                    mail.read_mail(cx);
                }
                (Err(error), Some(_)) => mail.failed("mail.sync.unread", &error, cx),
                (_, None) => {}
            },
        );
    }
}
