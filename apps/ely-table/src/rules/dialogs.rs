//! The rule dialogs: validation, conditional format, the format list.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::{Choice, Input, Select, Switch, TextInput},
    overlays::Dialog as Card,
};
use ely_lok::sheet::{Format, Validation};
use gpui::{App, Entity, ParentElement as _, SharedString, Styled as _, Window, div};

use super::Fields;
use crate::book::Book;
use crate::dialog::{Dialog, edit};

/// What a cell may hold.
const KINDS: [(&str, &str); 8] = [
    ("any", "table.rules.any"),
    ("whole", "table.rules.whole"),
    ("decimal", "table.rules.decimal"),
    ("date", "table.rules.date"),
    ("time", "table.rules.time"),
    ("textlength", "table.rules.textlength"),
    ("list", "table.rules.list"),
    ("custom", "table.rules.custom"),
];

const OPERATORS: [(&str, &str); 8] = [
    ("between", "table.rules.between"),
    ("notbetween", "table.rules.notbetween"),
    ("equal", "table.rules.equal"),
    ("notequal", "table.rules.notequal"),
    ("greater", "table.rules.greater"),
    ("less", "table.rules.less"),
    ("greaterequal", "table.rules.greaterequal"),
    ("lessequal", "table.rules.lessequal"),
];

const ALERTS: [(&str, &str); 3] = [
    ("stop", "table.rules.stop"),
    ("warning", "table.rules.warning"),
    ("info", "table.rules.info"),
];

/// Conditions a cell rule tests; more than validation has.
const CONDITIONS: [(&str, &str); 18] = [
    ("greater", "table.rules.greater"),
    ("less", "table.rules.less"),
    ("between", "table.rules.between"),
    ("notbetween", "table.rules.notbetween"),
    ("equal", "table.rules.equal"),
    ("notequal", "table.rules.notequal"),
    ("greaterequal", "table.rules.greaterequal"),
    ("lessequal", "table.rules.lessequal"),
    ("duplicate", "table.rules.duplicate"),
    ("unique", "table.rules.unique"),
    ("contains", "table.rules.contains"),
    ("beginswith", "table.rules.beginswith"),
    ("endswith", "table.rules.endswith"),
    ("top10", "table.rules.top"),
    ("bottom10", "table.rules.bottom"),
    ("aboveaverage", "table.rules.above_average"),
    ("belowaverage", "table.rules.below_average"),
    ("error", "table.rules.error"),
];

/// The kernel's cell styles a rule can apply.
const STYLES: [(&str, &str); 10] = [
    ("Good", "table.style.good"),
    ("Neutral", "table.style.neutral"),
    ("Bad", "table.style.bad"),
    ("Warning", "table.style.warning"),
    ("Error", "table.style.error"),
    ("Accent", "table.style.accent"),
    ("Accent 1", "table.style.accent1"),
    ("Accent 2", "table.style.accent2"),
    ("Accent 3", "table.style.accent3"),
    ("Note", "table.style.note"),
];

const FORMATS: [(&str, &str); 3] = [
    ("cell", "table.rules.cell"),
    ("scale", "table.rules.scale"),
    ("databar", "table.rules.databar"),
];

/// Color scales: label, low to high.
pub(super) const SCALES: [(&str, &[&str]); 4] = [
    ("table.rules.scale_rgy", &["#F8696B", "#FFEB84", "#63BE7B"]),
    ("table.rules.scale_gyr", &["#63BE7B", "#FFEB84", "#F8696B"]),
    ("table.rules.scale_wr", &["#FFFFFF", "#F8696B"]),
    ("table.rules.scale_wg", &["#FFFFFF", "#63BE7B"]),
];

/// Data bar colors.
pub(super) const BARS: [(&str, &str); 4] = [
    ("table.rules.bar_blue", "#638EC6"),
    ("table.rules.bar_green", "#63C384"),
    ("table.rules.bar_red", "#FF555A"),
    ("table.rules.bar_orange", "#FFB628"),
];

/// The table's value for a picked choice.
fn find(table: &[(&'static str, &'static str)], value: &str) -> &'static str {
    table
        .iter()
        .find(|(key, _)| *key == value)
        .map(|(key, _)| *key)
        .expect("a listed choice")
}

/// A select over `table`, named `name`; `set` takes the pick.
fn pick(
    this: &Entity<Book>,
    (id, name): (&'static str, &str),
    (table, selected): (&[(&'static str, &'static str)], &str),
    set: fn(&mut Dialog, &'static str),
    cx: &App,
) -> Select {
    let choices = table
        .iter()
        .map(|(value, key)| Choice::new(*value, Book::text(cx, key, &[])));
    let (this, table) = (this.clone(), table.to_vec());
    Select::new(id, choices)
        .label(Book::text(cx, name, &[]))
        .selected(selected.to_string())
        .on_change(move |value, _, cx| {
            let value = find(&table, value);
            edit(&this, cx, |dialog, _, _| set(dialog, value))
        })
}

/// A switch; `set` takes its state.
fn switch(
    this: &Entity<Book>,
    id: &'static str,
    on: bool,
    set: fn(&mut Validation, bool),
) -> Switch {
    let this = this.clone();
    Switch::new(id, on).on_change(move |on, _, cx| {
        edit(&this, cx, |dialog, _, _| {
            if let Dialog::Validation { rule, .. } = dialog {
                set(rule, on);
            }
        })
    })
}

fn field(input: &Entity<TextInput>) -> gpui::Div {
    div().w_full().max_w_64().child(Input::new(input))
}

fn rule_of(dialog: &mut Dialog) -> &mut Validation {
    match dialog {
        Dialog::Validation { rule, .. } => rule,
        _ => unreachable!("a validation dialog"),
    }
}

pub fn validation_dialog(
    this: Entity<Book>,
    (rule, fields): (&Validation, &Fields),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let label = |key: &str| Book::text(cx, key, &[]);
    let mut rows = DescriptionList::new().item(
        label("table.rules.allow"),
        pick(
            &this,
            ("table-rule-kind", "table.rules.allow"),
            (&KINDS, &rule.kind),
            |dialog, kind| rule_of(dialog).kind = kind.into(),
            cx,
        ),
    );
    match rule.kind.as_str() {
        "whole" | "decimal" | "date" | "time" | "textlength" => {
            rows = rows
                .item(
                    label("table.rules.data"),
                    pick(
                        &this,
                        ("table-rule-operator", "table.rules.data"),
                        (&OPERATORS, &rule.operator),
                        |dialog, op| rule_of(dialog).operator = op.into(),
                        cx,
                    ),
                )
                .item(label("table.rules.value"), field(&fields.value1));
            if matches!(rule.operator.as_str(), "between" | "notbetween") {
                rows = rows.item(label("table.rules.and"), field(&fields.value2));
            }
        }
        "list" => {
            rows = rows
                .item(label("table.rules.entries"), field(&fields.list))
                .item(
                    label("table.rules.dropdown"),
                    switch(&this, "table-rule-dropdown", rule.dropdown, |rule, on| {
                        rule.dropdown = on
                    }),
                );
        }
        "custom" => rows = rows.item(label("table.rules.formula"), field(&fields.value1)),
        _ => {}
    }
    if rule.kind != "any" {
        rows = rows.item(
            label("table.rules.blank"),
            switch(&this, "table-rule-blank", rule.ignore_blank, |rule, on| {
                rule.ignore_blank = on
            }),
        );
    }
    rows = rows.item(
        label("table.rules.help"),
        switch(&this, "table-rule-help", rule.show_input, |rule, on| {
            rule.show_input = on
        }),
    );
    if rule.show_input {
        rows = rows
            .item(label("table.rules.title"), field(&fields.input_title))
            .item(label("table.rules.message"), field(&fields.input_message));
    }
    if rule.kind != "any" {
        rows = rows.item(
            label("table.rules.alert"),
            switch(&this, "table-rule-alert", rule.show_error, |rule, on| {
                rule.show_error = on
            }),
        );
        if rule.show_error {
            rows = rows
                .item(
                    label("table.rules.style"),
                    pick(
                        &this,
                        ("table-rule-style", "table.rules.style"),
                        (&ALERTS, &rule.error_style),
                        |dialog, style| rule_of(dialog).error_style = style.into(),
                        cx,
                    ),
                )
                .item(label("table.rules.title"), field(&fields.error_title))
                .item(label("table.rules.message"), field(&fields.error_message));
        }
    }
    let go = label("table.rules.apply");
    Card::new("table-rule", label("table.rules.validation_title"), dismiss)
        .child(rows)
        .action(ely_office::cancel(cx))
        .action(move |_| {
            Button::new("table-rule-go", go)
                .variant(ButtonVariant::Primary)
                .on_click(move |_, _, cx| this.update(cx, |book, cx| book.rule_asked(cx)))
        })
}

/// The open conditional dialog's parts.
pub struct Shown<'a> {
    pub kind: &'a str,
    pub operator: &'a str,
    pub style: &'a str,
    pub preset: usize,
    pub value1: &'a Entity<TextInput>,
    pub value2: &'a Entity<TextInput>,
}

fn set_conditional(
    dialog: &mut Dialog,
    change: impl FnOnce(&mut &'static str, &mut &'static str, &mut &'static str, &mut usize),
) {
    if let Dialog::Conditional {
        kind,
        operator,
        style,
        preset,
        ..
    } = dialog
    {
        change(kind, operator, style, preset);
    }
}

pub fn conditional_dialog(
    this: Entity<Book>,
    shown: Shown,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let label = |key: &str| Book::text(cx, key, &[]);
    let mut rows = DescriptionList::new().item(
        label("table.rules.kind"),
        pick(
            &this,
            ("table-format-kind", "table.rules.kind"),
            (&FORMATS, shown.kind),
            |dialog, value| {
                set_conditional(dialog, |kind, _, _, preset| {
                    *kind = value;
                    *preset = 0;
                })
            },
            cx,
        ),
    );
    match shown.kind {
        "scale" | "databar" => {
            let table: Vec<(SharedString, SharedString)> = if shown.kind == "scale" {
                SCALES
                    .iter()
                    .enumerate()
                    .map(|(at, (key, _))| (at.to_string().into(), label(key)))
                    .collect()
            } else {
                BARS.iter()
                    .enumerate()
                    .map(|(at, (key, _))| (at.to_string().into(), label(key)))
                    .collect()
            };
            let picked = this.clone();
            let presets = Select::new(
                "table-format-preset",
                table
                    .into_iter()
                    .map(|(value, text)| Choice::new(value, text)),
            )
                .label(label("table.rules.colors"))
            .selected(shown.preset.to_string())
            .on_change(move |value, _, cx| {
                let at: usize = value.parse().expect("presets are indexes");
                edit(&picked, cx, |dialog, _, _| {
                    set_conditional(dialog, |_, _, _, preset| *preset = at)
                })
            });
            rows = rows.item(label("table.rules.colors"), presets);
        }
        _ => {
            rows = rows.item(
                label("table.rules.condition"),
                pick(
                    &this,
                    ("table-format-operator", "table.rules.condition"),
                    (&CONDITIONS, shown.operator),
                    |dialog, value| set_conditional(dialog, |_, operator, _, _| *operator = value),
                    cx,
                ),
            );
            match shown.operator {
                "duplicate" | "unique" | "aboveaverage" | "belowaverage" | "error" => {}
                "between" | "notbetween" => {
                    rows = rows
                        .item(label("table.rules.value"), field(shown.value1))
                        .item(label("table.rules.and"), field(shown.value2));
                }
                _ => rows = rows.item(label("table.rules.value"), field(shown.value1)),
            }
            rows = rows.item(
                label("table.rules.cell_style"),
                pick(
                    &this,
                    ("table-format-style", "table.rules.cell_style"),
                    (&STYLES, shown.style),
                    |dialog, value| set_conditional(dialog, |_, _, style, _| *style = value),
                    cx,
                ),
            );
        }
    }
    let go = label("table.rules.add");
    Card::new(
        "table-format",
        label("table.rules.conditional_title"),
        dismiss,
    )
    .child(rows)
    .action(ely_office::cancel(cx))
    .action(move |_| {
        Button::new("table-format-go", go)
            .variant(ButtonVariant::Primary)
            .on_click(move |_, _, cx| this.update(cx, |book, cx| book.conditional_asked(cx)))
    })
}

/// One format as the list shows it.
fn entry(format: &Format, cx: &App) -> SharedString {
    let kind = FORMATS
        .iter()
        .find(|(value, _)| *value == format.kind)
        .map_or_else(
            || format.kind.clone().into(),
            |(_, key)| Book::text(cx, key, &[]),
        );
    let style = STYLES
        .iter()
        .find(|(value, _)| *value == format.style)
        .map_or_else(
            || format.style.clone().into(),
            |(_, key)| Book::text(cx, key, &[]),
        );
    let args = [
        ("range", format.range.as_str()),
        ("kind", kind.as_ref()),
        ("style", style.as_ref()),
        ("value", format.value.as_str()),
    ];
    match format.kind.as_str() {
        "cell" => Book::text(cx, "table.rules.entry_cell", &args),
        _ => Book::text(cx, "table.rules.entry", &args),
    }
}

pub fn formats_dialog(
    this: Entity<Book>,
    (formats, chosen): (&[Format], usize),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let choices = formats
        .iter()
        .enumerate()
        .map(|(at, format)| Choice::new(at.to_string(), entry(format, cx)));
    let picked = this.clone();
    let list = Select::new("table-formats-pick", choices)
        .label(Book::text(cx, "table.rules.formats_title", &[]))
        .selected(chosen.to_string())
        .on_change(move |value, _, cx| {
            let at: usize = value.parse().expect("formats are listed by index");
            edit(&picked, cx, |dialog, _, _| {
                if let Dialog::Formats { chosen, .. } = dialog {
                    *chosen = at;
                }
            })
        });
    let go = Book::text(cx, "table.rules.remove", &[]);
    Card::new(
        "table-formats",
        Book::text(cx, "table.rules.formats_title", &[]),
        dismiss,
    )
    .child(list)
    .action(ely_office::cancel(cx))
    .action(move |_| {
        Button::new("table-formats-remove", go)
            .variant(ButtonVariant::Danger)
            .on_click(move |_, _, cx| this.update(cx, |book, cx| book.format_removed(cx)))
    })
}
