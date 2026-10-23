//! The message list and the reading pane (C12.9).

use ely_gpui_component::{
    buttons::{ToggleGroup, ToggleItem},
    feedback::{EmptyState, ErrorView},
    layout::SplitPane,
    mail::MailList,
    primitives::IconName,
};
use ely_mail_engine::Filter;
use gpui::{
    AnyElement, Axis, Context, InteractiveElement as _, IntoElement, ParentElement as _,
    Styled as _, div, px,
};

use crate::{
    boxes::Open,
    mail::{Mail, text},
};

type Filtered = fn(Filter) -> bool;

const FILTERS: [(&str, &str, IconName, Filtered); 3] = [
    ("unread", "mail.filter.unread", IconName::Mail, |f| f.unread),
    ("flagged", "mail.filter.flagged", IconName::Star, |f| {
        f.flagged
    }),
    (
        "attachments",
        "mail.filter.attachments",
        IconName::Paperclip,
        |f| f.attachments,
    ),
];

impl Mail {
    pub fn panes(&self, cx: &mut Context<Self>) -> AnyElement {
        let split = SplitPane::new("mail-panes", Axis::Horizontal, px(240.))
            .on_resize(|shares, _, cx| ely_ui::settings::panels::set_split(shares[0], cx));
        let split = match ely_ui::settings::panels::split(cx) {
            Some(share) => split.sizes(&[share, 1. - share]),
            None => split,
        };
        split
            .pane(self.list_pane(cx))
            .pane(match self.open {
                Open::Drafts => self.draft_pane(cx),
                Open::Outbox => self.outbox_pane(cx),
                _ => div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .children(self.actions(cx))
                    .child(div().flex_1().min_h_0().child(self.reader(cx)))
                    .into_any_element(),
            })
            .into_any_element()
    }

    fn list_pane(&self, cx: &mut Context<Self>) -> AnyElement {
        let on: Vec<&str> = FILTERS
            .iter()
            .filter(|(_, _, _, on)| on(self.filter))
            .map(|(key, _, _, _)| *key)
            .collect();
        let toggles = FILTERS.iter().fold(
            ToggleGroup::new("mail-filters").multiple().selected(on),
            |group, (key, label, icon, _)| {
                let words = text(cx, label, &[]);
                group.item(ToggleItem::new(*key).icon(*icon).label(words))
            },
        );
        let this = cx.weak_entity();
        let toggles = toggles.on_change(move |keys, _, cx| {
            let has = |key: &str| keys.iter().any(|each| each == key);
            let filter = Filter {
                unread: has("unread"),
                flagged: has("flagged"),
                attachments: has("attachments"),
            };
            this.update(cx, |mail, cx| mail.set_filter(filter, cx)).ok();
        });
        let list = match &self.listing.mails {
            None => div().into_any_element(),
            Some(Err(detail)) => ErrorView::new(
                "mail-list-error",
                IconName::TriangleAlert,
                text(cx, "mail.list.unread", &[]),
            )
            .detail(detail.clone())
            .into_any_element(),
            Some(Ok(mails)) if mails.is_empty() && self.seeking.unsearched.is_some() => {
                let none = text(cx, "mail.search.none", &[]);
                EmptyState::new("mail-search-none", IconName::SearchX, none).into_any_element()
            }
            Some(Ok(mails)) => {
                let (this, starring) = (cx.weak_entity(), cx.weak_entity());
                let list = MailList::new("mail-list", mails.clone())
                    .selected(self.listing.selected.clone())
                    .on_select(move |keys, _, cx| {
                        this.update(cx, |mail, cx| mail.select(keys, cx)).ok();
                    });
                // Drafts and the Outbox have no stars.
                let starred = !matches!(self.open, Open::Drafts | Open::Outbox);
                let list = match starred {
                    true => list.on_star(move |key, on, _, cx| {
                        starring.update(cx, |mail, cx| mail.star(key, on, cx)).ok();
                    }),
                    false => list,
                };
                list.into_any_element()
            }
        };
        let lists_mail = !matches!(self.open, Open::Drafts | Open::Outbox);
        let searched = matches!(self.open, Open::Inboxes | Open::Mailbox(..));
        div()
            .debug_selector(|| "mail-list-pane".into())
            .size_full()
            .flex()
            .flex_col()
            .children(searched.then(|| self.search_bar(cx)))
            .children(lists_mail.then(|| {
                div()
                    .debug_selector(|| "mail-filters".into())
                    .px_2()
                    .pb_2()
                    .child(toggles)
            }))
            .child(div().flex_1().min_h_0().child(list))
            .into_any_element()
    }
}
