//! The compose window (C12.12, MAIL-05).

use std::path::PathBuf;

use ely_gpui_component::{
    documents::{DocumentEvent, MarkdownDocument, MarkdownMode},
    mail::Draft,
    primitives::Severity,
};
use ely_mail_engine::{
    AccountId, Mailbox, Part,
    compose::{self, Earlier},
};
use gpui::{
    AnyWindowHandle, App, AppContext as _, Context, Entity, FocusHandle, PathPromptOptions,
    SharedString, WeakEntity, Window,
};

mod keeping;
mod lead;
mod page;

#[cfg(test)]
pub use keeping::PAUSE;
pub use keeping::Shelf;

use crate::{
    mail::{Mail, text},
    writing::{Opening, Outgoing, Sender},
};

/// An attached file and its key in the list.
pub struct File {
    pub key: u64,
    pub part: Part,
}

pub struct Compose {
    mail: WeakEntity<Mail>,
    pub senders: Vec<Sender>,
    pub from: AccountId,
    pub to: Vec<SharedString>,
    pub cc: Vec<SharedString>,
    pub bcc: Vec<SharedString>,
    pub subject: SharedString,
    in_reply_to: Option<String>,
    references: Vec<String>,
    pub before: Option<Earlier>,
    /// Only the answered message's headers are kept.
    pub headers_only: bool,
    pub markdown: bool,
    pub document: Entity<MarkdownDocument>,
    pub files: Vec<File>,
    next_file: u64,
    /// Files picked and not yet read.
    pub reading: usize,
    /// Forwarded files not downloaded.
    pub missing: usize,
    /// Contacts offered as addresses.
    pub offered: Vec<ely_gpui_component::mail::Contact>,
    /// What it opened with, or last sent.
    base: Carried,
    /// The addresses and subject as now written.
    written: Draft,
    /// What the send under way carries.
    in_flight: Option<Carried>,
    /// Sent, waiting for a draft write to land.
    to_queue: Option<Frozen>,
    pub sending: bool,
    pub problem: Option<(Severity, SharedString)>,
    pub discarding: bool,
    pub shelf: Shelf,
    window: AnyWindowHandle,
    focus: FocusHandle,
}

/// A file's shown name; some have none.
pub fn file_name(part: &Part, cx: &App) -> SharedString {
    match &part.name {
        Some(name) => name.clone().into(),
        None => text(cx, "mail.write.unnamed", &[]),
    }
}

fn addresses(list: &[SharedString]) -> Vec<Mailbox> {
    list.iter()
        .map(|email| Mailbox {
            name: None,
            email: email.to_string(),
        })
        .collect()
}

impl Compose {
    pub fn new(
        mail: WeakEntity<Mail>,
        opening: Opening,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // A link's body is plain text.
        let (words, markdown) = match (&opening.draft, &opening.asked) {
            (Some(kept), _) => (kept.unsent.words.as_str(), kept.unsent.markdown),
            (None, Some(asked)) if !asked.body.is_empty() => (asked.body.as_str(), false),
            _ => ("", true),
        };
        let mode = match markdown {
            true => MarkdownMode::Visual,
            false => MarkdownMode::Source,
        };
        let document = cx.new(|cx| MarkdownDocument::new(words, mode, window, cx));
        cx.subscribe(&document, |compose, _, event, cx| {
            if matches!(event, DocumentEvent::Changed) {
                compose.touched(cx);
            }
        })
        .detach();
        let this = cx.weak_entity();
        ely_gpui_component::shell::guard_close(window, cx, move |window, cx| {
            this.update(cx, |compose, cx| compose.may_close(window, cx))
                .unwrap_or(true)
        });
        let mut compose = Compose {
            mail,
            senders: opening.senders,
            from: opening.from,
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: SharedString::default(),
            in_reply_to: None,
            references: Vec::new(),
            before: None,
            headers_only: false,
            markdown,
            document,
            files: Vec::new(),
            next_file: 0,
            reading: 0,
            missing: 0,
            offered: Vec::new(),
            base: Carried {
                draft: blank(),
                words: String::new(),
                files: Vec::new(),
                from: opening.from,
                markdown: true,
            },
            written: blank(),
            in_flight: None,
            to_queue: None,
            sending: false,
            problem: None,
            discarding: false,
            shelf: Shelf::default(),
            window: window.window_handle(),
            focus: cx.focus_handle(),
        };
        if let Some(draft) = opening.draft {
            compose.restore(draft);
        }
        if let Some(answered) = opening.answered {
            let start = answered.start;
            compose.to = start.to.iter().map(|to| to.email.clone().into()).collect();
            compose.cc = start.cc.iter().map(|cc| cc.email.clone().into()).collect();
            compose.subject = start.subject.into();
            compose.in_reply_to = start.in_reply_to;
            compose.references = start.references;
            compose.headers_only = !answered.whole;
            compose.missing = answered.missing;
            for part in answered.files {
                compose.add_file(part);
            }
            if answered.whole {
                let earlier = lead::lead(answered.how, &answered.mail, answered.text, cx);
                compose.before = Some(earlier);
            }
        }
        if let Some(asked) = opening.asked {
            let addresses = |list: Vec<String>| list.into_iter().map(Into::into).collect();
            compose.to = addresses(asked.to);
            compose.cc = addresses(asked.cc);
            compose.bcc = addresses(asked.bcc);
            compose.subject = asked.subject.into();
        }
        compose.written = Draft {
            to: compose.to.clone(),
            cc: compose.cc.clone(),
            bcc: compose.bcc.clone(),
            subject: compose.subject.trim().to_owned().into(),
        };
        compose.base = compose.carried(cx);
        if let Some(id) = compose.shelf.id {
            compose.shelf = Shelf::reopened(id, compose.base.clone());
        }
        compose
    }

    /// Takes up a kept draft where it was left.
    fn restore(&mut self, draft: Reopened) {
        let addresses = |list: Vec<String>| list.into_iter().map(Into::into).collect();
        let unsent = draft.unsent;
        self.to = addresses(unsent.to);
        self.cc = addresses(unsent.cc);
        self.bcc = addresses(unsent.bcc);
        self.subject = unsent.subject.into();
        self.in_reply_to = unsent.in_reply_to;
        self.references = unsent.references;
        self.before = unsent.earlier;
        self.headers_only = unsent.headers_only;
        self.missing = unsent.missing;
        for part in draft.files {
            self.add_file(part);
        }
        self.shelf.id = Some(draft.id);
    }

    /// The composer's addresses and subject after an edit.
    pub fn edited(&mut self, draft: Draft, cx: &mut Context<Self>) {
        self.written = draft;
        self.touched(cx);
        cx.notify();
    }

    pub fn choose_sender(&mut self, from: AccountId, cx: &mut Context<Self>) {
        self.from = from;
        self.touched(cx);
        cx.notify();
    }

    fn sender(&self) -> Option<&Sender> {
        self.senders.iter().find(|sender| sender.id == self.from)
    }

    fn add_file(&mut self, part: Part) {
        self.next_file += 1;
        self.files.push(File {
            key: self.next_file,
            part,
        });
    }

    fn carried(&self, cx: &App) -> Carried {
        Carried {
            draft: self.written.clone(),
            words: self.document.read(cx).text(cx),
            files: self.files.iter().map(|file| file.key).collect(),
            from: self.from,
            markdown: self.markdown,
        }
    }

    fn set_markdown(&mut self, markdown: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.markdown = markdown;
        let mode = match markdown {
            true => MarkdownMode::Visual,
            false => MarkdownMode::Source,
        };
        self.document
            .update(cx, |document, cx| document.set_mode(mode, window, cx));
        self.touched(cx);
        cx.notify();
    }

    fn pick_files(&mut self, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: None,
        });
        cx.spawn(async move |this, cx| {
            let paths = match picked.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) => return log::info!("mail: the attach panel was cancelled"),
                Ok(Err(error)) => return log::error!("mail: the attach panel failed: {error}"),
                Err(error) => return log::error!("mail: the attach panel went away: {error}"),
            };
            this.update(cx, |compose, cx| compose.attach(paths, cx))
                .ok();
        })
        .detach();
    }

    /// Reads the files off the UI thread, then lists them.
    pub fn offer(&mut self, contacts: Vec<ely_mail_engine::Contact>, cx: &mut Context<Self>) {
        let named = |contact: ely_mail_engine::Contact| {
            ely_gpui_component::mail::Contact::new(contact.name.unwrap_or_default(), contact.address)
        };
        self.offered = contacts.into_iter().map(named).collect();
        cx.notify();
    }

    pub fn attach(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let count = paths.len();
        self.reading += count;
        cx.notify();
        let read = cx.background_spawn(async move {
            paths
                .into_iter()
                .map(|path| {
                    let name = path
                        .file_name()
                        .ok_or_else(|| "a path names no file".to_owned())?
                        .to_string_lossy()
                        .into_owned();
                    let bytes = std::fs::read(&path).map_err(|err| err.to_string())?;
                    Ok(compose::attachment(&name, bytes))
                })
                .collect::<Result<Vec<_>, String>>()
        });
        cx.spawn(async move |this, cx| {
            let read = read.await;
            this.update(cx, |compose, cx| {
                compose.reading -= count;
                match read {
                    Ok(parts) => {
                        log::info!("mail: {} files attached", parts.len());
                        parts.into_iter().for_each(|part| compose.add_file(part));
                        compose.touched(cx);
                    }
                    Err(error) => {
                        log::warn!("mail: a file was not attached");
                        let words = text(cx, "mail.write.attach_failed", &[("error", &error)]);
                        compose.problem = Some((Severity::Danger, words));
                        compose.unread(cx);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn outgoing(&self, draft: &Draft, cx: &App) -> Option<Outgoing> {
        let sender = self.sender()?;
        Some(Outgoing {
            from: Mailbox {
                name: sender.identity.name.clone(),
                email: sender.address.clone(),
            },
            to: addresses(&draft.to),
            cc: addresses(&draft.cc),
            bcc: addresses(&draft.bcc),
            subject: draft.subject.to_string(),
            words: self.document.read(cx).text(cx),
            markdown: self.markdown,
            signature: sender.identity.signature.clone(),
            before: self.before.clone(),
            attachments: self.files.iter().map(|file| file.part.clone()).collect(),
            in_reply_to: self.in_reply_to.clone(),
            references: self.references.clone(),
            date: jiff::Timestamp::now().as_second(),
        })
    }

    /// Hands it to the Outbox; the queue sends it.
    pub fn send(&mut self, draft: &Draft, _: &mut Window, cx: &mut Context<Self>) {
        if self.sending {
            return;
        }
        if self.reading > 0 {
            return log::info!("mail: send waits for {} files", self.reading);
        }
        self.begin_sending(cx);
        let Some(out) = self.outgoing(draft, cx) else {
            log::error!("mail: the sender {} is not listed", self.from.0);
            return self.queued(Err("the sender is not listed".into()), cx);
        };
        log::info!(
            "mail: queueing for {} addresses",
            draft.to.len() + draft.cc.len()
        );
        let mut unsent = self.unsent(cx);
        unsent.to = out.to.iter().map(|to| to.email.clone()).collect();
        unsent.cc = out.cc.iter().map(|cc| cc.email.clone()).collect();
        unsent.bcc = out.bcc.iter().map(|bcc| bcc.email.clone()).collect();
        unsent.subject = out.subject.clone();
        let from = self.from;
        self.to_queue = Some(Frozen { from, unsent, out });
        self.queue_now(cx);
    }

    /// Queues once no draft write is under way.
    fn queue_now(&mut self, cx: &mut Context<Self>) {
        if self.shelf_busy() {
            return log::info!("mail: send waits for its draft write");
        }
        let Some(Frozen { from, unsent, out }) = self.to_queue.take() else {
            return log::error!("mail: queueing with nothing sent");
        };
        let (held, this) = (self.shelf.id, cx.weak_entity());
        let asked = self.mail.update(cx, |mail, cx| {
            mail.queue_message(from, held, unsent, out, this, cx)
        });
        if asked.is_err() {
            self.queued(Err("the mail window is closed".into()), cx);
        }
    }

    pub fn begin_sending(&mut self, cx: &mut Context<Self>) {
        self.sending = true;
        self.in_flight = Some(self.carried(cx));
        self.problem = None;
        cx.notify();
    }

    /// In the Outbox, it closes; else it stays to retry.
    pub fn queued(&mut self, done: Result<(), String>, cx: &mut Context<Self>) {
        self.sending = false;
        let carried = self.in_flight.take();
        match done {
            Ok(()) => {
                self.handed_over();
                self.after_sending(carried, cx);
            }
            Err(error) => {
                log::warn!("mail: a message was not queued");
                let words = text(cx, "mail.outbox.unqueued", &[("error", &error)]);
                self.problem = Some((Severity::Danger, words));
                self.next(cx);
            }
        }
    }
}

impl Compose {
    /// Closes, unless edits came after what was sent.
    fn after_sending(&mut self, carried: Option<Carried>, cx: &mut Context<Self>) {
        let Some(carried) = carried else {
            log::error!("mail: a send ended that was never started");
            return;
        };
        if self.reading == 0 && carried == self.carried(cx) {
            return self.sent_off(true, cx);
        }
        log::info!("mail: edits made while sending stay, unsent");
        self.base = carried;
        self.problem = Some((Severity::Info, text(cx, "mail.write.sent_later", &[])));
        self.sent_off(false, cx);
        cx.notify();
    }
}

/// A send, fixed as it was pressed.
struct Frozen {
    from: AccountId,
    unsent: ely_mail_engine::Unsent,
    out: Outgoing,
}

/// What one send carried.
#[derive(Clone, PartialEq)]
pub struct Carried {
    draft: Draft,
    words: String,
    files: Vec<u64>,
    from: AccountId,
    markdown: bool,
}

impl Carried {
    /// The same message, but for outer spaces.
    pub(super) fn blank_as(&self, base: &Carried) -> bool {
        let words = self.words.trim() == base.words.trim();
        words
            && self.draft == base.draft
            && self.files == base.files
            && self.from == base.from
            && self.markdown == base.markdown
    }
}

/// A draft from the store, to write on.
pub struct Reopened {
    pub id: ely_mail_engine::DraftId,
    pub unsent: ely_mail_engine::Unsent,
    pub files: Vec<Part>,
}

fn blank() -> Draft {
    Draft {
        to: Vec::new(),
        cc: Vec::new(),
        bcc: Vec::new(),
        subject: SharedString::default(),
    }
}
