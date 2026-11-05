//! The outbox: queued messages go one by one (C12.14).

use std::collections::VecDeque;

use ely_gpui_component::primitives::Severity;
use ely_mail_engine::{
    Account, AccountId, DraftId, Error, Filed, Handed, Landing, OutboxId, Protocol, Role, Unsent,
    sent, sync, without_bcc,
};
use gpui::{AppContext as _, Context, WeakEntity};

use crate::{
    compose::Compose,
    mail::{Mail, text},
    writing::{Outgoing, prepare},
};

/// One send or filing at a time.
#[derive(Default)]
pub struct Runner {
    pub busy: bool,
    /// Asked while busy; true takes the unreached too.
    again: Option<bool>,
    /// The message being handed over now.
    pub sending: Option<OutboxId>,
}

/// What a round still has to do.
struct Round {
    send: VecDeque<OutboxId>,
    file: VecDeque<OutboxId>,
    /// A retry round says nothing of the unreached.
    retry: bool,
}

impl Mail {
    /// Builds it, then queues it with its draft.
    pub fn queue_message(
        &mut self,
        from: AccountId,
        draft: Option<DraftId>,
        unsent: Unsent,
        out: Outgoing,
        compose: WeakEntity<Compose>,
        cx: &mut Context<Self>,
    ) {
        let files = out.attachments.clone();
        let built = cx.background_spawn(async move { prepare(&out) });
        cx.spawn(async move |this, cx| {
            let built = built.await;
            this.update(cx, |mail, cx| {
                let built = match built {
                    Ok(built) => built,
                    Err(error) => return queued(compose, Err(error.to_string()), cx),
                };
                mail.with_store(
                    cx,
                    move |store| store.enqueue(from, draft, &unsent, &files, &built),
                    move |mail, done, cx| {
                        let done = done.map(|(_, left)| {
                            if let Some(left) = left {
                                mail.take_copy(left, cx);
                            }
                            mail.pending += 1;
                            mail.run_outbox(false, cx);
                        });
                        queued(compose, done, cx);
                        mail.drafts_changed(cx);
                        mail.read_mail(cx);
                    },
                );
            })
            .ok();
        })
        .detach();
    }

    /// `retry` takes the unreached and waiting copies too.
    pub fn run_outbox(&mut self, retry: bool, cx: &mut Context<Self>) {
        if self.runner.busy {
            self.runner.again = Some(self.runner.again.unwrap_or(false) || retry);
            return;
        }
        self.runner.busy = true;
        self.with_store(
            cx,
            move |store| {
                let file = match retry {
                    true => store.to_file()?,
                    false => Vec::new(),
                };
                Ok((store.to_send(retry)?, file))
            },
            move |mail, read, cx| match read {
                Ok((send, file)) => {
                    log::info!("mail: outbox round: {} to send", send.len());
                    let round = Round {
                        send: send.into(),
                        file: file.into(),
                        retry,
                    };
                    mail.next_out(round, cx);
                }
                Err(error) => {
                    mail.failed("mail.outbox.unread", &error, cx);
                    mail.ran(cx);
                }
            },
        );
    }

    fn next_out(&mut self, mut round: Round, cx: &mut Context<Self>) {
        if let Some(id) = round.send.pop_front() {
            return self.send_one(id, round, cx);
        }
        if let Some(id) = round.file.pop_front() {
            return self.file_one(id, round, cx);
        }
        self.ran(cx);
    }

    fn ran(&mut self, cx: &mut Context<Self>) {
        self.runner.busy = false;
        self.read_mail(cx);
        if let Some(retry) = self.runner.again.take() {
            self.run_outbox(retry, cx);
        }
    }

    fn send_one(&mut self, id: OutboxId, round: Round, cx: &mut Context<Self>) {
        self.with_store(
            cx,
            move |store| {
                let Some(handed) = store.take_queued(id)? else {
                    return Ok(None);
                };
                let copy = store.sent_copy(handed.account)?;
                Ok(Some((handed, copy)))
            },
            move |mail, read, cx| match read {
                Ok(None) => mail.next_out(round, cx),
                Ok(Some((handed, copy))) => {
                    mail.read_mail(cx);
                    mail.hand_over(id, handed, copy, round, cx);
                }
                Err(error) => {
                    mail.failed("mail.outbox.unsaved", &error, cx);
                    mail.next_out(round, cx);
                }
            },
        );
    }

    fn hand_over(
        &mut self,
        id: OutboxId,
        handed: Handed,
        copy: Option<bool>,
        round: Round,
        cx: &mut Context<Self>,
    ) {
        let from = handed.account;
        let (account, tls) = match self.reach(from) {
            Ok(reach) => reach,
            Err(error) => return self.land(id, from, Landing::Failed(error), None, round, cx),
        };
        let file = copies(&account, copy);
        let vault = self.vault.clone();
        self.runner.sending = Some(id);
        let providers = self.providers.clone();
        let work = async move {
            let bytes = without_bcc(&handed.bytes)?;
            let secret = sync::secret(&account, providers(account.auth), &vault).await?;
            account.send(secret, &handed.envelope, &bytes, &tls).await
        };
        self.on_runtime(cx, work, move |mail, done, cx| {
            mail.runner.sending = None;
            let (landing, limit) = landing(done, file);
            mail.land(id, from, landing, limit, round, cx);
        });
    }

    fn land(
        &mut self,
        id: OutboxId,
        from: AccountId,
        landing: Landing,
        limit: Option<Option<u64>>,
        mut round: Round,
        cx: &mut Context<Self>,
    ) {
        let told = say(&landing, round.retry);
        let file = matches!(landing, Landing::Accepted { file: true });
        self.with_store(
            cx,
            move |store| {
                store.landed(id, landing)?;
                match limit {
                    Some(limit) => store.set_smtp_limit(from, limit),
                    None => Ok(()),
                }
            },
            move |mail, done, cx| {
                match done {
                    Ok(()) => {
                        if let Some((key, severity, error)) = told {
                            let words = text(cx, key, &[("error", &error)]);
                            mail.toast(words, severity, cx);
                        }
                        if file {
                            round.file.push_front(id);
                        }
                    }
                    Err(error) => mail.failed("mail.outbox.unsaved", &error, cx),
                }
                mail.read_mail(cx);
                mail.next_out(round, cx);
            },
        );
    }

    fn file_one(&mut self, id: OutboxId, round: Round, cx: &mut Context<Self>) {
        self.with_store(
            cx,
            move |store| {
                let Some((account, bytes)) = store.filing(id)? else {
                    return Ok(None);
                };
                let sent = store.folders(account)?.into_iter();
                let mailbox = sent
                    .filter(|folder| folder.role == Some(Role::Sent))
                    .map(|folder| folder.name)
                    .next();
                Ok(Some((account, bytes, mailbox)))
            },
            move |mail, read, cx| match read {
                Ok(None) => mail.next_out(round, cx),
                Ok(Some((_, _, None))) => {
                    let words = text(cx, "mail.outbox.no_sent_folder", &[]).to_string();
                    mail.filed_as(id, Filed::Failed(words), round, cx);
                }
                Ok(Some((account, bytes, Some(mailbox)))) => {
                    mail.file_copy(id, account, bytes, mailbox, round, cx);
                }
                Err(error) => {
                    mail.failed("mail.outbox.unsaved", &error, cx);
                    mail.next_out(round, cx);
                }
            },
        );
    }

    fn file_copy(
        &mut self,
        id: OutboxId,
        from: AccountId,
        bytes: Vec<u8>,
        mailbox: String,
        round: Round,
        cx: &mut Context<Self>,
    ) {
        let (account, tls) = match self.reach(from) {
            Ok(reach) => reach,
            Err(error) => return self.filed_as(id, Filed::Failed(error), round, cx),
        };
        let vault = self.vault.clone();
        let providers = self.providers.clone();
        let work = async move {
            let secret = sync::secret(&account, providers(account.auth), &vault).await?;
            sent::file(&account, secret, &tls, &mailbox, &bytes).await
        };
        self.on_runtime(cx, work, move |mail, done, cx| {
            mail.filed_as(id, filed(done), round, cx);
        });
    }

    fn filed_as(&mut self, id: OutboxId, filed: Filed, round: Round, cx: &mut Context<Self>) {
        let told = say_filed(&filed, round.retry);
        self.with_store(
            cx,
            move |store| store.filed(id, filed),
            move |mail, done, cx| {
                match (done, told) {
                    (Ok(()), Some((key, severity, error))) => {
                        let words = text(cx, key, &[("error", &error)]);
                        mail.toast(words, severity, cx);
                    }
                    (Ok(()), None) => {}
                    (Err(error), _) => mail.failed("mail.outbox.unsaved", &error, cx),
                }
                mail.read_mail(cx);
                mail.next_out(round, cx);
            },
        );
    }
}

/// Whether we place its Sent copy; None follows the provider.
pub fn copies(account: &Account, choice: Option<bool>) -> bool {
    account.protocol == Protocol::Imap && choice.unwrap_or_else(|| !account.files_sent_itself())
}

/// How a send ended, and the size limit it told.
pub fn landing(
    done: Result<ely_mail_engine::Result<Option<u64>>, String>,
    file: bool,
) -> (Landing, Option<Option<u64>>) {
    match done {
        Ok(Ok(limit)) => (Landing::Accepted { file }, Some(limit)),
        Ok(Err(Error::TooLarge { size, limit })) => {
            let error = Error::TooLarge { size, limit }.to_string();
            (Landing::Failed(error), Some(Some(limit)))
        }
        Ok(Err(error @ (Error::Connect(_) | Error::Timeout(_)))) => {
            (Landing::Waiting(error.to_string()), None)
        }
        Ok(Err(error @ Error::Unsure(_))) => (Landing::Unsure(error.to_string()), None),
        Ok(Err(error)) => (Landing::Failed(error.to_string()), None),
        // The task itself broke: it may have gone.
        Err(error) => (Landing::Unsure(error), None),
    }
}

/// How placing a Sent copy ended.
pub fn filed(done: Result<ely_mail_engine::Result<()>, String>) -> Filed {
    match done {
        Ok(Ok(())) => Filed::Done,
        Ok(Err(error @ (Error::Connect(_) | Error::Timeout(_)))) => Filed::Later(error.to_string()),
        Ok(Err(error)) => Filed::Failed(error.to_string()),
        Err(error) => Filed::Failed(error),
    }
}

/// What a landing says; retries keep quiet offline.
fn say(landing: &Landing, retry: bool) -> Option<(&'static str, Severity, String)> {
    match landing {
        Landing::Accepted { .. } => Some(("mail.outbox.sent", Severity::Success, String::new())),
        Landing::Waiting(_) if retry => None,
        Landing::Waiting(error) => Some(("mail.outbox.offline", Severity::Info, error.clone())),
        Landing::Failed(error) => {
            Some(("mail.outbox.failed_toast", Severity::Danger, error.clone()))
        }
        Landing::Unsure(error) => {
            Some(("mail.outbox.unsure_toast", Severity::Warning, error.clone()))
        }
    }
}

/// What a copy's end says; retries keep quiet offline.
pub fn say_filed(filed: &Filed, retry: bool) -> Option<(&'static str, Severity, String)> {
    match filed {
        Filed::Done => None,
        Filed::Later(_) if retry => None,
        Filed::Later(error) => Some(("mail.outbox.unfiled_later", Severity::Info, error.clone())),
        Filed::Failed(error) => Some((
            "mail.outbox.unfiled_toast",
            Severity::Warning,
            error.clone(),
        )),
    }
}

/// Tells the window how queueing went, outside Mail.
fn queued(compose: WeakEntity<Compose>, done: Result<(), String>, cx: &mut gpui::App) {
    cx.defer(move |cx| {
        compose
            .update(cx, |compose, cx| compose.queued(done, cx))
            .ok();
    });
}
