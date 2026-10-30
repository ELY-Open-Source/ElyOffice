//! What the window has open, and its list.

use std::{collections::HashMap, rc::Rc};

use ely_gpui_component::mail::Mail as Row;
use ely_mail_engine::{AccountId, Filter, MailboxId, MessageId, Role, Scope, Summary};
use gpui::{Context, SharedString};

use crate::{
    draft_list,
    mail::{Mail, text},
    outbox_list,
};

pub type Unread = HashMap<MailboxId, u32>;

/// What the middle of the window shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Open {
    Inboxes,
    /// Drafts kept here, of every account.
    Drafts,
    /// Messages handed over to send, of every account.
    Outbox,
    /// The people mail is written to.
    Contacts,
    Mailbox(AccountId, MailboxId),
    /// The account's settings and sync.
    Account(AccountId),
}

impl Open {
    pub fn key(self) -> SharedString {
        match self {
            Open::Inboxes => "inboxes".into(),
            Open::Drafts => "drafts".into(),
            Open::Outbox => "outbox".into(),
            Open::Contacts => "contacts".into(),
            Open::Mailbox(_, mailbox) => format!("box-{}", mailbox.0).into(),
            Open::Account(id) => format!("account-{}", id.0).into(),
        }
    }

    pub fn account(self) -> Option<AccountId> {
        match self {
            Open::Inboxes | Open::Drafts | Open::Outbox | Open::Contacts => None,
            Open::Mailbox(id, _) | Open::Account(id) => Some(id),
        }
    }

    fn scope(self) -> Option<Scope> {
        match self {
            Open::Inboxes => Some(Scope::Inboxes),
            Open::Mailbox(_, mailbox) => Some(Scope::Mailbox(mailbox)),
            Open::Drafts | Open::Outbox | Open::Contacts | Open::Account(_) => None,
        }
    }
}

/// Where a listed message is, and its state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fact {
    pub id: MessageId,
    pub account: AccountId,
    pub mailbox: MailboxId,
    pub role: Option<Role>,
    pub pop: bool,
    pub seen: bool,
    pub flagged: bool,
}

/// Rows read, and where each key sits.
pub struct Indexed {
    rows: Vec<Row>,
    at: HashMap<SharedString, usize>,
    facts: HashMap<SharedString, Fact>,
}

/// Off the UI thread: a long list takes a while.
pub fn index(rows: Vec<Row>) -> Indexed {
    let at = rows
        .iter()
        .enumerate()
        .map(|(ix, row)| (row.key.clone(), ix))
        .collect();
    Indexed {
        rows,
        at,
        facts: HashMap::new(),
    }
}

/// The open list and the messages picked in it.
#[derive(Default)]
pub struct Listing {
    /// Numbers reads; an older one is dropped.
    pub read: u64,
    /// None until read, or when nothing lists mail.
    pub mails: Option<Result<Rc<Vec<Row>>, String>>,
    at: HashMap<SharedString, usize>,
    /// Mail rows only; drafts and the Outbox have none.
    pub facts: HashMap<SharedString, Fact>,
    pub selected: Vec<SharedString>,
    /// The one message picked, for the reading pane.
    pub chosen: Option<Row>,
}

impl Listing {
    pub fn show(&mut self, mails: Option<Result<Indexed, String>>) {
        self.at = HashMap::new();
        self.facts = HashMap::new();
        self.mails = mails.map(|read| {
            read.map(|indexed| {
                self.at = indexed.at;
                self.facts = indexed.facts;
                Rc::new(indexed.rows)
            })
        });
    }

    /// The picked messages' facts, in list order.
    pub fn picked(&self) -> Vec<Fact> {
        let facts = self.selected.iter().filter_map(|key| self.facts.get(key));
        facts.copied().collect()
    }

    /// Keeps the picks still listed.
    fn pick(&mut self, keys: Vec<SharedString>) {
        let Some(Ok(mails)) = &self.mails else {
            self.selected.clear();
            self.chosen = None;
            return;
        };
        self.selected = keys
            .into_iter()
            .filter(|key| self.at.contains_key(key))
            .collect();
        self.chosen = match &self.selected[..] {
            [key] => Some(mails[self.at[key]].clone()),
            _ => None,
        };
    }
}

impl Mail {
    pub fn open_box(&mut self, open: Open, cx: &mut Context<Self>) {
        log::info!("mail: open {}", open.key());
        if open != self.open {
            self.open = open;
            if let Open::Account(id) = open {
                self.read_identity(id, cx);
                self.read_rules(id, cx);
                self.read_drafts_up(id, cx);
                self.read_sent_copy(id, cx);
            }
            if open == Open::Contacts {
                self.read_contacts(cx);
            }
            self.listing.show(None);
            self.listing.pick(Vec::new());
            self.stop_seeking(cx);
            self.read_letter(cx);
        }
        self.read_mail(cx);
        self.notify(cx);
    }

    pub fn set_filter(&mut self, filter: Filter, cx: &mut Context<Self>) {
        log::info!("mail: filter {filter:?}");
        self.filter = filter;
        self.read_mail(cx);
        cx.notify();
    }

    pub fn select(&mut self, keys: &[SharedString], cx: &mut Context<Self>) {
        self.listing.pick(keys.to_vec());
        self.read_letter(cx);
        self.read_on_open(cx);
        cx.notify();
    }

    /// Unread counts, and the open list when it lists mail.
    pub fn read_mail(&mut self, cx: &mut Context<Self>) {
        self.listing.read += 1;
        let (read, open, filter) = (self.listing.read, self.open, self.filter);
        let scope = open.scope();
        let search = match scope.map(|_| self.seeking.search(filter, cx)).transpose() {
            Ok(search) => search.flatten(),
            Err(error) => {
                log::warn!("mail: a search's dates failed: {error}");
                self.listing.show(Some(Err(error)));
                self.listing.pick(Vec::new());
                return cx.notify();
            }
        };
        let none = (
            text(cx, "mail.no_sender", &[]),
            text(cx, "mail.no_subject", &[]),
        );
        let names = none.clone();
        let searched = search.is_some();
        self.with_store(
            cx,
            move |store| {
                let (mut kept, mut unsearched) = (Vec::new(), None);
                let rows = match scope {
                    Some(scope) => {
                        let list = match &search {
                            Some(search) => {
                                let found = store.search(search)?;
                                unsearched = Some(found.unsearched);
                                found.messages
                            }
                            None => store.list(scope, filter)?,
                        };
                        let facts = list.iter().map(|each| (key(each), fact(each))).collect();
                        let rows = list.into_iter().map(|each| row(each, &none)).collect();
                        Some(Indexed {
                            facts,
                            ..index(rows)
                        })
                    }
                    None if open == Open::Drafts => {
                        kept = store.drafts()?;
                        Some(index(draft_list::rows(&kept, &none)))
                    }
                    None => None,
                };
                let queued = match open {
                    Open::Outbox => store.outbox()?,
                    _ => Vec::new(),
                };
                let unread = store.unread()?;
                Ok((rows, unread, kept, queued, store.pending()?, unsearched))
            },
            move |mail, read_now, cx| {
                if mail.listing.read != read {
                    return log::info!("mail: list read {read} ended after a newer one");
                }
                match read_now {
                    Ok((rows, unread, kept, queued, pending, unsearched)) => {
                        mail.seeking.unsearched = unsearched;
                        mail.unread = unread;
                        mail.shelved = kept;
                        mail.pending = pending;
                        let rows = match open {
                            Open::Outbox => Some(index(outbox_list::rows(&queued, &names, cx))),
                            _ => rows,
                        };
                        mail.queued = queued;
                        let shown = rows.as_ref().map_or(0, |rows| rows.rows.len());
                        log::info!("mail: list read {read} shows {shown} messages, searched {searched}");
                        mail.listing.show(rows.map(Ok));
                    }
                    Err(error) => {
                        log::warn!("mail: list read {read} failed: {error}");
                        mail.seeking.unsearched = None;
                        let listed = scope.is_some() || matches!(open, Open::Drafts | Open::Outbox);
                        mail.listing.show(listed.then_some(Err(error)));
                    }
                }
                let keys = std::mem::take(&mut mail.listing.selected);
                mail.listing.pick(keys);
                mail.read_letter(cx);
            },
        );
    }

    /// Unread mail in every account's inbox.
    pub fn inboxes_unread(&self) -> u32 {
        self.syncs
            .values()
            .flat_map(|syncing| &syncing.folders)
            .filter(|folder| folder.role == Some(Role::Inbox))
            .filter_map(|folder| self.unread.get(&folder.id))
            .sum()
    }
}

fn key(summary: &Summary) -> SharedString {
    summary.id.0.to_string().into()
}

fn fact(summary: &Summary) -> Fact {
    Fact {
        id: summary.id,
        account: summary.account,
        mailbox: summary.mailbox,
        role: summary.role,
        pop: summary.pop,
        seen: summary.seen,
        flagged: summary.flagged,
    }
}

/// `none` names a missing sender, then subject.
fn row(summary: Summary, none: &(SharedString, SharedString)) -> Row {
    let key = key(&summary);
    let from = summary.from.first().map(|from| match &from.name {
        Some(name) if !name.trim().is_empty() => name.clone(),
        _ => from.email.clone(),
    });
    let from: SharedString = from.map_or_else(|| none.0.clone(), Into::into);
    let subject: SharedString = summary.subject.map_or_else(|| none.1.clone(), Into::into);
    let row = match summary.date.map(jiff::Timestamp::from_second) {
        Some(Ok(at)) => Row::new(key, from, subject, at),
        Some(Err(error)) => {
            log::warn!("mail: message {} date unshown: {error}", summary.id.0);
            Row::undated(key, from, subject)
        }
        None => Row::undated(key, from, subject),
    };
    let row = row.unread(!summary.seen).starred(summary.flagged);
    match summary.attachments {
        Some(count) => row.attachments(count as usize),
        None => row,
    }
}
