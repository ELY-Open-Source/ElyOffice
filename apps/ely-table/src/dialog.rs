//! ElyTable's one open dialog.

use ely_gpui_component::{
    forms::TextInput,
    tables::{FilterRule, SortKey},
};
use ely_lok::sheet::{Entry, Format, References, Selection, Validation};
use gpui::{AnyElement, App, Context, Entity, IntoElement, SharedString, Window};

use crate::book::Book;
use crate::{analysis, data, rules};

/// Changes the open dialog's state.
pub(crate) fn edit(
    this: &Entity<Book>,
    cx: &mut App,
    change: impl FnOnce(&mut Dialog, &mut Book, &mut Context<Book>),
) {
    this.update(cx, |book, cx| {
        if let Some(mut dialog) = book.dialog.take() {
            change(&mut dialog, book, cx);
            book.dialog = Some(dialog);
        }
        cx.notify();
    });
}

/// What the open dialog asks, for which workbook.
pub enum Dialog {
    RMacro(Box<crate::macros::Editor>),
    Rename {
        tab: u64,
        /// The state the sheet number was read in.
        at: ely_lok::Doc,
        index: usize,
        field: Entity<TextInput>,
    },
    Show {
        tab: u64,
        hidden: Vec<String>,
        chosen: String,
    },
    Delete {
        tab: u64,
        /// The state the sheet number was read in.
        at: ely_lok::Doc,
        index: usize,
        name: String,
        references: References,
    },
    Size {
        tab: u64,
        column: bool,
        field: Entity<TextInput>,
    },
    Merge {
        tab: u64,
    },
    Define {
        tab: u64,
        name: Entity<TextInput>,
        range: Entity<TextInput>,
        scope: String,
    },
    Names {
        tab: u64,
        names: Vec<ely_lok::sheet::Name>,
        chosen: usize,
        range: Entity<TextInput>,
    },
    Sort {
        tab: u64,
        picked: Selection,
        keys: Vec<SortKey>,
        header: bool,
    },
    Filter {
        tab: u64,
        picked: Selection,
        rules: Vec<FilterRule>,
        any: bool,
    },
    Dedupe {
        tab: u64,
        picked: Selection,
        header: bool,
        state: crate::data::ReadState,
    },
    Columns {
        tab: u64,
        separators: Vec<SharedString>,
        other: Entity<TextInput>,
    },
    /// A column button's filter menu.
    Column {
        tab: u64,
        column: u32,
        /// The button's row: its cell names the filtered range.
        row: u32,
        entries: Vec<Entry>,
        picked: Vec<SharedString>,
        /// Narrows the entries shown; hidden ones keep their tick.
        search: Entity<TextInput>,
    },
    Validation {
        tab: u64,
        rule: Box<Validation>,
        fields: Box<crate::rules::Fields>,
    },
    Conditional {
        tab: u64,
        kind: &'static str,
        operator: &'static str,
        style: &'static str,
        preset: usize,
        value1: Entity<TextInput>,
        value2: Entity<TextInput>,
    },
    Formats {
        tab: u64,
        formats: Vec<Format>,
        chosen: usize,
    },
    Pivot {
        tab: u64,
        source: String,
        fields: Vec<String>,
        layout: ely_lok::sheet::Layout,
        /// Changes the pivot table at the cursor.
        editing: bool,
    },
    Chart {
        tab: u64,
        /// The selected chart; none inserts one.
        chart: Option<Box<ely_lok::sheet::Chart>>,
        look: Box<ely_lok::sheet::Look>,
        fields: Box<crate::analysis::ChartFields>,
    },
    /// A text file before it opens.
    Import(Box<crate::text::Pending>),
    Export {
        tab: u64,
        format: crate::output::Format,
    },
    Page {
        tab: u64,
        layout: Box<ely_lok::sheet::PrintLayout>,
        landscape: bool,
        scale: ely_lok::sheet::Scale,
        fields: Box<crate::page::PageFields>,
    },
    Protect {
        tab: u64,
        protecting: Box<crate::protect::Protecting>,
    },
}

impl Dialog {
    /// The workbook it asks about; imports have none yet.
    pub fn tab(&self) -> Option<u64> {
        match self {
            Dialog::Rename { tab, .. }
            | Dialog::Show { tab, .. }
            | Dialog::Delete { tab, .. }
            | Dialog::Size { tab, .. }
            | Dialog::Merge { tab, .. }
            | Dialog::Define { tab, .. }
            | Dialog::Names { tab, .. }
            | Dialog::Sort { tab, .. }
            | Dialog::Filter { tab, .. }
            | Dialog::Dedupe { tab, .. }
            | Dialog::Columns { tab, .. }
            | Dialog::Column { tab, .. }
            | Dialog::Validation { tab, .. }
            | Dialog::Conditional { tab, .. }
            | Dialog::Formats { tab, .. }
            | Dialog::Pivot { tab, .. }
            | Dialog::Chart { tab, .. }
            | Dialog::Export { tab, .. }
            | Dialog::Page { tab, .. }
            | Dialog::Protect { tab, .. } => Some(*tab),
            Dialog::RMacro(editor) => Some(editor.tab),
            Dialog::Import(_) => None,
        }
    }
}

impl Book {
    /// The define dialog's sheets: its own workbook's.
    pub(crate) fn define_scopes(&self) -> Option<Vec<String>> {
        let Some(Dialog::Define { tab, .. }) = &self.dialog else {
            return None;
        };
        self.sheet_in(*tab).map(|(_, _, names)| names)
    }

    /// ElyTable's open dialog, if any.
    pub fn dialog_element(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let this = cx.entity();
        let dismiss = {
            let this = this.clone();
            move |_: &mut Window, cx: &mut App| {
                this.update(cx, |book, cx| {
                    book.dialog = None;
                    cx.notify();
                })
            }
        };
        let element = match self.dialog.as_ref()? {
            Dialog::RMacro(editor) => crate::macros::dialog(this, editor, dismiss, cx).into_any_element(),
            Dialog::Rename { tab, index, field, .. } => {
                let names = self.sheet_in(*tab)?.2;
                crate::sheets::dialogs::rename(this, (*index, field, names), dismiss, cx)
                    .into_any_element()
            }
            Dialog::Show {
                tab,
                hidden,
                chosen,
            } => crate::sheets::dialogs::show(this, (*tab, hidden, chosen), dismiss, cx)
                .into_any_element(),
            Dialog::Delete {
                name, references, ..
            } => crate::sheets::dialogs::delete(this, (name, references), dismiss, cx)
                .into_any_element(),
            Dialog::Size { column, field, .. } => {
                crate::cells::size_dialog(this, (*column, field), dismiss, cx).into_any_element()
            }
            Dialog::Merge { tab } => {
                crate::cells::merge_dialog(this, *tab, dismiss, cx).into_any_element()
            }
            Dialog::Define {
                name, range, scope, ..
            } => crate::formulas::define_dialog(
                this,
                (name, range, scope, self.define_scopes()?),
                dismiss,
                cx,
            )
            .into_any_element(),
            Dialog::Names {
                names,
                chosen,
                range,
                ..
            } => crate::formulas::names_dialog(this, (names, *chosen, range), dismiss, cx)
                .into_any_element(),
            Dialog::Sort {
                picked,
                keys,
                header,
                ..
            } => data::sort_dialog(this, (picked, keys, *header), dismiss, cx).into_any_element(),
            Dialog::Filter {
                picked, rules, any, ..
            } => data::filter_dialog(this, (picked, rules, *any), dismiss, cx).into_any_element(),
            Dialog::Dedupe { picked, header, .. } => {
                data::dedupe_dialog(this, (picked, *header), dismiss, cx).into_any_element()
            }
            Dialog::Columns {
                separators, other, ..
            } => data::columns_dialog(this, (separators, other), dismiss, cx).into_any_element(),
            Dialog::Column {
                column,
                entries,
                picked,
                search,
                ..
            } => data::column_dialog(this, (*column, entries, picked, search), dismiss, cx)
                .into_any_element(),
            Dialog::Validation { rule, fields, .. } => {
                rules::validation_dialog(this, (rule, fields), dismiss, cx).into_any_element()
            }
            Dialog::Conditional {
                kind,
                operator,
                style,
                preset,
                value1,
                value2,
                ..
            } => {
                let shown = rules::Shown {
                    kind,
                    operator,
                    style,
                    preset: *preset,
                    value1,
                    value2,
                };
                rules::conditional_dialog(this, shown, dismiss, cx).into_any_element()
            }
            Dialog::Formats {
                formats, chosen, ..
            } => rules::formats_dialog(this, (formats, *chosen), dismiss, cx).into_any_element(),
            Dialog::Pivot {
                source,
                fields,
                layout,
                editing,
                ..
            } => analysis::pivot_dialog(this, (source, fields, layout, *editing), dismiss, cx)
                .into_any_element(),
            Dialog::Chart {
                chart,
                look,
                fields,
                ..
            } => analysis::chart_dialog(this, (chart.as_deref(), look, fields), dismiss, cx)
                .into_any_element(),
            Dialog::Import(pending) => {
                crate::text::import_dialog(this, pending, dismiss, cx).into_any_element()
            }
            Dialog::Export { tab, format } => {
                let sheet = self.sheet_in(*tab)?.1;
                crate::output::export_dialog(this, (*format, &sheet), dismiss, cx)
                    .into_any_element()
            }
            Dialog::Page {
                layout,
                landscape,
                scale,
                fields,
                ..
            } => crate::page::page_dialog(this, (layout, *landscape, *scale, fields), dismiss, cx)
                .into_any_element(),
            Dialog::Protect { protecting, .. } => {
                crate::protect::protect_dialog(this, protecting, dismiss, cx).into_any_element()
            }
        };
        Some(element)
    }
}
