//! Sheet protection and locked cells (TABLE-15).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::{Input, Switch, TextInput},
    overlays::Dialog as Card,
    primitives::Severity,
};
use ely_lok::sheet::{self, PROTECT_OPTIONS, Protection};
use ely_office::prelude::*;
use gpui::{
    App, AppContext as _, Context, Div, Entity, InteractiveElement as _, ParentElement as _,
    Styled as _, Window, actions, div,
};

use crate::book::Book;
use crate::dialog::{Dialog, edit};

actions!(table, [ProtectSheet, LockCells, UnlockCells]);

pub fn bind(root: Div, cx: &mut Context<Book>) -> Div {
    root.on_action(cx.listener(|book, _: &ProtectSheet, _, cx| book.ask_protect(cx)))
        .on_action(cx.listener(|book, _: &LockCells, _, cx| book.run_active("cells.lock", &["true"], cx)))
        .on_action(
            cx.listener(|book, _: &UnlockCells, _, cx| book.run_active("cells.lock", &["false"], cx)),
        )
}

/// The protection dialog's state.
pub struct Protecting {
    pub now: Protection,
    pub password: Entity<TextInput>,
    pub confirm: Entity<TextInput>,
    pub allow: Vec<String>,
}

impl Book {
    pub(crate) fn ask_protect(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.core.active() else {
            return self.warn_key("command.blocked.no_document", cx);
        };
        if !self.settled(id, cx) {
            return self.when_settled(id, |book, _, cx| book.ask_protect(cx));
        }
        self.ask_in(
            id,
            |worker, doc| sheet::protection(&worker.values(doc, sheet::PROTECTION)?),
            move |book, read, window, cx| match read {
                Ok(now) => {
                    let mut masked = |key: &str, cx: &mut Context<Self>| {
                        let name = Book::text(cx, key, &[]);
                        cx.new(|cx| TextInput::new(window, cx).masked().label(name))
                    };
                    let protecting = Protecting {
                        allow: now.allow.clone(),
                        password: masked("table.protect.password", cx),
                        confirm: masked("table.protect.confirm", cx),
                        now,
                    };
                    book.open_dialog(Dialog::Protect {
                        tab: id,
                        protecting: Box::new(protecting),
                    });
                    cx.notify();
                }
                Err(error) => book.engine_failed(&error.to_string(), cx),
            },
            cx,
        );
    }

    /// Protects or unprotects as the dialog says.
    pub(crate) fn protect_asked(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Protect { tab, protecting }) = &self.dialog else {
            return;
        };
        let (tab, password) = (*tab, protecting.password.read(cx).text().to_string());
        if !protecting.now.protected {
            if password != protecting.confirm.read(cx).text() {
                return self.warn_key("table.protect.mismatch", cx);
            }
            let json = sheet::protect_json(&password, &protecting.allow);
            log::info!("table: protects tab {tab}, allows {:?}", protecting.allow);
            self.dialog = None;
            cx.notify();
            return self.run_in(Some(tab), "sheet.protect", &[&json], cx);
        }
        self.dialog = None;
        cx.notify();
        log::info!("table: unprotects tab {tab}");
        let json = sheet::unprotect_json(&password);
        self.read_in(
            tab,
            move |worker, doc| match worker.run(doc, "sheet.protect", &[&json]) {
                Ok(()) => Ok(true),
                // Still protected: the password was wrong.
                Err(error) => match sheet::protection(&worker.values(doc, sheet::PROTECTION)?)? {
                    Protection { protected: true, .. } => Ok(false),
                    _ => Err(error),
                },
            },
            |book, done, _, cx| match done {
                Ok(true) => {}
                Ok(false) => {
                    let text = Book::text(cx, "table.protect.wrong", &[]);
                    book.toast(text, Severity::Danger, cx);
                }
                Err(error) => book.engine_failed(&error.to_string(), cx),
            },
            cx,
        );
    }
}

/// Protects with options, or asks the password back.
pub fn protect_dialog(
    this: Entity<Book>,
    protecting: &Protecting,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let label = |key: &str| Book::text(cx, key, &[]);
    let input = |field| div().w_full().max_w_48().child(Input::new(field));
    let protected = protecting.now.protected;
    let (title, go) = match protected {
        true => ("table.protect.off_title", "table.protect.off"),
        false => ("table.protect.on_title", "table.protect.on"),
    };
    let mut card = Card::new("table-protect", label(title), dismiss);
    if !protected {
        let switches = PROTECT_OPTIONS.iter().enumerate().map(|(at, option)| {
            let toggling = this.clone();
            let on = protecting.allow.iter().any(|allowed| allowed == option);
            Switch::new(("table-protect-allow", at), on)
                .label(label(&format!("table.protect.{option}")))
                .on_change(move |on, _, cx| {
                    edit(&toggling, cx, |dialog, _, _| {
                        if let Dialog::Protect { protecting, .. } = dialog {
                            protecting.allow.retain(|allowed| allowed != option);
                            if on {
                                protecting.allow.push(option.to_string());
                            }
                        }
                    })
                })
        });
        card = card
            .child(
                DescriptionList::new()
                    .item(label("table.protect.password"), input(&protecting.password))
                    .item(label("table.protect.confirm"), input(&protecting.confirm)),
            )
            .child(div().flex().flex_col().gap_2().children(switches));
    } else if protecting.now.password {
        card = card.child(
            DescriptionList::new().item(label("table.protect.password"), input(&protecting.password)),
        );
    }
    let go = label(go);
    card.action(ely_office::cancel(cx)).action(move |_| {
        Button::new("table-protect-go", go)
            .variant(ButtonVariant::Primary)
            .on_click(move |_, _, cx| this.update(cx, |book, cx| book.protect_asked(cx)))
    })
}
