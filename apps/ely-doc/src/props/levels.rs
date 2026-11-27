//! Multilevel list schemes, as `list.levels` takes them (#130).

use ely_gpui_component::{
    forms::{Choice, Select},
    theme::ControlSize,
};
use ely_office::panel::Panel as _;
use gpui::{AnyElement, Context, IntoElement as _, SharedString};

use super::Props;
use crate::docs::Docs;

/// The kernel's list state: each level's "type|format".
pub(super) const LEVELS: &str = ".uno:ElyListLevels";

/// Levels in a Writer list.
const DEPTH: usize = 10;

/// css.style.NumberingType values; letters run z, aa, bb as Word's.
pub(crate) const UPPER_LETTER: u8 = 9;
pub(crate) const LOWER_LETTER: u8 = 10;
pub(crate) const UPPER_ROMAN: u8 = 2;
pub(crate) const LOWER_ROMAN: u8 = 3;
pub(crate) const ARABIC: u8 = 4;
pub(crate) const CIRCLED: u8 = 14;
pub(crate) const CHINESE: u8 = 15;

/// Each scheme's label key and repeating levels; `#` numbers.
pub(super) const SCHEMES: [(&str, &[(u8, &str)]); 5] = [
    ("doc.props.levels.decimal", &[]),
    ("doc.props.levels.paren", &[(ARABIC, "#."), (ARABIC, "(#)"), (CIRCLED, "#"), (LOWER_LETTER, "#)"), (LOWER_ROMAN, "#.")]),
    ("doc.props.levels.chinese", &[(CHINESE, "#、"), (CHINESE, "（#）"), (ARABIC, "#."), (ARABIC, "（#）"), (CIRCLED, "#")]),
    ("doc.props.levels.letters", &[(ARABIC, "#."), (LOWER_LETTER, "#."), (LOWER_ROMAN, "#.")]),
    ("doc.props.levels.outline", &[(UPPER_ROMAN, "#."), (UPPER_LETTER, "#."), (ARABIC, "#."), (LOWER_LETTER, "#."), (LOWER_ROMAN, "#.")]),
];

/// The scheme's ten levels as the command takes them.
pub(super) fn levels(cycle: &[(u8, &str)]) -> String {
    let level = |n: usize| match cycle {
        // Every level carries its parents: 1.1.1.
        [] => (ARABIC, (1..=n).map(|it| format!("%{it}%.")).collect()),
        _ => {
            let (kind, format) = cycle[(n - 1) % cycle.len()];
            (kind, format.replace('#', &format!("%{n}%")))
        }
    };
    (1..=DEPTH)
        .map(|n| {
            let (kind, format) = level(n);
            format!("{kind}|{format}")
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// The label key of the scheme that made `state`.
pub(super) fn scheme_of(state: &str) -> Option<&'static str> {
    SCHEMES.iter().find(|(_, cycle)| levels(cycle) == state).map(|(key, _)| *key)
}

impl Props {
    /// The scheme picker; other lists show as custom.
    pub(super) fn levels_menu(&self, cx: &Context<Self>) -> AnyElement {
        let this = cx.entity();
        let state = self.state(LEVELS, cx);
        let current = state.as_deref().and_then(scheme_of);
        let placeholder = match state.as_deref() {
            Some(ely_lok::MIXED) => Docs::text(cx, "office.props.mixed", &[]),
            Some(state) if !state.is_empty() && current.is_none() => {
                Docs::text(cx, "doc.props.list_levels.custom", &[])
            }
            _ => Docs::text(cx, "doc.props.choose", &[]),
        };
        let choices = SCHEMES
            .iter()
            .map(|(key, _)| Choice::new(SharedString::from(*key), Docs::text(cx, key, &[])));
        let mut menu = Select::new("doc-props-list-levels", choices)
            .label(Docs::text(cx, "doc.props.list_levels", &[]))
            .size(ControlSize::Sm)
            .disabled(state.is_none())
            .placeholder(placeholder)
            .on_change(move |key, _, cx| {
                let Some((_, cycle)) = SCHEMES.iter().find(|(it, _)| *it == key.as_ref()) else {
                    return;
                };
                this.update(cx, |props, cx| props.run("list.levels", &[&levels(cycle)], cx));
            });
        if let Some(key) = current {
            menu = menu.selected(SharedString::from(key));
        }
        menu.into_any_element()
    }
}
