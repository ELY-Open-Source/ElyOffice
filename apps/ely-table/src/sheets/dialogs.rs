//! The sheet dialogs: rename, show, delete (TABLE-01).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::{Choice, Select, TextInput},
    overlays::{Dialog as Card, PromptDialog},
};
use ely_lok::sheet::References;
use ely_office::prelude::*;
use gpui::{App, Entity, ParentElement as _, SharedString, Window};

use super::Dialog;
use crate::book::Book;

/// Why `name` cannot name a sheet, as a message key.
fn refused(name: &str, names: &[String]) -> Option<&'static str> {
    if name.is_empty() {
        return Some("table.rename.empty");
    }
    let bad = name.contains([':', '\\', '/', '?', '*', '[', ']'])
        || name.starts_with('\'')
        || name.ends_with('\'');
    if bad {
        return Some("table.rename.invalid");
    }
    names
        .iter()
        .any(|taken| taken.to_lowercase() == name.to_lowercase())
        .then_some("table.rename.taken")
}

pub(crate) fn rename(
    this: Entity<Book>,
    (index, field, names): (usize, &Entity<TextInput>, Vec<String>),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> PromptDialog {
    let current = names.get(index).cloned().unwrap_or_default();
    let others: Vec<String> = names.into_iter().filter(|name| *name != current).collect();
    let title = Book::text(cx, "table.rename.title", &[("name", &current)]);
    let messages: Vec<(&str, SharedString)> = [
        "table.rename.empty",
        "table.rename.invalid",
        "table.rename.taken",
    ]
    .into_iter()
    .map(|key| (key, Book::text(cx, key, &[])))
    .collect();
    PromptDialog::new("table-rename", title, field, dismiss)
        .label(Book::text(cx, "table.rename.label", &[]))
        .submit(Book::text(cx, "table.size.apply", &[]))
        .check(move |text| match refused(text, &others) {
            Some(key) => Err(messages
                .iter()
                .find(|(known, _)| *known == key)
                .map(|(_, message)| message.clone())
                .expect("every refusal has a message")),
            None => Ok(()),
        })
        .on_submit(move |text, _, cx| {
            let name = text.to_string();
            this.update(cx, |book, cx| book.rename_asked(&name, cx))
        })
}

pub(crate) fn show(
    this: Entity<Book>,
    (tab, hidden, chosen): (u64, &[String], &str),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let choices = hidden
        .iter()
        .map(|name| Choice::new(name.clone(), name.clone()));
    let picked = this.clone();
    let select = Select::new("table-show-pick", choices)
        .label(Book::text(cx, "table.show.title", &[]))
        .selected(chosen.to_string())
        .on_change(move |value, _, cx| {
            picked.update(cx, |book, cx| {
                if let Some(Dialog::Show { chosen, .. }) = &mut book.dialog {
                    *chosen = value.to_string();
                }
                cx.notify();
            })
        });
    let label = Book::text(cx, "table.show.go", &[]);
    Card::new(
        "table-show",
        Book::text(cx, "table.show.title", &[]),
        dismiss,
    )
    .child(select)
    .action(ely_office::cancel(cx))
    .action(move |_| {
        Button::new("table-show-go", label)
            .variant(ButtonVariant::Primary)
            .on_click(move |_, _, cx| {
                this.update(cx, |book, cx| {
                    let Some(Dialog::Show { chosen, .. }) = book.dialog.take() else {
                        return;
                    };
                    book.run_in(Some(tab), "sheet.show", &[&chosen], cx);
                })
            })
    })
}

pub(crate) fn delete(
    this: Entity<Book>,
    (name, references): (&str, &References),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let mut detail = if references.formulas + references.names == 0 {
        Book::text(cx, "table.delete.alone", &[])
    } else {
        let count = |key, n: u64| ely_ui::i18n::counted(cx, key, n, &[("n", &n.to_string())]);
        let formulas = count("table.delete.formulas", references.formulas);
        let names = count("table.delete.names", references.names);
        let args = [("formulas", formulas.as_ref()), ("first", &references.first), ("names", &names)];
        Book::text(cx, "table.delete.refs", &args)
    };
    if references.dynamic > 0 {
        let count = references.dynamic.to_string();
        let dynamic = ely_ui::i18n::counted(cx, "table.delete.dynamic", references.dynamic, &[("count", &count)]);
        detail = format!("{detail} {dynamic}").into();
    }
    let label = Book::text(cx, "table.delete.go", &[]);
    let title = Book::text(cx, "table.delete.title", &[("name", name)]);
    Card::new("table-delete", title, dismiss)
        .detail(detail)
        .action(ely_office::cancel(cx))
        .action(move |_| {
            Button::new("table-delete-go", label)
                .variant(ButtonVariant::Danger)
                .on_click(move |_, _, cx| this.update(cx, |book, cx| book.delete_asked(cx)))
        })
}
