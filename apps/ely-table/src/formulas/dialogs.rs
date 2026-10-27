//! The name dialogs: define, and the list (TABLE-06).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::{Choice, Input, Select, TextInput},
    overlays::Dialog as Card,
};
use ely_lok::sheet::Name;
use gpui::{App, Entity, ParentElement as _, SharedString, Styled as _, Window, div};

use super::NameAct;
use crate::book::Book;
use crate::dialog::Dialog;

/// What the scope list shows: the workbook, then each sheet.
fn scope_label(scope: &str, cx: &App) -> SharedString {
    if scope.is_empty() {
        Book::text(cx, "table.names.workbook", &[])
    } else {
        scope.to_string().into()
    }
}

pub fn define_dialog(
    this: Entity<Book>,
    (name, range, scope, sheets): (&Entity<TextInput>, &Entity<TextInput>, &str, Vec<String>),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let scopes = std::iter::once(String::new())
        .chain(sheets)
        .map(|scope| Choice::new(scope.clone(), scope_label(&scope, cx)));
    let picked = this.clone();
    let scope = Select::new("table-define-scope", scopes)
        .label(Book::text(cx, "table.names.scope", &[]))
        .selected(scope.to_string())
        .on_change(move |value, _, cx| {
            picked.update(cx, |book, cx| {
                if let Some(Dialog::Define { scope, .. }) = &mut book.dialog {
                    *scope = value.to_string();
                }
                cx.notify();
            })
        });
    let fields = DescriptionList::new()
        .item(
            Book::text(cx, "table.names.name", &[]),
            div().w_full().max_w_64().child(Input::new(name)),
        )
        .item(
            Book::text(cx, "table.names.refers", &[]),
            div().w_full().max_w_64().child(Input::new(range)),
        )
        .item(Book::text(cx, "table.names.scope", &[]), scope);
    let empty = name.read(cx).text().trim().is_empty() || range.read(cx).text().trim().is_empty();
    let label = Book::text(cx, "table.names.go_define", &[]);
    Card::new(
        "table-define",
        Book::text(cx, "table.names.define_title", &[]),
        dismiss,
    )
    .child(fields)
    .action(ely_office::cancel(cx))
    .action(move |_| {
        Button::new("table-define-go", label)
            .variant(ButtonVariant::Primary)
            .disabled(empty)
            .on_click(move |_, _, cx| this.update(cx, |book, cx| book.define(cx)))
    })
}

/// One name as the list shows it.
fn entry(name: &Name, cx: &App) -> SharedString {
    let args = [
        ("name", name.name.as_str()),
        ("scope", &scope_label(&name.scope, cx)),
        ("formula", name.formula.as_str()),
    ];
    let key = if name.broken {
        "table.names.entry_broken"
    } else {
        "table.names.entry"
    };
    Book::text(cx, key, &args)
}

pub fn names_dialog(
    this: Entity<Book>,
    (names, chosen, range): (&[Name], usize, &Entity<TextInput>),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let choices = names
        .iter()
        .enumerate()
        .map(|(index, name)| Choice::new(index.to_string(), entry(name, cx)));
    let picked = this.clone();
    let list = Select::new("table-names-pick", choices)
        .label(Book::text(cx, "table.names.name", &[]))
        .selected(chosen.to_string())
        .on_change(move |value, _, cx| {
            let index: usize = value.parse().expect("names are listed by index");
            picked.update(cx, |book, cx| book.choose_name(index, cx))
        });
    let fields = DescriptionList::new()
        .item(Book::text(cx, "table.names.name", &[]), list)
        .item(
            Book::text(cx, "table.names.refers", &[]),
            div().w_full().max_w_64().child(Input::new(range)),
        );
    let nowhere = names[chosen].broken || names[chosen].range.is_empty();
    let act = |id: &'static str, key: &str, act: NameAct, variant, off: bool| {
        let (this, label) = (this.clone(), Book::text(cx, key, &[]));
        move |_| {
            Button::new(id, label)
                .variant(variant)
                .disabled(off)
                .on_click(move |_, _, cx| this.update(cx, |book, cx| book.act_on_name(act, cx)))
        }
    };
    Card::new(
        "table-names",
        Book::text(cx, "table.names.manage_title", &[]),
        dismiss,
    )
    .child(fields)
    .action(act(
        "table-names-remove",
        "table.names.remove",
        NameAct::Remove,
        ButtonVariant::Danger,
        false,
    ))
    .action(act(
        "table-names-go",
        "table.names.goto",
        NameAct::Go,
        ButtonVariant::Secondary,
        nowhere,
    ))
    .action(act(
        "table-names-apply",
        "table.names.apply",
        NameAct::Apply,
        ButtonVariant::Primary,
        false,
    ))
}
