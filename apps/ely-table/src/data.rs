//! The Data menu: sort, filter, duplicates, columns, pastes (TABLE-08).

use ely_gpui_component::{
    forms::TextInput,
    primitives::Severity,
    tables::{FilterRule, SortKey, Test},
};
use ely_lok::{
    Request,
    sheet::{self, Entry, Selection},
};
use ely_office::prelude::*;
use ely_platform::clipboard::{self, Clip};
use gpui::{
    App, AppContext as _, Context, Div, InteractiveElement as _, Menu, MenuItem, SharedString,
    Window, actions,
};

use crate::book::Book;
use crate::dialog::Dialog;

/// The state a selection was read in, and its sheet.
pub(crate) type ReadState = (ely_lok::Doc, usize);

mod dialogs;
pub use dialogs::{column_dialog, columns_dialog, dedupe_dialog, filter_dialog, sort_dialog};

actions!(
    table,
    [
        Sort,
        AutoFilter,
        StandardFilter,
        ClearFilter,
        RemoveDuplicates,
        TextToColumns,
        PasteValues,
        PasteFormulas
    ]
);

pub fn menu(cx: &App) -> Menu {
    let label = |key: &str| Book::text(cx, key, &[]);
    Menu::new(label("menu.data")).items([
        MenuItem::action(label("table.data.sort"), Sort),
        MenuItem::separator(),
        MenuItem::action(label("table.data.autofilter"), AutoFilter),
        MenuItem::action(label("table.data.filter"), StandardFilter),
        MenuItem::action(label("table.data.unfilter"), ClearFilter),
        MenuItem::separator(),
        MenuItem::action(label("table.data.dedupe"), RemoveDuplicates),
        MenuItem::action(label("table.data.columns"), TextToColumns),
        MenuItem::separator(),
        MenuItem::action(label("office.find.open"), ely_office::FindReplace),
        MenuItem::action(label("table.data.paste_values"), PasteValues),
        MenuItem::action(label("table.data.paste_formulas"), PasteFormulas),
        MenuItem::separator(),
        MenuItem::action(label("table.rules.validation"), crate::rules::Validate),
        MenuItem::action(label("table.rules.unvalidate"), crate::rules::Unvalidate),
    ])
}

pub fn bind(root: Div, cx: &mut Context<Book>) -> Div {
    root.on_action(cx.listener(|book, _: &Sort, _, cx| book.ask_data(Asked::Sort, cx)))
        .on_action(cx.listener(|book, _: &StandardFilter, _, cx| book.ask_data(Asked::Filter, cx)))
        .on_action(
            cx.listener(|book, _: &RemoveDuplicates, _, cx| book.ask_data(Asked::Dedupe, cx)),
        )
        .on_action(cx.listener(|book, _: &TextToColumns, _, cx| book.ask_columns(cx)))
        .on_action(
            cx.listener(|book, _: &AutoFilter, _, cx| book.run_active("data.autofilter", &[], cx)),
        )
        .on_action(
            cx.listener(|book, _: &ClearFilter, _, cx| book.run_active("data.unfilter", &[], cx)),
        )
        .on_action(cx.listener(|book, _: &PasteValues, _, cx| book.paste_only("paste.values", cx)))
        .on_action(
            cx.listener(|book, _: &PasteFormulas, _, cx| book.paste_only("paste.formulas", cx)),
        )
}

/// Which data dialog to open.
#[derive(Clone, Copy, Debug)]
pub enum Asked {
    Sort,
    Filter,
    Dedupe,
}

/// The kernel's operator for a builder test.
fn op(test: Test) -> &'static str {
    match test {
        Test::Contains => "contains",
        Test::Equals => "equals",
        Test::NotEquals => "notequals",
        Test::Above => "greater",
        Test::Below => "less",
        Test::Empty => "empty",
    }
}

/// The `data.filter` query for builder rules.
pub fn query(rules: &[FilterRule], any: bool) -> String {
    let conditions: Vec<_> = rules
        .iter()
        .enumerate()
        .map(|(at, rule)| {
            serde_json::json!({
                "field": rule.column.parse::<u32>().expect("columns are indexes"),
                "op": op(rule.test),
                "value": rule.value.as_ref(),
                "or": any && at > 0,
            })
        })
        .collect();
    serde_json::json!({ "conditions": conditions }).to_string()
}

/// The `data.filter` query keeping `picked` of a column.
pub fn column_query(column: u32, entries: &[Entry], picked: &[SharedString]) -> String {
    if entries
        .iter()
        .all(|entry| picked.iter().any(|p| *p == entry.text))
    {
        return serde_json::json!({ "column": column }).to_string();
    }
    let values: Vec<&str> = picked.iter().map(SharedString::as_ref).collect();
    serde_json::json!({
        "column": column,
        "conditions": [{ "op": "values", "values": values }],
    })
    .to_string()
}

/// The `data.sort` values for up to three keys.
pub fn sort_values(header: bool, keys: &[SortKey]) -> Vec<String> {
    let mut values = vec![header.to_string()];
    for at in 0..3 {
        match keys.get(at) {
            Some(key) => {
                let column: u32 = key.column.parse().expect("columns are indexes");
                values.push((column + 1).to_string());
                values.push(key.rising.to_string());
            }
            None => values.extend(["0".to_string(), "true".to_string()]),
        }
    }
    values
}

/// A column's title: its header text, else its letters.
pub fn title(picked: &Selection, column: u32, header: bool) -> SharedString {
    let (first, ..) = picked.area.expect("a data area");
    let text = picked.top[(column - first) as usize].trim();
    if header && !text.is_empty() {
        text.to_string().into()
    } else {
        sheet::column_name(column).into()
    }
}

impl Book {
    pub(crate) fn warn_key(&self, key: &str, cx: &mut Context<Self>) {
        let text = Book::text(cx, key, &[]);
        self.toast(text, Severity::Warning, cx);
    }

    /// Reads tab `id`'s selection; a lone cell grows to data.
    pub(crate) fn data_of(
        &mut self,
        id: u64,
        grow: bool,
        then: impl FnOnce(&mut Self, Selection, ReadState, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        self.ask_in(
            id,
            move |worker, doc| {
                let read = |worker: &mut ely_lok::Worker| {
                    sheet::selected_range(&worker.values(doc, sheet::SELECTED)?)
                };
                let picked = match read(worker) {
                    // The kernel's data area around the cell.
                    Ok(Some(lone)) if grow && !lone.range.contains(':') => {
                        worker.run(doc, "select.data", &[])?;
                        let grown = read(worker)?;
                        // No data: unmark, or Enter stays in the cell.
                        if grown.as_ref().is_some_and(|picked| picked.area.is_none()) {
                            worker.run(doc, "cell.goto", &[&lone.range])?;
                        }
                        Ok(grown)
                    }
                    other => other,
                }?;
                // Numbers in the picked range hold in this state.
                Ok((picked, (worker.now(doc)?, doc.sheets(worker)?.current)))
            },
            move |book, read, window, cx| match read {
                Ok((Some(picked), state)) if picked.area.is_some() => then(book, picked, state, window, cx),
                Ok((Some(_), _)) => book.warn_key("table.data.empty", cx),
                Ok((None, _)) => book.warn_key("table.data.one_range", cx),
                Err(error) => book.engine_failed(&error.to_string(), cx),
            },
            cx,
        );
    }

    pub(crate) fn ask_data(&mut self, asked: Asked, cx: &mut Context<Self>) {
        let Some(id) = self.core.active else {
            return self.warn_key("command.blocked.no_document", cx);
        };
        self.data_in(id, asked, cx);
    }

    /// The data dialog for tab `id`, once its input lands.
    fn data_in(&mut self, id: u64, asked: Asked, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |book, _, cx| book.data_in(id, asked, cx));
        }
        self.data_of(
            id,
            true,
            move |book, picked, state, _, cx| book.data_dialog(id, asked, picked, state, cx),
            cx,
        );
    }

    fn data_dialog(&mut self, id: u64, asked: Asked, picked: Selection, state: ReadState, cx: &mut Context<Self>) {
        log::info!("table: {asked:?} on {}", picked.range);
        let header = picked.header;
        self.open_dialog(match asked {
            Asked::Sort => {
                let (first, ..) = picked.area.expect("a data area");
                Dialog::Sort {
                    tab: id,
                    keys: vec![SortKey {
                        column: first.to_string().into(),
                        rising: true,
                    }],
                    header,
                    picked,
                }
            }
            Asked::Filter => Dialog::Filter {
                tab: id,
                rules: Vec::new(),
                any: false,
                picked,
            },
            Asked::Dedupe => Dialog::Dedupe {
                tab: id,
                header,
                picked,
                state,
            },
        });
        cx.notify();
    }

    /// Sorts, filters or removes duplicates as asked.
    pub(crate) fn data_asked(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.dialog.take() else {
            return;
        };
        match dialog {
            Dialog::Sort {
                tab, keys, header, ..
            } => {
                let values = sort_values(header, &keys);
                let values: Vec<&str> = values.iter().map(String::as_str).collect();
                self.run_in(Some(tab), "data.sort", &values, cx);
            }
            Dialog::Filter {
                tab, rules, any, ..
            } => self.run_in(Some(tab), "data.filter", &[&query(&rules, any)], cx),
            Dialog::Dedupe {
                tab,
                header,
                picked,
                state: (at, sheet),
            } => {
                let (c1, r1, c2, r2) = picked.area.expect("a data area");
                let sheet = sheet.to_string();
                let corners = [c1, r1, c2, r2].map(|n| n.to_string());
                let header = header.to_string();
                let mut values = vec!["true", header.as_str(), "true"];
                values.extend(corners.iter().map(String::as_str));
                values.push(&sheet);
                self.run_on(tab, &at, "data.dedupe", &values, cx);
            }
            other => self.dialog = Some(other),
        }
    }

    /// Splits one column of text by the chosen separators.
    pub(crate) fn ask_columns(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.core.active else {
            return self.warn_key("command.blocked.no_document", cx);
        };
        self.columns_in(id, cx);
    }

    /// The columns dialog for tab `id`, once its input lands.
    pub(crate) fn columns_in(&mut self, id: u64, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |book, _, cx| book.columns_in(id, cx));
        }
        self.data_of(
            id,
            false,
            move |book, picked, _, window, cx| {
                let (c1, _, c2, _) = picked.area.expect("a data area");
                if c1 != c2 {
                    return book.warn_key("table.columns.same", cx);
                }
                let other = cx.new(|cx| TextInput::new(window, cx));
                book.open_dialog(Dialog::Columns {
                    tab: id,
                    separators: vec!["9".into()],
                    other,
                });
                cx.notify();
            },
            cx,
        );
    }

    /// Runs the split the columns dialog describes.
    pub(crate) fn columns_asked(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Columns {
            tab,
            separators,
            other,
        }) = self.dialog.take()
        else {
            return;
        };
        let mut codes: Vec<String> = separators.iter().map(ToString::to_string).collect();
        if let Some(c) = other.read(cx).text().chars().next() {
            codes.push(u32::from(c).to_string());
        }
        // Separators, quote, UTF-8, first row.
        let options = format!("{},34,76,1", codes.join("/"));
        self.run_in(Some(tab), "data.columns", &[&options], cx);
    }

    fn paste_only(&mut self, id: &'static str, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active else {
            return self.warn_key("command.blocked.no_document", cx);
        };
        self.paste_only_in(tab, id, cx);
    }

    /// Values or formulas alone, from cells this workbook copied.
    pub(crate) fn paste_only_in(&mut self, tab: u64, id: &'static str, cx: &mut Context<Self>) {
        if !self.settled(tab, cx) {
            return self.when_settled(tab, move |book, _, cx| book.paste_only_in(tab, id, cx));
        }
        let Some(view) = self.tab(tab).map(|tab| tab.view.clone()) else {
            return;
        };
        let html = match clipboard::read() {
            Ok(Some(Clip::Html { html, .. })) => Some(html),
            Ok(_) => None,
            Err(error) => {
                let text = Book::text(cx, "table.data.clipboard", &[("error", &format!("{error:#}"))]);
                return self.toast(text, Severity::Danger, cx);
            }
        };
        // The kernel keeps only its own cells' values and formulas.
        if !html.is_some_and(|html| view.read(cx).holds_copy(&html)) {
            return self.warn_key("table.data.paste_foreign", cx);
        }
        log::info!("table: {id}");
        view.update(cx, |view, _| view.paste_only(id));
    }

    /// The filter menu a column button asks for.
    pub(crate) fn ask_column(&mut self, id: u64, request: &Request, cx: &mut Context<Self>) {
        let Request::Filter { column, row } = *request else {
            return;
        };
        self.ask_in(
            id,
            move |worker, doc| {
                let json = worker.values(doc, &sheet::cell_query(sheet::FILTER, column, row))?;
                sheet::filter_values(&json)
            },
            move |book, read, window, cx| match read {
                Ok(filter) => {
                    let search = dialogs::search_field(window, cx);
                    let picked = filter
                        .entries
                        .iter()
                        .filter(|entry| entry.selected)
                        .map(|entry| SharedString::from(entry.text.clone()))
                        .collect();
                    book.open_dialog(Dialog::Column {
                        tab: id,
                        column,
                        row,
                        entries: filter.entries,
                        picked,
                        search,
                    });
                    cx.notify();
                }
                Err(error) => book.engine_failed(&error.to_string(), cx),
            },
            cx,
        );
    }

    /// Keeps the picked entries, or sorts by the column.
    pub(crate) fn column_asked(&mut self, sort: Option<bool>, cx: &mut Context<Self>) {
        let Some(Dialog::Column {
            tab,
            column,
            row,
            entries,
            picked,
            ..
        }) = self.dialog.take()
        else {
            return;
        };
        // The kernel filters the cursor's range: go to the button.
        self.run_in(Some(tab), "cell.goto", &[&sheet::address(column, row)], cx);
        match sort {
            Some(rising) => {
                let key = SortKey {
                    column: column.to_string().into(),
                    rising,
                };
                let values = sort_values(true, &[key]);
                let values: Vec<&str> = values.iter().map(String::as_str).collect();
                self.run_in(Some(tab), "data.sort", &values, cx);
            }
            None => {
                let query = column_query(column, &entries, &picked);
                self.run_in(Some(tab), "data.filter", &[&query], cx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_and_keys_become_kernel_values() {
        let rules = [
            FilterRule {
                column: "2".into(),
                test: Test::Above,
                value: "5".into(),
            },
            FilterRule {
                column: "0".into(),
                test: Test::Empty,
                value: "".into(),
            },
        ];
        let json: serde_json::Value = serde_json::from_str(&query(&rules, true)).expect("json");
        assert_eq!(json["conditions"][0]["op"], "greater");
        assert_eq!(json["conditions"][1]["or"], true);
        let keys = [SortKey {
            column: "1".into(),
            rising: false,
        }];
        assert_eq!(
            sort_values(true, &keys),
            ["true", "2", "false", "0", "true", "0", "true"]
        );
        let entries = [
            Entry {
                text: "a".into(),
                hidden: false,
                selected: true,
            },
            Entry {
                text: "b".into(),
                hidden: false,
                selected: true,
            },
        ];
        assert_eq!(
            column_query(3, &entries, &["a".into(), "b".into()]),
            r#"{"column":3}"#
        );
        assert!(column_query(3, &entries, &["a".into()]).contains(r#""values":["a"]"#));
    }
}
