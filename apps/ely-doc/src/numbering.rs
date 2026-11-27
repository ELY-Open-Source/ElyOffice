//! Right-click an automatic number, as Word does (#147).

use ely_gpui_component::menus::{Menu, MenuItem};
use ely_office::prelude::*;
use gpui::{App, Entity, Window};

use crate::docs::Docs;
use crate::props::levels::{ARABIC, CHINESE, CIRCLED, LOWER_LETTER, LOWER_ROMAN, UPPER_LETTER, UPPER_ROMAN};
use crate::refs::Ask;

/// The item's own number; empty off a numbered item.
const NUMBER: &str = ".uno:ElyNumberFrom";
/// Enabled on a list's first item after a list, or a restart.
const JOIN: &str = ".uno:ElyNumberJoin";

/// One item's formats: numbering type and pattern, `#` its number.
const FORMATS: [(u8, &str); 12] = [
    (ARABIC, "#."),
    (ARABIC, "#)"),
    (ARABIC, "(#)"),
    (UPPER_ROMAN, "#."),
    (LOWER_ROMAN, "#."),
    (UPPER_LETTER, "#."),
    (LOWER_LETTER, "#)"),
    (CIRCLED, "#"),
    (CHINESE, "#、"),
    (CHINESE, "（#）"),
    (ARABIC, "第#条"),
    (CHINESE, "第#条"),
];

/// How the first number of `kind` reads.
fn first(kind: u8) -> &'static str {
    match kind {
        UPPER_ROMAN => "I",
        LOWER_ROMAN => "i",
        UPPER_LETTER => "A",
        LOWER_LETTER => "a",
        CIRCLED => "①",
        CHINESE => "一",
        _ => "1",
    }
}

/// A format's label: its pattern with the first number.
pub(crate) fn sample((kind, pattern): (u8, &str)) -> String {
    pattern.replace('#', first(kind))
}

/// The cursor's numbered item: its number, and whether it continues.
pub(crate) struct Numbered {
    number: String,
    joins: bool,
}

impl Docs {
    /// The active tab's numbered item, if the cursor is on one.
    pub(crate) fn numbered(&self, cx: &App) -> Option<Numbered> {
        let view = self.active_tab()?.view.read(cx);
        let number = view.state(NUMBER).filter(|number| !number.is_empty())?.to_string();
        Some(Numbered { number, joins: view.state(JOIN) == Some("enabled") })
    }
}

/// The submenu for a numbered item.
pub(crate) fn submenu(this: &Entity<Docs>, Numbered { number, joins }: Numbered, cx: &App) -> MenuItem {
    let text = |key: &str| Docs::text(cx, key, &[]);
    let run = |id: &'static str, value: Option<String>| {
        let this = this.clone();
        move |_: &mut Window, cx: &mut App| {
            log::info!("doc: the menu numbers with {id}");
            this.update(cx, |docs, cx| match &value {
                Some(value) => docs.run_active(id, &[value.as_str()], cx),
                None => docs.run_active(id, &[], cx),
            })
        }
    };
    let styles = FORMATS.into_iter().fold(Menu::new(), |menu, format| {
        let (kind, pattern) = format;
        menu.item(MenuItem::new(sample(format)).on_click(run("list.number_format", Some(format!("{kind}|{pattern}")))))
    });
    let ask = {
        let this = this.clone();
        move |window: &mut Window, cx: &mut App| {
            this.update(cx, |docs, cx| docs.ask(Ask::Number, &number, window, cx))
        }
    };
    let rows = Menu::new()
        .item(MenuItem::new(text("doc.menu.numbering.restart")).on_click(run("list.number_from", Some("1".into()))))
        .item(MenuItem::new(text("doc.menu.numbering.continue")).disabled(!joins).on_click(run("list.join", None)))
        .item(MenuItem::new(text("doc.menu.numbering.set_value")).on_click(ask))
        .item(MenuItem::submenu(text("doc.menu.numbering.style"), styles));
    MenuItem::submenu(text("doc.menu.numbering"), rows)
}

/// Why a typed value cannot go, if it cannot.
pub(crate) fn refuse(value: &str) -> Option<&'static str> {
    match value.parse::<u16>() {
        Ok(1..=9999) => None,
        _ => Some("doc.number.invalid"),
    }
}

#[cfg(test)]
mod tests;
