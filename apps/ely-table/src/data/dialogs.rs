//! The data dialogs: sort, filter, duplicates, columns.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::{Checkbox, CheckboxGroup, Choice, Input, InputEvent, SearchInput, Switch, TextInput},
    layout::ScrollArea,
    overlays::{Close, Dialog as Card},
    primitives::Severity,
    tables::{FilterBuilder, FilterRule, SortBuilder, SortKey},
    theme::ControlSize,
};
use ely_lok::sheet::{self, Entry, Selection};
use ely_office::prelude::*;
use gpui::{App, AppContext as _, Entity, ParentElement as _, SharedString, Styled as _, Window, div};

use super::title;
use crate::book::Book;
use crate::dialog::{Dialog, edit};

/// The kernel sorts by up to three keys.
const KEYS: usize = 3;

/// The primary button that runs what the dialog asks.
fn go(
    this: Entity<Book>,
    (id, label): (&'static str, SharedString),
    off: bool,
    run: fn(&mut Book, &mut gpui::Context<Book>),
) -> impl FnOnce(Close) -> Button + 'static {
    move |_| {
        Button::new(id, label)
            .variant(ButtonVariant::Primary)
            .disabled(off)
            .on_click(move |_, _, cx| this.update(cx, run))
    }
}

fn header_switch(this: &Entity<Book>, id: &'static str, on: bool, cx: &App) -> Switch {
    let this = this.clone();
    Switch::new(id, on)
        .label(Book::text(cx, "table.data.header", &[]))
        .on_change(move |on, _, cx| {
            edit(&this, cx, |dialog, _, _| match dialog {
                Dialog::Sort { header, .. } | Dialog::Dedupe { header, .. } => *header = on,
                _ => {}
            })
        })
}

pub fn sort_dialog(
    this: Entity<Book>,
    (picked, keys, header): (&Selection, &[SortKey], bool),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let (first, _, last, _) = picked.area.expect("a data area");
    let columns: Vec<_> = (first..=last)
        .map(|column| (column.to_string(), title(picked, column, header)))
        .collect();
    let keyed = this.clone();
    let builder = SortBuilder::new("table-sort-keys", columns)
        .keys(keys.to_vec())
        .on_change(move |keys, _, cx| {
            let mut keys = keys.to_vec();
            let over = keys.len() > KEYS;
            keys.truncate(KEYS);
            edit(&keyed, cx, |dialog, book, cx| {
                if let Dialog::Sort { keys: held, .. } = dialog {
                    *held = keys;
                }
                if over {
                    let text = Book::text(cx, "table.data.three", &[]);
                    book.toast(text, Severity::Info, cx);
                }
            })
        });
    let label = Book::text(cx, "table.data.sort_go", &[]);
    Card::new(
        "table-sort",
        Book::text(cx, "table.data.sort_title", &[]),
        dismiss,
    )
    .child(
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(header_switch(&this, "table-sort-header", header, cx))
            .child(builder),
    )
    .action(ely_office::cancel(cx))
    .action(go(
        this,
        ("table-sort-go", label),
        keys.is_empty(),
        Book::data_asked,
    ))
}

pub fn filter_dialog(
    this: Entity<Book>,
    (picked, rules, any): (&Selection, &[FilterRule], bool),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let (first, _, last, _) = picked.area.expect("a data area");
    let columns: Vec<_> = (first..=last)
        .map(|column| (column.to_string(), title(picked, column, true)))
        .collect();
    let ruled = this.clone();
    let builder = FilterBuilder::new("table-filter-rules", columns)
        .rules(rules.to_vec(), any)
        .on_change(move |rules, any, _, cx| {
            let rules = rules.to_vec();
            edit(&ruled, cx, |dialog, _, _| {
                if let Dialog::Filter {
                    rules: held,
                    any: joined,
                    ..
                } = dialog
                {
                    *held = rules;
                    *joined = any;
                }
            })
        });
    let label = Book::text(cx, "table.data.filter_go", &[]);
    Card::new(
        "table-filter",
        Book::text(cx, "table.data.filter_title", &[]),
        dismiss,
    )
    .child(builder)
    .action(ely_office::cancel(cx))
    .action(go(
        this,
        ("table-filter-go", label),
        rules.is_empty(),
        Book::data_asked,
    ))
}

pub fn dedupe_dialog(
    this: Entity<Book>,
    (picked, header): (&Selection, bool),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let detail = Book::text(cx, "table.data.dedupe_detail", &[("range", &picked.range)]);
    let label = Book::text(cx, "table.data.dedupe_go", &[]);
    let removing = this.clone();
    Card::new(
        "table-dedupe",
        Book::text(cx, "table.data.dedupe_title", &[]),
        dismiss,
    )
    .detail(detail)
    .child(header_switch(&this, "table-dedupe-header", header, cx))
    .action(ely_office::cancel(cx))
    .action(move |_| {
        Button::new("table-dedupe-go", label)
            .variant(ButtonVariant::Danger)
            .on_click(move |_, _, cx| removing.update(cx, |book, cx| book.data_asked(cx)))
    })
}

/// Separators as kernel character codes.
const SEPARATORS: [(&str, &str); 4] = [
    ("9", "table.columns.tab"),
    ("44", "table.columns.comma"),
    ("59", "table.columns.semicolon"),
    ("32", "table.columns.space"),
];

pub fn columns_dialog(
    this: Entity<Book>,
    (separators, other): (&[SharedString], &Entity<TextInput>),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let choices = SEPARATORS
        .iter()
        .map(|(code, key)| Choice::new(*code, Book::text(cx, key, &[])));
    let picked = this.clone();
    let group = CheckboxGroup::new("table-columns-separators", choices)
        .selected(separators.to_vec())
        .on_change(move |values, _, cx| {
            let values = values.to_vec();
            edit(&picked, cx, |dialog, _, _| {
                if let Dialog::Columns { separators, .. } = dialog {
                    *separators = values;
                }
            })
        });
    let other_row = div()
        .flex()
        .items_center()
        .gap_2()
        .child(Book::text(cx, "table.columns.other", &[]))
        .child(div().w_16().child(Input::new(other).size(ControlSize::Sm)));
    let none = separators.is_empty() && other.read(cx).text().is_empty();
    let label = Book::text(cx, "table.columns.go", &[]);
    Card::new(
        "table-columns",
        Book::text(cx, "table.columns.title", &[]),
        dismiss,
    )
    .child(
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(group)
            .child(other_row),
    )
    .action(ely_office::cancel(cx))
    .action(go(
        this,
        ("table-columns-go", label),
        none,
        Book::columns_asked,
    ))
}

/// The column menu's search: each change redraws the list.
pub(crate) fn search_field(window: &mut Window, cx: &mut gpui::Context<Book>) -> Entity<TextInput> {
    let search = cx.new(|cx| TextInput::new(window, cx));
    cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
        if *event == InputEvent::Changed {
            cx.notify();
        }
    })
    .detach();
    search
}

/// Entries whose label holds `search`, ignoring case.
fn shown<'a>(entries: &'a [Entry], search: &str, empty: &str) -> Vec<&'a Entry> {
    let search = search.trim().to_lowercase();
    entries
        .iter()
        .filter(|entry| {
            let label = if entry.text.is_empty() { empty } else { &entry.text };
            label.to_lowercase().contains(&search)
        })
        .collect()
}

/// `picked`, with only the shown entries set to `on`.
fn merged(all: &[SharedString], shown: &[SharedString], picked: &[SharedString], on: &[SharedString]) -> Vec<SharedString> {
    all.iter()
        .filter(|text| match shown.contains(text) {
            true => on.contains(text),
            false => picked.contains(text),
        })
        .cloned()
        .collect()
}

pub fn column_dialog(
    this: Entity<Book>,
    (column, entries, picked, search): (u32, &[Entry], &[SharedString], &Entity<TextInput>),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let empty = Book::text(cx, "table.filter.empty", &[]);
    let seen = shown(entries, search.read(cx).text(), &empty);
    let all: Vec<SharedString> = entries.iter().map(|entry| entry.text.clone().into()).collect();
    let visible: Vec<SharedString> = seen.iter().map(|entry| entry.text.clone().into()).collect();
    let choices = seen.iter().map(|entry| {
        let label = match entry.text.is_empty() {
            true => empty.clone(),
            false => entry.text.clone().into(),
        };
        Choice::new(entry.text.clone(), label)
    });
    let (checked, every_all, every_visible) = (this.clone(), all.clone(), visible.clone());
    let group = CheckboxGroup::new("table-column-entries", choices)
        .selected(picked.to_vec())
        .on_change(move |values, _, cx| {
            edit(&checked, cx, |dialog, _, _| {
                if let Dialog::Column { picked, .. } = dialog {
                    *picked = merged(&all, &visible, picked, values);
                }
            })
        });
    let every = this.clone();
    let whole_on = !every_visible.is_empty() && every_visible.iter().all(|text| picked.contains(text));
    let whole = Checkbox::new("table-column-all", whole_on)
        .label(Book::text(cx, "table.filter.all", &[]))
        .on_change(move |on, _, cx| {
            let on = if on { every_visible.clone() } else { Vec::new() };
            edit(&every, cx, |dialog, _, _| {
                if let Dialog::Column { picked, .. } = dialog {
                    *picked = merged(&every_all, &every_visible, picked, &on);
                }
            })
        });
    let list = match seen.is_empty() {
        true => div().text_sm().child(Book::text(cx, "table.filter.none", &[])),
        false => div().child(group),
    };
    let sort = |id: &'static str, key: &str, rising: bool| {
        let this = this.clone();
        Button::new(id, Book::text(cx, key, &[]))
            .size(ControlSize::Sm)
            .on_click(move |_, _, cx| {
                this.update(cx, |book, cx| book.column_asked(Some(rising), cx))
            })
    };
    let title = Book::text(
        cx,
        "table.filter.title",
        &[("column", &sheet::column_name(column))],
    );
    let label = Book::text(cx, "table.filter.go", &[]);
    let (applying, none) = (this.clone(), picked.is_empty());
    Card::new("table-column", title, dismiss)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(sort("table-column-up", "table.filter.up", true))
                        .child(sort("table-column-down", "table.filter.down", false)),
                )
                .child(SearchInput::new("table-column-search", search).size(ControlSize::Sm))
                .child(whole)
                .child(
                    div().h_64().child(
                        ScrollArea::new("table-column-scroll")
                            .size_full()
                            .child(list),
                    ),
                ),
        )
        .action(ely_office::cancel(cx))
        .action(move |_| {
            Button::new("table-column-go", label)
                .variant(ButtonVariant::Primary)
                .disabled(none)
                .on_click(move |_, _, cx| {
                    applying.update(cx, |book, cx| book.column_asked(None, cx))
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(texts: &[&str]) -> Vec<Entry> {
        texts.iter().map(|text| Entry { text: (*text).into(), hidden: false, selected: true }).collect()
    }

    fn strings(texts: &[&str]) -> Vec<SharedString> {
        texts.iter().map(|text| SharedString::from(*text)).collect()
    }

    #[test]
    fn the_search_keeps_entries_holding_it_in_any_case() {
        let list = entries(&["Beijing", "北京", "", "Shanghai", "jing"]);
        let texts = |search| shown(&list, search, "(Empty)").iter().map(|entry| entry.text.clone()).collect::<Vec<_>>();
        assert_eq!(texts("JING"), ["Beijing", "jing"]);
        assert_eq!(texts(" 北 "), ["北京"]);
        assert_eq!(texts("empty"), [""], "the blank entry by its label");
        assert_eq!(texts(""), ["Beijing", "北京", "", "Shanghai", "jing"]);
        assert!(texts("x").is_empty());
    }

    #[test]
    fn a_change_to_shown_entries_keeps_the_hidden_ticks() {
        let all = strings(&["a", "b", "c", "d"]);
        let picked = strings(&["a", "c"]);
        // b and c shown: b ticked, c unticked.
        assert_eq!(merged(&all, &strings(&["b", "c"]), &picked, &strings(&["b"])), strings(&["a", "b"]));
        // All, then none, over the shown only.
        assert_eq!(merged(&all, &strings(&["b", "d"]), &picked, &strings(&["b", "d"])), strings(&["a", "b", "c", "d"]));
        assert_eq!(merged(&all, &strings(&["a"]), &picked, &[]), strings(&["c"]));
    }
}
