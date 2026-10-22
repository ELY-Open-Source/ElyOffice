//! An account's local rules, on its page (C12.18, MAIL-11).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, IconButton},
    feedback::Alert,
    forms::{Choice, Input, Select, TextInput},
    primitives::{IconName, Severity},
    theme::ControlSize,
};
use ely_mail_engine::{AccountId, Action, MailboxId, Match, Protocol, Role, Rule, RuleId};
use gpui::{
    AnyElement, App, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, Styled as _, Window, div,
};

use crate::mail::{Mail, text};

const FIELDS: [(&str, &str, Match); 3] = [
    ("from", "mail.rules.from", Match::From),
    ("to", "mail.rules.to", Match::To),
    ("subject", "mail.rules.subject", Match::Subject),
];

/// The open account's rules, and the one being made.
pub struct Ruling {
    pub account: AccountId,
    /// Numbers the read; the words box starts over with it.
    pub read: u64,
    pub rules: Option<Result<Vec<Rule>, String>>,
    pub field: Match,
    /// "flag", "archive", or a folder's id to move to.
    pub action: SharedString,
}

impl Mail {
    pub fn read_rules(&mut self, account: AccountId, cx: &mut Context<Self>) {
        self.asked += 1;
        let read = self.asked;
        self.ruling = Some(Ruling { account, read, rules: None, field: Match::From, action: "flag".into() });
        self.with_store(cx, move |store| store.rules(account), move |mail, rules, cx| {
            if let Some(ruling) = mail.ruling.as_mut().filter(|each| each.read == read) {
                ruling.rules = Some(rules);
            }
            mail.notify(cx);
        });
    }

    /// The rule the controls name, or why it cannot be.
    fn chosen_action(ruling: &Ruling) -> Option<Action> {
        match ruling.action.as_ref() {
            "flag" => Some(Action::Flag),
            "archive" => Some(Action::Archive),
            folder => folder.parse().ok().map(|id| Action::Move(MailboxId(id))),
        }
    }

    pub(crate) fn add_rule(&mut self, words: String, cx: &mut Context<Self>) {
        let Some(ruling) = &self.ruling else {
            return log::error!("mail: a rule was asked for with no account open");
        };
        let (account, field) = (ruling.account, ruling.field);
        let Some(action) = Self::chosen_action(ruling) else {
            return log::error!("mail: a rule's action is not one offered");
        };
        self.with_store(cx, move |store| store.add_rule(account, field, &words, action), move |mail, added, cx| match added {
            Ok(_) => mail.reread_rules(account, cx),
            Err(error) => mail.failed("mail.rules.unsaved", &error, cx),
        });
    }

    fn remove_rule(&mut self, id: RuleId, cx: &mut Context<Self>) {
        let Some(account) = self.ruling.as_ref().map(|ruling| ruling.account) else {
            return;
        };
        self.with_store(cx, move |store| store.remove_rule(id), move |mail, removed, cx| match removed {
            Ok(()) => mail.reread_rules(account, cx),
            Err(error) => mail.failed("mail.rules.unsaved", &error, cx),
        });
    }

    /// Only while the page still shows `account`.
    fn reread_rules(&mut self, account: AccountId, cx: &mut Context<Self>) {
        match self.ruling.as_ref().is_some_and(|ruling| ruling.account == account) {
            true => self.read_rules(account, cx),
            false => log::info!("mail: rules saved for an account no longer shown"),
        }
    }

    /// Each rule with its remove button, then the add controls.
    pub fn rules_editor(
        &self,
        account: AccountId,
        protocol: Protocol,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let ruling = self.ruling.as_ref().filter(|each| each.account == account)?;
        let rules = match ruling.rules.as_ref()? {
            Ok(rules) => rules,
            Err(error) => {
                let words = text(cx, "mail.rules.unread", &[("error", error)]);
                return Some(Alert::new("mail-rules-unread", Severity::Danger, words).into_any_element());
            }
        };
        let folders = self.syncs.get(&account).map(|syncing| syncing.folders.as_slice()).unwrap_or_default();
        let this = cx.weak_entity();
        let rows: Vec<_> = rules.iter().map(|rule| {
            let said = said(rule, cx);
            let (id, this) = (rule.id, this.clone());
            let remove = IconButton::new(("mail-rule-remove", rule.id.0 as u64), IconName::X)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(text(cx, "mail.rules.remove", &[]))
                .on_click(move |_, _, cx| {
                    this.update(cx, |mail, cx| mail.remove_rule(id, cx)).ok();
                });
            div()
                .debug_selector(move || format!("mail-rule-{}", id.0))
                .flex()
                .items_center()
                .gap_2()
                .child(div().flex_1().min_w_0().child(said))
                .child(div().debug_selector(move || format!("mail-rule-remove-{}", id.0)).child(remove))
        }).collect();
        let words = window.use_keyed_state(("mail-rule-words", ruling.read), cx, |window, cx| {
            let hint = text(cx, "mail.rules.words", &[]);
            TextInput::new(window, cx).placeholder(hint.clone()).label(hint)
        });
        Some(
            div()
                .debug_selector(|| "mail-rules".into())
                .flex()
                .flex_col()
                .gap_2()
                .children(rows)
                .child(self.rule_controls(ruling, protocol, folders, &words, cx))
                .into_any_element(),
        )
    }

    fn rule_controls(
        &self,
        ruling: &Ruling,
        protocol: Protocol,
        folders: &[ely_mail_engine::LocalFolder],
        words: &Entity<TextInput>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let this = cx.weak_entity();
        let fields = FIELDS.map(|(key, words, _)| Choice::new(key, text(cx, words, &[])));
        let picked = FIELDS.iter().find(|(_, _, field)| *field == ruling.field).map_or("from", |(key, ..)| key);
        let field = Select::new("mail-rule-field", fields)
            .label(text(cx, "mail.rules.field", &[]))
            .selected(picked.to_owned())
            .size(ControlSize::Sm)
            .on_change({
                let this = this.clone();
                move |value, _, cx| {
                    let Some((_, _, field)) = FIELDS.iter().find(|(key, ..)| *key == value.as_ref()) else {
                        return log::error!("mail: no rule field {value}");
                    };
                    this.update(cx, |mail, cx| {
                        if let Some(ruling) = mail.ruling.as_mut() {
                            ruling.field = *field;
                        }
                        mail.notify(cx);
                    })
                    .ok();
                }
            });
        let action = Select::new("mail-rule-action", rule_actions(protocol, folders, cx))
            .label(text(cx, "mail.rules.action", &[]))
            .selected(ruling.action.clone())
            .size(ControlSize::Sm)
            .on_change({
                let this = this.clone();
                move |value, _, cx| {
                    this.update(cx, |mail, cx| {
                        if let Some(ruling) = mail.ruling.as_mut() {
                            ruling.action = value.clone();
                        }
                        mail.notify(cx);
                    })
                    .ok();
                }
            });
        let typed = words.clone();
        let add = Button::new("mail-rule-add", text(cx, "mail.rules.add", &[]))
            .size(ControlSize::Sm)
            .on_click(move |_, _, cx| {
                let words = typed.read(cx).text().to_owned();
                this.update(cx, |mail, cx| mail.add_rule(words, cx)).ok();
            });
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(field)
            .child(div().debug_selector(|| "mail-rule-words".into()).w_40().child(Input::new(words)))
            .child(action)
            .child(div().debug_selector(|| "mail-rule-add".into()).child(add))
            .into_any_element()
    }
}

/// "From has “shop”: move to Receipts".
pub(crate) fn said(rule: &Rule, cx: &App) -> SharedString {
    let (_, field, _) = FIELDS.iter().find(|(_, _, field)| *field == rule.field).expect("every field is listed");
    let action = match (rule.action, &rule.folder) {
        (Action::Flag, _) => text(cx, "mail.rules.flag", &[]),
        (Action::Archive, _) => text(cx, "mail.rules.archive", &[]),
        (Action::Move(_), Some(name)) => text(cx, "mail.rules.move", &[("name", name)]),
        (Action::Move(target), None) => unreachable!("the store joins rule {} to folder {}", rule.id.0, target.0),
    };
    let field = text(cx, field, &[]);
    text(cx, "mail.rules.said", &[("field", &field), ("words", &rule.words), ("action", &action)])
}

/// A rule flags; on IMAP it also archives or moves.
pub(crate) fn rule_actions(protocol: Protocol, folders: &[ely_mail_engine::LocalFolder], cx: &App) -> Vec<Choice> {
    let mut actions = vec![Choice::new("flag", text(cx, "mail.rules.flag", &[]))];
    // POP3 keeps one folder: it can only flag.
    if protocol == Protocol::Imap {
        // Archiving needs an Archive or All Mail folder.
        let archives = |folder: &ely_mail_engine::LocalFolder| !folder.local && matches!(folder.role, Some(Role::Archive | Role::All));
        if folders.iter().any(archives) {
            actions.push(Choice::new("archive", text(cx, "mail.rules.archive", &[])));
        }
        // An imported folder is not on the server.
        actions.extend(folders.iter().filter(|folder| !folder.local).map(|folder| {
            Choice::new(folder.id.0.to_string(), text(cx, "mail.rules.move", &[("name", &folder.shown)]))
        }));
    }
    actions
}
