//! A message kept as a draft (C12.13).

use std::time::Duration;

use ely_gpui_component::primitives::Severity;
use ely_mail_engine::{DraftId, Unsent};
use gpui::{App, Context, SharedString, Window};

use super::{Carried, Compose};
use crate::mail::text;

/// Typing stops this long before the draft is kept.
pub const PAUSE: Duration = Duration::from_millis(1500);

/// The draft in the store, and the writes to it.
#[derive(Default)]
pub struct Shelf {
    pub id: Option<DraftId>,
    /// What the store holds.
    kept: Option<Carried>,
    /// Being written now.
    writing: Option<Carried>,
    /// Opened from a draft: untouched, it stays.
    reopened: bool,
    /// Thrown away: nothing more is kept.
    discarded: bool,
    edits: u64,
    busy: bool,
    /// Asked while a write was under way.
    queued: Option<Step>,
    leaving: Option<Leave>,
}

impl Shelf {
    pub fn reopened(id: DraftId, kept: Carried) -> Self {
        Self {
            id: Some(id),
            kept: Some(kept),
            reopened: true,
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Keep,
    Drop,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Leave {
    Close,
    Quit,
}

fn list(addresses: &[SharedString]) -> Vec<String> {
    addresses.iter().map(|it| it.to_string()).collect()
}

impl Compose {
    /// Something changed; kept once typing stops.
    pub(super) fn touched(&mut self, cx: &mut Context<Self>) {
        if self.shelf.discarded || self.shelf.leaving.is_some() {
            return self.keep(cx);
        }
        self.shelf.edits += 1;
        let edits = self.shelf.edits;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(PAUSE).await;
            this.update(cx, |compose, cx| {
                if compose.shelf.edits == edits {
                    compose.keep(cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// Nothing waits, and the store holds all of it.
    fn settled(&self, cx: &App) -> bool {
        if self.shelf.busy || self.shelf.queued.is_some() || self.reading > 0 {
            return false;
        }
        if self.shelf.discarded {
            return true;
        }
        let now = self.carried(cx);
        match self.shelf.id {
            Some(_) => self.shelf.kept.as_ref() == Some(&now),
            None => now.blank_as(&self.base) && !self.shelf.reopened,
        }
    }

    /// Writes what changed; back to blank, drops it.
    pub fn keep(&mut self, cx: &mut Context<Self>) {
        if self.shelf.discarded {
            return self.settle(cx);
        }
        if self.shelf.busy || self.sending {
            self.shelf.queued.get_or_insert(Step::Keep);
            return;
        }
        if self.reading > 0 {
            return log::info!("mail: a draft waits for {} files", self.reading);
        }
        let now = self.carried(cx);
        if now.blank_as(&self.base) && !self.shelf.reopened {
            return match self.shelf.id {
                Some(_) => self.drop_draft(cx),
                None => self.settle(cx),
            };
        }
        if self.shelf.kept.as_ref() == Some(&now) {
            return self.settle(cx);
        }
        self.write(now, cx);
    }

    fn write(&mut self, now: Carried, cx: &mut Context<Self>) {
        let same = |kept: &Carried| kept.files == now.files;
        let files = match self.shelf.kept.as_ref().is_some_and(same) {
            true => None,
            false => Some(self.files.iter().map(|file| file.part.clone()).collect()),
        };
        let unsent = self.unsent(cx);
        let out = self.outgoing(&self.written, cx);
        self.shelf.busy = true;
        self.shelf.writing = Some(now);
        let (id, from, this) = (self.shelf.id, self.from, cx.weak_entity());
        let asked = self.mail.update(cx, |mail, cx| {
            mail.keep_draft(id, from, unsent, files, out, this, cx)
        });
        if asked.is_err() {
            self.wrote(Err("the mail window is closed".into()), cx);
        }
    }

    /// The store's answer to a write.
    pub fn wrote(&mut self, done: Result<DraftId, String>, cx: &mut Context<Self>) {
        self.shelf.busy = false;
        let writing = self.shelf.writing.take();
        match done {
            Ok(id) => {
                self.shelf.id = Some(id);
                self.shelf.kept = writing;
            }
            Err(error) => {
                log::warn!("mail: a draft was not kept");
                self.shelf.leaving = None;
                let words = text(cx, "mail.drafts.unkept", &[("error", &error)]);
                self.problem = Some((Severity::Danger, words));
                return self.after_failing(cx);
            }
        }
        self.next(cx);
    }

    fn drop_draft(&mut self, cx: &mut Context<Self>) {
        if self.shelf.busy {
            self.shelf.queued = Some(Step::Drop);
            return;
        }
        let Some(id) = self.shelf.id else {
            return self.next(cx);
        };
        self.shelf.busy = true;
        let this = cx.weak_entity();
        let asked = self
            .mail
            .update(cx, |mail, cx| mail.drop_draft(id, Some(this), cx));
        if asked.is_err() {
            self.dropped(Err("the mail window is closed".into()), cx);
        }
    }

    /// The store's answer to a drop.
    pub fn dropped(&mut self, done: Result<(), String>, cx: &mut Context<Self>) {
        self.shelf.busy = false;
        match done {
            Ok(()) => {
                self.shelf.id = None;
                self.shelf.kept = None;
            }
            Err(error) => {
                log::warn!("mail: a draft was not dropped");
                self.shelf.leaving = None;
                self.shelf.discarded = false;
                let words = text(cx, "mail.drafts.undropped", &[("error", &error)]);
                self.problem = Some((Severity::Danger, words));
                return self.after_failing(cx);
            }
        }
        self.next(cx);
    }

    /// Keeping stops when the store holds it all.
    pub(super) fn next(&mut self, cx: &mut Context<Self>) {
        if self.to_queue.is_some() {
            return self.queue_now(cx);
        }
        match self.shelf.queued.take() {
            Some(Step::Drop) => self.drop_draft(cx),
            Some(Step::Keep) | None => self.keep(cx),
        }
    }

    /// No retry; an asked drop still goes.
    fn after_failing(&mut self, cx: &mut Context<Self>) {
        if self.to_queue.is_some() {
            return self.queue_now(cx);
        }
        match self.shelf.queued.take() {
            Some(Step::Drop) => self.drop_draft(cx),
            Some(Step::Keep) | None => cx.notify(),
        }
    }

    /// A close or quit asked meanwhile now goes.
    fn settle(&mut self, cx: &mut Context<Self>) {
        cx.notify();
        let Some(leave) = self.shelf.leaving else {
            return;
        };
        if !self.settled(cx) {
            return log::info!("mail: closing waits for its writes and files");
        }
        self.shelf.leaving = None;
        self.left(cx);
        let window = self.window;
        cx.defer(move |cx| {
            window
                .update(cx, |_, window, _| window.remove_window())
                .ok();
            if leave == Leave::Quit {
                ely_ui::commands::quit(cx);
            }
        });
    }

    /// A file failed: a discard goes on, a close stops.
    pub(super) fn unread(&mut self, cx: &mut Context<Self>) {
        match self.shelf.discarded {
            true => self.keep(cx),
            false => self.shelf.leaving = None,
        }
    }

    /// Says where a closing message stays.
    fn left(&self, cx: &mut App) {
        if self.shelf.id.is_none() || self.shelf.discarded {
            return;
        }
        log::info!("mail: a message closed into its draft");
        let words = text(cx, "mail.drafts.kept", &[]);
        self.mail
            .update(cx, |mail, cx| mail.toast(words, Severity::Info, cx))
            .ok();
    }

    /// Closes once kept; never while sending.
    pub fn may_close(&mut self, _: &mut Window, cx: &mut Context<Self>) -> bool {
        self.may_go(Leave::Close, cx)
    }

    /// Asked on the way out; true lets the app go.
    pub fn may_leave(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let leaves = self.may_go(Leave::Quit, cx);
        if !leaves {
            window.activate_window();
        }
        leaves
    }

    fn may_go(&mut self, leave: Leave, cx: &mut Context<Self>) -> bool {
        if self.still_sending(cx) {
            return false;
        }
        if self.settled(cx) {
            self.left(cx);
            return true;
        }
        self.shelf.leaving = Some(leave);
        self.keep(cx);
        false
    }

    /// Says to wait while the server has it.
    fn still_sending(&mut self, cx: &mut Context<Self>) -> bool {
        if self.sending {
            self.problem = Some((Severity::Warning, text(cx, "mail.write.wait", &[])));
            cx.notify();
        }
        self.sending
    }

    /// Asks before throwing words away; blank just closes.
    pub fn discard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.still_sending(cx) {
            return;
        }
        let blank = self.shelf.id.is_none() && !self.shelf.busy && self.reading == 0;
        if blank && self.carried(cx).blank_as(&self.base) {
            log::info!("mail: a blank message closed");
            return window.remove_window();
        }
        self.discarding = true;
        cx.notify();
    }

    /// Drops the draft here and on the server, then closes.
    pub fn confirm_discard(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.discarding = false;
        if self.still_sending(cx) {
            return log::info!("mail: a discard waits out the send");
        }
        log::info!("mail: a message was discarded");
        self.shelf.discarded = true;
        self.shelf.leaving = Some(Leave::Close);
        self.drop_draft(cx);
    }

    pub fn keep_writing(&mut self, cx: &mut Context<Self>) {
        self.discarding = false;
        cx.notify();
    }

    pub(crate) fn shelf_busy(&self) -> bool {
        self.shelf.busy
    }

    /// The queue took the draft; none is kept now.
    pub(super) fn handed_over(&mut self) {
        self.shelf.id = None;
        self.shelf.kept = None;
    }

    /// Sent: the draft goes; then closes, or keeps later edits.
    pub(super) fn sent_off(&mut self, closes: bool, cx: &mut Context<Self>) {
        if closes {
            self.shelf.discarded = true;
            self.shelf.leaving = Some(Leave::Close);
            return self.drop_draft(cx);
        }
        self.shelf.reopened = false;
        self.drop_draft(cx);
    }

    pub(super) fn unsent(&self, cx: &App) -> Unsent {
        Unsent {
            to: list(&self.written.to),
            cc: list(&self.written.cc),
            bcc: list(&self.written.bcc),
            subject: self.written.subject.to_string(),
            words: self.document.read(cx).text(cx),
            markdown: self.markdown,
            earlier: self.before.clone(),
            in_reply_to: self.in_reply_to.clone(),
            references: self.references.clone(),
            headers_only: self.headers_only,
            missing: self.missing,
        }
    }
}
