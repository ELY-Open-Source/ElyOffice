//! Every inbox, then each account's folders, down the left.

use ely_gpui_component::{
    layout::ScrollArea,
    mail::{Mailbox, MailboxList},
    menus::{ContextMenu, Menu, MenuItem},
    primitives::IconName,
};
use ely_mail_engine::{AccountId, LocalFolder, MailboxId, Role};
use gpui::{
    App, Context, IntoElement, ParentElement as _, Pixels, Point, Render, SharedString,
    Styled as _, WeakEntity, Window, div,
};

use ely_ui::menu::Offer;

use crate::{
    boxes::Open,
    mail::{Mail, text},
};

/// The folder a menu was asked for.
struct Asked {
    account: AccountId,
    mailbox: MailboxId,
    pinned: bool,
    local: bool,
    name: SharedString,
}

pub struct Side {
    mail: WeakEntity<Mail>,
    asked: Option<Offer<Asked>>,
}

impl Side {
    pub fn new(mail: WeakEntity<Mail>) -> Self {
        Self { mail, asked: None }
    }

    /// A folder's pin menu, where the right press fell.
    fn ask(&mut self, key: &SharedString, at: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(mail) = self.mail.upgrade() else {
            return;
        };
        let mail = mail.read(cx);
        let Some(Open::Mailbox(account, mailbox)) = mail.opened(key) else {
            return log::info!("mail: no menu for a box that is no folder");
        };
        let folders = mail
            .syncs
            .get(&account)
            .map_or(&[][..], |each| &each.folders);
        let Some(folder) = folders.iter().find(|folder| folder.id == mailbox) else {
            return log::error!("mail: a menu for a folder not listed");
        };
        let asked = Asked {
            account,
            mailbox,
            pinned: folder.pinned,
            local: folder.local,
            name: folder_name(folder, cx),
        };
        self.asked = Some(Offer::new(self.asked.as_ref(), 0, asked, at));
        cx.notify();
    }

    fn menu(&self, asked: &Asked, cx: &App) -> Menu {
        let (account, mailbox, pinned) = (asked.account, asked.mailbox, asked.pinned);
        let (key, icon) = match pinned {
            true => ("mail.boxes.unpin", IconName::PinOff),
            false => ("mail.boxes.pin", IconName::Pin),
        };
        let this = self.mail.clone();
        let item = MenuItem::new(text(cx, key, &[]))
            .icon(icon)
            .on_click(move |_, cx| {
                this.update(cx, |mail, cx| mail.pin(account, mailbox, !pinned, cx))
                    .ok();
            });
        let (this, name) = (self.mail.clone(), asked.name.clone());
        let export = MenuItem::new(text(cx, "mail.mbox.export", &[]))
            .icon(IconName::Upload)
            .on_click(move |_, cx| {
                this.update(cx, |mail, cx| mail.export_mbox(mailbox, &name, cx)).ok();
            });
        let mut menu = Menu::new().item(item).item(export);
        if asked.local {
            let (this, name) = (self.mail.clone(), asked.name.clone());
            let remove = MenuItem::new(text(cx, "mail.mbox.remove", &[]))
                .icon(IconName::Trash2)
                .on_click(move |_, cx| {
                    this.update(cx, |mail, cx| mail.ask_remove_imported(account, mailbox, name.clone(), cx)).ok();
                });
            menu = menu.item(remove);
        }
        menu
    }
}

impl Render for Side {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(mail) = self.mail.upgrade() else {
            return div().into_any_element();
        };
        let mail = mail.read(cx);
        let inboxes = Mailbox::new(
            Open::Inboxes.key(),
            text(cx, "mail.boxes.all", &[]),
            IconName::Inbox,
        )
        .unread(mail.inboxes_unread());
        let drafts = Mailbox::new(
            Open::Drafts.key(),
            text(cx, "mail.boxes.drafts", &[]),
            IconName::File,
        );
        let outbox = Mailbox::new(
            Open::Outbox.key(),
            text(cx, "mail.boxes.outbox", &[]),
            IconName::Send,
        )
        .unread(mail.pending as u32);
        let contacts = Mailbox::new(
            Open::Contacts.key(),
            text(cx, "mail.boxes.contacts", &[]),
            IconName::Users,
        );
        let box_of = |id: AccountId, folder: &LocalFolder, name: SharedString| {
            let open = Open::Mailbox(id, folder.id);
            // Boxes with nothing unread are absent.
            let unread = mail.unread.get(&folder.id).copied().unwrap_or(0);
            let icon = match folder.local {
                true => IconName::HardDrive,
                false => icon(folder.role),
            };
            Mailbox::new(open.key(), name, icon).unread(unread)
        };
        let named = mail.accounts().len() > 1;
        let mut pinned = Vec::new();
        let mut accounts = Vec::new();
        for (id, account) in mail.accounts() {
            let folders = mail.syncs.get(id).map_or(&[][..], |each| &each.folders);
            let mut sorted: Vec<&LocalFolder> = folders.iter().collect();
            sorted.sort_by(|a, b| rank(a.role).cmp(&rank(b.role)).then(a.shown.cmp(&b.shown)));
            let mut boxes = Vec::new();
            for folder in sorted {
                let name = folder_name(folder, cx);
                match folder.pinned {
                    // Pinned: at the top, by its account's name.
                    true if named => {
                        let name = format!("{name} · {}", account.address);
                        pinned.push(box_of(*id, folder, name.into()));
                    }
                    true => pinned.push(box_of(*id, folder, name)),
                    false => boxes.push(box_of(*id, folder, name)),
                }
            }
            boxes.push(Mailbox::new(
                Open::Account(*id).key(),
                text(cx, "mail.boxes.settings", &[]),
                IconName::Settings,
            ));
            accounts.push((account.address.clone(), boxes));
        }
        let folded = mail
            .accounts()
            .iter()
            .filter(|(id, _)| mail.folded.contains(id));
        let folded: Vec<String> = folded.map(|(_, account)| account.address.clone()).collect();
        let mut list = MailboxList::new("mail-boxes").fixed_section(
            text(cx, "mail.boxes.unified", &[]),
            [inboxes, drafts, outbox, contacts].into_iter().chain(pinned),
        );
        for (address, boxes) in accounts {
            list = list.section(address, boxes);
        }
        let (this, folding, side) = (self.mail.clone(), self.mail.clone(), cx.weak_entity());
        let list = list
            .open(mail.open.key())
            .folded(folded)
            .on_open(move |key, _, cx| {
                this.update(cx, |mail, cx| match mail.opened(key) {
                    Some(open) => mail.open_box(open, cx),
                    None => log::error!("mail: the mailbox list sent an unknown box"),
                })
                .ok();
            })
            .on_fold(move |title, folded, _, cx| {
                folding
                    .update(cx, |mail, cx| mail.fold(title, folded, cx))
                    .ok();
            })
            .on_menu(move |key, at, _, cx| {
                side.update(cx, |side, cx| side.ask(key, at, cx)).ok();
            });
        let scroll = ScrollArea::new("mail-side-scroll")
            .vertical()
            .size_full()
            .child(div().p_2().child(list));
        // The host stays, so the list's focus survives the menu.
        let host = match &self.asked {
            Some(offer) => offer.host("mail-side-menu", self.menu(&offer.what, cx)),
            None => ContextMenu::new("mail-side-menu", Menu::new()).manual(None),
        };
        host.size_full()
            .child(scroll)
            .into_any_element()
    }
}

impl Mail {
    /// The box a side key names, if still here.
    pub(crate) fn opened(&self, key: &SharedString) -> Option<Open> {
        for open in [Open::Inboxes, Open::Drafts, Open::Outbox, Open::Contacts] {
            if *key == open.key() {
                return Some(open);
            }
        }
        self.accounts().iter().find_map(|(id, _)| {
            let folders = self.syncs.get(id).map_or(&[][..], |each| &each.folders);
            let boxes = folders.iter().map(|folder| Open::Mailbox(*id, folder.id));
            boxes
                .chain([Open::Account(*id)])
                .find(|open| open.key() == *key)
        })
    }

    /// Folds an account's section, by its address.
    fn fold(&mut self, address: &str, folded: bool, cx: &mut Context<Self>) {
        let found = self
            .accounts()
            .iter()
            .find(|(_, each)| each.address == address);
        let Some((id, _)) = found else {
            return log::error!("mail: the side folded a section that is no account");
        };
        let id = *id;
        self.with_store(
            cx,
            move |store| store.set_folded(id, folded),
            move |mail, done, cx| match done {
                Ok(()) if folded => _ = mail.folded.insert(id),
                Ok(()) => _ = mail.folded.remove(&id),
                Err(error) => mail.failed("mail.boxes.unsaved", &error, cx),
            },
        );
    }

    fn pin(
        &mut self,
        account: AccountId,
        mailbox: MailboxId,
        pinned: bool,
        cx: &mut Context<Self>,
    ) {
        self.with_store(
            cx,
            move |store| store.set_pinned(mailbox, pinned),
            move |mail, done, cx| match done {
                Ok(()) => mail.read_synced(account, cx),
                Err(error) => mail.failed("mail.boxes.unsaved", &error, cx),
            },
        );
    }
}

/// Inbox in the reader's words; others as named.
pub fn folder_name(folder: &LocalFolder, cx: &App) -> SharedString {
    match folder.role {
        _ if folder.local => text(cx, "mail.boxes.imported", &[("name", &folder.shown)]),
        Some(Role::Inbox) => text(cx, "mail.boxes.inbox", &[]),
        _ => folder.shown.clone().into(),
    }
}

fn rank(role: Option<Role>) -> u8 {
    match role {
        Some(Role::Inbox) => 0,
        Some(Role::Flagged) => 1,
        Some(Role::Drafts) => 2,
        Some(Role::Sent) => 3,
        Some(Role::Archive) => 4,
        Some(Role::All) => 5,
        Some(Role::Junk) => 6,
        Some(Role::Trash) => 7,
        None => 8,
    }
}

fn icon(role: Option<Role>) -> IconName {
    match role {
        Some(Role::Inbox) => IconName::Inbox,
        Some(Role::Flagged) => IconName::Star,
        Some(Role::Drafts) => IconName::File,
        Some(Role::Sent) => IconName::Send,
        Some(Role::Archive) => IconName::Archive,
        Some(Role::All) => IconName::Mail,
        Some(Role::Junk) => IconName::TriangleAlert,
        Some(Role::Trash) => IconName::Trash2,
        None => IconName::Folder,
    }
}
