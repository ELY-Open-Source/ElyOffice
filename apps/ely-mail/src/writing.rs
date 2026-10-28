//! Opening a message to write, and handing it over (C12.12).

use std::path::PathBuf;

use ely_mail_engine::{
    AccountId, Draft, Error, Identity, Letter, Mail as Message, Mailbox, MessageId, Part, Result,
    Store, build,
    compose::{self, Earlier, Start, Writing},
    read,
};
use ely_platform::AppId;
use gpui::{App, AppContext as _, Context, WeakEntity, Window, WindowHandle, px, size};

use crate::{
    compose::{Compose, Reopened},
    mail::{Mail, text},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum How {
    Reply,
    ReplyAll,
    Forward,
}

/// An account one may send from.
#[derive(Clone)]
pub struct Sender {
    pub id: AccountId,
    pub address: String,
    pub identity: Identity,
    /// The outgoing server's last stated size limit.
    pub limit: Option<u64>,
}

/// What a new message starts from.
pub struct Opening {
    pub senders: Vec<Sender>,
    pub from: AccountId,
    pub answered: Option<Answered>,
    pub draft: Option<Reopened>,
    /// A mail link's addresses, subject and body.
    pub asked: Option<ely_mail_engine::mailto::Asked>,
}

/// The message answered or forwarded.
pub struct Answered {
    pub how: How,
    pub mail: Message,
    pub start: Start,
    /// Its words, empty when only its headers are kept.
    pub text: String,
    pub whole: bool,
    /// Forwarded files that are here.
    pub files: Vec<Part>,
    /// Forwarded files not downloaded, left out.
    pub missing: usize,
}

/// Everything a message is built from.
pub struct Outgoing {
    pub from: Mailbox,
    pub to: Vec<Mailbox>,
    pub cc: Vec<Mailbox>,
    pub bcc: Vec<Mailbox>,
    pub subject: String,
    pub words: String,
    pub markdown: bool,
    pub signature: String,
    pub before: Option<Earlier>,
    pub attachments: Vec<Part>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub date: i64,
}

/// Bcc stays in it until it is sent.
pub fn prepare(out: &Outgoing) -> Result<Letter> {
    let bodies = compose::bodies(&Writing {
        words: &out.words,
        markdown: out.markdown,
        signature: &out.signature,
        earlier: out.before.as_ref(),
    })?;
    let built = build(&Draft {
        from: out.from.clone(),
        to: out.to.clone(),
        cc: out.cc.clone(),
        bcc: out.bcc.clone(),
        subject: out.subject.clone(),
        text: bodies.text,
        html: bodies.html,
        attachments: out.attachments.clone(),
        in_reply_to: out.in_reply_to.clone(),
        references: out.references.clone(),
        date: out.date,
    })?;
    let mut envelope: Vec<String> = Vec::new();
    for person in out.to.iter().chain(&out.cc).chain(&out.bcc) {
        if !envelope
            .iter()
            .any(|kept| kept.eq_ignore_ascii_case(&person.email))
        {
            envelope.push(person.email.clone());
        }
    }
    Ok(Letter {
        envelope,
        message_id: built.message_id,
        bytes: built.bytes,
    })
}

pub fn senders(store: &Store) -> Result<Vec<Sender>> {
    store
        .accounts()?
        .into_iter()
        .map(|(id, account)| {
            Ok(Sender {
                id,
                address: account.address,
                identity: store.identity(id)?,
                limit: store.smtp_limit(id)?,
            })
        })
        .collect()
}

fn answered(store: &Store, id: MessageId, how: How) -> Result<Opening> {
    let senders = senders(store)?;
    let stored = store.get(id)?;
    let from = store.account_of(stored.mailbox)?;
    let me = senders
        .iter()
        .find(|sender| sender.id == from)
        .map(|sender| sender.address.clone())
        .ok_or_else(|| Error::Store(format!("account {} is not kept", from.0)))?;
    let (bytes, whole) = match store.original(id)? {
        Some(raw) => (raw, true),
        None => (store.head(id)?, false),
    };
    let mail = read(&bytes)?;
    let start = match how {
        How::Reply => compose::answer(&mail, &me, false),
        How::ReplyAll => compose::answer(&mail, &me, true),
        How::Forward => compose::forward(&mail),
    };
    let text = match whole {
        true => compose::original(&mail),
        false => String::new(),
    };
    let (files, missing) = match how {
        How::Forward => {
            let waiting = |part: &Part| {
                part.section
                    .as_ref()
                    .is_some_and(|section| stored.missing.contains(section))
            };
            let (gone, here): (Vec<Part>, Vec<Part>) =
                mail.attachments.iter().cloned().partition(waiting);
            (here, gone.len())
        }
        _ => (Vec::new(), 0),
    };
    Ok(Opening {
        senders,
        from,
        answered: Some(Answered {
            how,
            mail,
            start,
            text,
            whole,
            files,
            missing,
        }),
        draft: None,
        asked: None,
    })
}

/// Quit and closing the main window ask each message first.
pub fn guard_leaving(mail: WeakEntity<Mail>, window: &mut Window, cx: &mut App) {
    let quitting = mail.clone();
    ely_ui::commands::route_quit(cx, move |cx| {
        if may_leave(&quitting, cx) {
            cx.quit();
        }
    });
    // A kept window closes to the tray; only quitting asks.
    ely_gpui_component::shell::guard_close(window, cx, move |_, cx| ely_ui::tray::kept(cx) || may_leave(&mail, cx));
}

/// True when every message, then the Outbox, lets it go.
fn may_leave(mail: &WeakEntity<Mail>, cx: &mut App) -> bool {
    for each in cx.windows() {
        let Some(compose) = each.downcast::<Compose>() else {
            continue;
        };
        let leaves = compose.update(cx, |compose, window, cx| compose.may_leave(window, cx));
        match leaves {
            Ok(true) => {}
            Ok(false) => {
                log::info!("mail: a message holds the quit");
                return false;
            }
            Err(error) => log::warn!("mail: a closing message did not answer: {error}"),
        }
    }
    mail.update(cx, |mail, cx| mail.may_quit(cx))
        .unwrap_or(true)
}

impl Mail {
    /// A blank message from the open account, else the first.
    pub fn write_new(&mut self, cx: &mut Context<Self>) {
        self.write_with(Vec::new(), cx);
    }

    /// A new message carrying `files` (§16); it never sends itself.
    pub fn write_with(&mut self, files: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.write(files, None, cx);
    }

    /// A new message filled from a mail link.
    pub fn write_asked(&mut self, asked: ely_mail_engine::mailto::Asked, cx: &mut Context<Self>) {
        self.write(Vec::new(), Some(asked), cx);
    }

    fn write(&mut self, files: Vec<PathBuf>, asked: Option<ely_mail_engine::mailto::Asked>, cx: &mut Context<Self>) {
        let preferred = self.open.account();
        self.with_store(
            cx,
            |store| senders(store),
            move |mail, read, cx| match read {
                Ok(senders) if senders.is_empty() => {
                    mail.toast(
                        text(cx, "mail.write.no_account", &[]),
                        ely_gpui_component::primitives::Severity::Info,
                        cx,
                    );
                }
                Ok(senders) => {
                    let from = preferred
                        .filter(|id| senders.iter().any(|sender| sender.id == *id))
                        .unwrap_or(senders[0].id);
                    let opened = mail.open_compose(
                        Opening {
                            senders,
                            from,
                            answered: None,
                            draft: None,
                            asked,
                        },
                        cx,
                    );
                    if let Some(window) = opened.filter(|_| !files.is_empty()) {
                        window.update(cx, |compose, _, cx| compose.attach(files, cx)).ok();
                    }
                }
                Err(error) => mail.failed("mail.write.unread", &error, cx),
            },
        );
    }

    pub fn answer(&mut self, id: MessageId, how: How, cx: &mut Context<Self>) {
        log::info!("mail: {how:?} to message {}", id.0);
        self.with_store(
            cx,
            move |store| answered(store, id, how),
            |mail, read, cx| match read {
                Ok(opening) => {
                    mail.open_compose(opening, cx);
                }
                Err(error) => mail.failed("mail.write.unread", &error, cx),
            },
        );
    }

    pub fn open_compose(&mut self, opening: Opening, cx: &mut Context<Self>) -> Option<WindowHandle<Compose>> {
        let mut options = ely_ui::window_options(AppId::Mail, size(px(760.0), px(680.0)), cx);
        let subject = match (&opening.draft, &opening.answered) {
            (Some(draft), _) => Some(&draft.unsent.subject),
            (None, Some(answered)) => Some(&answered.start.subject),
            (None, None) => None,
        };
        let title = match subject {
            Some(subject) if !subject.is_empty() => subject.clone().into(),
            _ => text(cx, "mail.write.title", &[]),
        };
        if let Some(bar) = options.titlebar.as_mut() {
            bar.title = Some(title);
        }
        let this = cx.weak_entity();
        let opened = cx.open_window(options, |window, cx| {
            cx.new(|cx| Compose::new(this, opening, window, cx))
        });
        match opened {
            Ok(window) => {
                log::info!("mail: a compose window opened");
                self.offer_contacts(window, cx);
                Some(window)
            }
            Err(error) => {
                self.failed("mail.write.unopened", &format!("{error:#}"), cx);
                None
            }
        }
    }
}
