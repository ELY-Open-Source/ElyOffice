//! Drafts kept here, and their copies on the server (C12.13).

use ely_gpui_component::primitives::Severity;
use ely_mail_engine::{
    Account, AccountId, DraftId, Left, Part, Remote, Role, Store, TlsConnector, Unsent, drafts,
    sync,
};
use gpui::{App, Context, SharedString, WeakEntity, WindowHandle};

use crate::{
    compose::{Compose, Reopened},
    mail::{Mail, text},
    writing::{Opening, Outgoing, prepare, senders},
};

/// The next server copy asked while one goes.
pub struct Waiting {
    account: AccountId,
    out: Outgoing,
    compose: WeakEntity<Compose>,
}

impl Mail {
    /// Keeps the words here, then on the server if asked.
    #[allow(clippy::too_many_arguments)]
    pub fn keep_draft(
        &mut self,
        id: Option<DraftId>,
        account: AccountId,
        unsent: Unsent,
        files: Option<Vec<Part>>,
        out: Option<Outgoing>,
        compose: WeakEntity<Compose>,
        cx: &mut Context<Self>,
    ) {
        self.with_store(
            cx,
            move |store| store.shelve(id, account, &unsent, files.as_deref()),
            move |mail, done, cx| {
                let done = done.map(|stowed| {
                    if let Some(left) = stowed.left {
                        mail.take_copy(left, cx);
                    }
                    stowed.id
                });
                if let (Ok(id), Some(out)) = (&done, out) {
                    mail.push_draft(*id, account, out, compose.clone(), cx);
                }
                // The window may write again; Mail is busy here.
                cx.defer(move |cx| {
                    compose
                        .update(cx, |compose, cx| compose.wrote(done, cx))
                        .ok();
                });
                mail.drafts_changed(cx);
            },
        );
    }

    /// Drops it here, then its copy on the server.
    pub fn drop_draft(
        &mut self,
        id: DraftId,
        compose: Option<WeakEntity<Compose>>,
        cx: &mut Context<Self>,
    ) {
        self.with_store(
            cx,
            move |store| store.unshelve(id),
            move |mail, done, cx| {
                if let Ok(Some(left)) = &done {
                    mail.take_copy(left.clone(), cx);
                }
                let done = done.map(|_| ());
                match compose {
                    Some(compose) => cx.defer(move |cx| {
                        compose
                            .update(cx, |compose, cx| compose.dropped(done, cx))
                            .ok();
                    }),
                    None => {
                        if let Err(error) = done {
                            mail.failed("mail.drafts.undropped", &error, cx);
                        }
                    }
                }
                mail.drafts_changed(cx);
            },
        );
    }

    /// One server copy at a time per draft.
    fn push_draft(
        &mut self,
        id: DraftId,
        account: AccountId,
        out: Outgoing,
        compose: WeakEntity<Compose>,
        cx: &mut Context<Self>,
    ) {
        if let Some(waiting) = self.pushing.get_mut(&id) {
            *waiting = Some(Waiting {
                account,
                out,
                compose,
            });
            return;
        }
        self.pushing.insert(id, None);
        self.with_store(
            cx,
            move |store| aimed(store, id, account),
            move |mail, read, cx| match read {
                Ok(None) => mail.pushed(id, cx),
                Ok(Some((None, _))) => {
                    tell(
                        mail,
                        &compose,
                        Severity::Info,
                        "mail.drafts.no_folder",
                        "",
                        cx,
                    );
                    mail.pushed(id, cx);
                }
                Ok(Some((Some(mailbox), old))) => {
                    mail.put_copy(id, account, out, mailbox, old, compose, cx);
                }
                Err(error) => {
                    tell(
                        mail,
                        &compose,
                        Severity::Warning,
                        "mail.drafts.server_failed",
                        &error,
                        cx,
                    );
                    mail.pushed(id, cx);
                }
            },
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn put_copy(
        &mut self,
        id: DraftId,
        account_id: AccountId,
        out: Outgoing,
        mailbox: String,
        old: Option<Remote>,
        compose: WeakEntity<Compose>,
        cx: &mut Context<Self>,
    ) {
        let (account, tls) = match self.reach(account_id) {
            Ok(reach) => reach,
            Err(error) => {
                tell(
                    self,
                    &compose,
                    Severity::Warning,
                    "mail.drafts.server_failed",
                    &error,
                    cx,
                );
                return self.pushed(id, cx);
            }
        };
        let vault = self.vault.clone();
        let work = async move {
            let built = prepare(&out)?;
            let secret = sync::secret(&account, &vault).await?;
            drafts::put(&account, secret, &tls, &mailbox, &built.bytes, old.as_ref()).await
        };
        self.on_runtime(cx, work, move |mail, done, cx| {
            let placed = match done.and_then(|done| done.map_err(|err| err.to_string())) {
                Ok(placed) => placed,
                Err(error) => {
                    tell(
                        mail,
                        &compose,
                        Severity::Warning,
                        "mail.drafts.server_failed",
                        &error,
                        cx,
                    );
                    return mail.pushed(id, cx);
                }
            };
            if let Some(left) = &placed.left {
                tell(
                    mail,
                    &compose,
                    Severity::Warning,
                    "mail.drafts.server_stale",
                    left,
                    cx,
                );
            }
            let remote = placed.remote;
            mail.with_store(
                cx,
                move |store| store.set_remote(id, account_id, &remote),
                move |mail, done, cx| {
                    match done {
                        Ok(Some(orphan)) => mail.take_copy(orphan, cx),
                        Ok(None) => log::info!("mail: draft {} has its server copy", id.0),
                        Err(error) => mail.failed("mail.drafts.server_failed", &error, cx),
                    }
                    mail.pushed(id, cx);
                },
            );
        });
    }

    fn pushed(&mut self, id: DraftId, cx: &mut Context<Self>) {
        if let Some(Some(next)) = self.pushing.remove(&id) {
            self.push_draft(id, next.account, next.out, next.compose, cx);
        }
    }

    /// Removes a server copy no draft names now.
    pub(crate) fn take_copy(&mut self, left: Left, cx: &mut Context<Self>) {
        let (account, tls) = match self.reach(left.account) {
            Ok(reach) => reach,
            Err(error) => return self.failed("mail.drafts.server_stale", &error, cx),
        };
        let vault = self.vault.clone();
        let work = async move {
            let secret = sync::secret(&account, &vault).await?;
            drafts::take(&account, secret, &tls, &left.remote).await
        };
        self.on_runtime(cx, work, |mail, done, cx| {
            match done.and_then(|done| done.map_err(|err| err.to_string())) {
                Ok(true) => log::info!("mail: a server copy of a draft went"),
                Ok(false) => {
                    let words = "the drafts folder was renumbered";
                    mail.failed("mail.drafts.server_stale", words, cx);
                }
                Err(error) => mail.failed("mail.drafts.server_stale", &error, cx),
            }
        });
    }

    /// Opens it to write on, or its window.
    pub fn reopen(&mut self, id: DraftId, cx: &mut Context<Self>) {
        if let Some(compose) = holder(id, cx) {
            log::info!("mail: draft {} is open already", id.0);
            compose
                .update(cx, |_, window, _| window.activate_window())
                .ok();
            return;
        }
        if !self.reopening.insert(id) {
            return log::info!("mail: draft {} is opening already", id.0);
        }
        self.with_store(
            cx,
            move |store| {
                let draft = store
                    .draft(id)?
                    .ok_or_else(|| ely_mail_engine::Error::Store(format!("no draft {}", id.0)))?;
                Ok(Opening {
                    senders: senders(store)?,
                    from: draft.account,
                    answered: None,
                    draft: Some(Reopened {
                        id,
                        unsent: draft.unsent,
                        files: store.draft_files(id)?,
                    }),
                    asked: None,
                })
            },
            move |mail, read, cx| {
                mail.reopening.remove(&id);
                if mail.unwanted.remove(&id) {
                    log::info!("mail: draft {} was deleted while opening", id.0);
                    return mail.drop_draft(id, None, cx);
                }
                match read {
                    Ok(opening) => {
                        mail.open_compose(opening, cx);
                    }
                    Err(error) => mail.failed("mail.drafts.unread", &error, cx),
                }
            },
        );
    }

    /// The account and TLS a server step needs.
    pub fn reach(&self, id: AccountId) -> Result<(Account, TlsConnector), String> {
        let account = self
            .accounts()
            .iter()
            .find(|(each, _)| *each == id)
            .map(|(_, account)| account.clone())
            .ok_or_else(|| "the account is no longer here".to_owned())?;
        Ok((account, self.tls.clone()?))
    }
}

/// Where the copy goes, and the old one.
fn aimed(
    store: &mut Store,
    id: DraftId,
    account: AccountId,
) -> ely_mail_engine::Result<Option<(Option<String>, Option<Remote>)>> {
    let Some(draft) = store.draft(id)?.filter(|draft| draft.account == account) else {
        return Ok(None);
    };
    if !store.drafts_up(account)? {
        return Ok(None);
    }
    let mailbox = store
        .folders(account)?
        .into_iter()
        .find(|folder| folder.role == Some(Role::Drafts))
        .map(|folder| folder.name);
    Ok(Some((mailbox, draft.remote)))
}

/// The message window that holds this draft.
pub fn holder(id: DraftId, cx: &App) -> Option<WindowHandle<Compose>> {
    cx.windows().into_iter().find_map(|each| {
        let compose = each.downcast::<Compose>()?;
        let holds = compose.read(cx).is_ok_and(|it| it.shelf.id == Some(id));
        holds.then_some(compose)
    })
}

/// Says it in the message's window; closed, in this one.
fn tell(
    mail: &Mail,
    compose: &WeakEntity<Compose>,
    severity: Severity,
    key: &str,
    error: &str,
    cx: &mut App,
) {
    let words: SharedString = text(cx, key, &[("error", error)]);
    log::warn!("mail: {key}");
    let shown = compose.update(cx, |compose, cx| {
        compose.problem = Some((severity, words.clone()));
        cx.notify();
    });
    if shown.is_err() {
        mail.toast(words, severity, cx);
    }
}
