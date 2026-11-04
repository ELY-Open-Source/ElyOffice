//! The Page menu: print area, titles, page, breaks (TABLE-14).

use ely_gpui_component::forms::TextInput;
use ely_lok::sheet::{self, PrintLayout, Scale, Span};
use ely_office::prelude::*;
use gpui::{
    App, AppContext as _, Context, Div, Entity, InteractiveElement as _, Menu, MenuItem, Window,
    actions,
};

use crate::book::Book;
use crate::dialog::Dialog;

mod dialogs;
pub use dialogs::page_dialog;

actions!(
    table,
    [
        PageSetup,
        RowBreak,
        ColumnBreak,
        RemoveRowBreak,
        RemoveColumnBreak
    ]
);

pub fn menu(cx: &App) -> Menu {
    let label = |key: &str| Book::text(cx, key, &[]);
    Menu::new(label("menu.page")).items([
        MenuItem::action(label("table.page.setup"), PageSetup),
        MenuItem::separator(),
        MenuItem::action(label("table.page.row_break"), RowBreak),
        MenuItem::action(label("table.page.column_break"), ColumnBreak),
        MenuItem::action(label("table.page.remove_row_break"), RemoveRowBreak),
        MenuItem::action(label("table.page.remove_column_break"), RemoveColumnBreak),
    ])
}

pub fn bind(root: Div, cx: &mut Context<Book>) -> Div {
    root.on_action(cx.listener(|book, _: &PageSetup, _, cx| book.ask_page(cx)))
        .on_action(cx.listener(|book, _: &RowBreak, _, cx| book.run_active("break.row", &[], cx)))
        .on_action(
            cx.listener(|book, _: &ColumnBreak, _, cx| book.run_active("break.column", &[], cx)),
        )
        .on_action(cx.listener(|book, _: &RemoveRowBreak, _, cx| book.remove_break(true, cx)))
        .on_action(
            cx.listener(|book, _: &RemoveColumnBreak, _, cx| book.remove_break(false, cx)),
        )
}

/// The page setup dialog's text fields.
pub struct PageFields {
    pub area: Entity<TextInput>,
    pub rows: Entity<TextInput>,
    pub columns: Entity<TextInput>,
}

/// A column's number from its letters, A is 1.
fn column_number(letters: &str) -> Option<u32> {
    (1..=3).contains(&letters.len()).then_some(())?;
    let number = letters.bytes().try_fold(0u32, |number, byte| {
        byte.is_ascii_uppercase()
            .then(|| number * 26 + u32::from(byte - b'A') + 1)
    })?;
    (number <= 16384).then_some(number)
}

fn row_number(digits: &str) -> Option<u32> {
    let number: u32 = digits.parse().ok()?;
    (1..=1_048_576).contains(&number).then_some(number)
}

/// One A1 cell such as `$B$2`.
fn cell_ok(cell: &str) -> bool {
    let cell = cell.strip_prefix('$').unwrap_or(cell);
    let split = cell.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(cell.len());
    let (letters, rest) = cell.split_at(split);
    let digits = rest.strip_prefix('$').unwrap_or(rest);
    column_number(&letters.to_ascii_uppercase()).is_some()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && row_number(digits).is_some()
}

/// Whole columns or rows, like `$A:$D`; one `$` each.
fn whole(from: &str, to: &str, number: fn(&str) -> Option<u32>) -> bool {
    let end = |end: &str| {
        let end = end.strip_prefix('$').unwrap_or(end);
        end.bytes().all(|byte| byte.is_ascii_alphanumeric()).then(|| number(&end.to_ascii_uppercase()))?
    };
    matches!((end(from), end(to)), (Some(first), Some(last)) if first <= last)
}

/// Areas as the kernel takes them, or None.
fn areas(text: &str) -> Option<String> {
    let tokens: Vec<&str> = text
        .split([',', ';'])
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect();
    let ok = |token: &&str| match token.split_once(':') {
        Some((from, to)) => {
            cell_ok(from) && cell_ok(to) || whole(from, to, column_number) || whole(from, to, row_number)
        }
        None => cell_ok(token),
    };
    tokens.iter().all(ok).then(|| tokens.join(","))
}

/// A span such as `1:2` or `$A`; empty is none.
fn span(text: &str, number: fn(&str) -> Option<u32>) -> Option<Option<Span>> {
    let text = text.trim();
    if text.is_empty() {
        return Some(None);
    }
    let end = |end: &str| number(&end.trim().trim_start_matches('$').to_ascii_uppercase());
    let (first, last) = match text.split_once(':') {
        Some((first, last)) => (end(first)?, end(last)?),
        None => (end(text)?, end(text)?),
    };
    (first <= last).then_some(Some((first, last)))
}

impl Book {
    pub(crate) fn ask_page(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.core.active() else {
            return self.warn_key("command.blocked.no_document", cx);
        };
        if !self.settled(id, cx) {
            return self.when_settled(id, |book, _, cx| book.ask_page(cx));
        }
        self.ask_in(
            id,
            |worker, doc| sheet::print_layout(&worker.values(doc, sheet::LAYOUT)?),
            move |book, read, window, cx| match read {
                Ok(layout) => book.page_dialog(id, layout, window, cx),
                Err(error) => book.engine_failed(&error.to_string(), cx),
            },
            cx,
        );
    }

    fn page_dialog(&mut self, id: u64, layout: PrintLayout, window: &mut Window, cx: &mut Context<Self>) {
        let mut field = |text: String, hint: &str, cx: &mut Context<Self>| {
            let hint = Book::text(cx, hint, &[]);
            cx.new(|cx| {
                let mut input = TextInput::new(window, cx).placeholder(hint);
                input.set_text(text, cx);
                input
            })
        };
        let fields = PageFields {
            area: field(layout.areas.join(","), "table.page.area_hint", cx),
            rows: field(sheet::rows_ref(layout.rows), "table.page.rows_hint", cx),
            columns: field(sheet::columns_ref(layout.columns), "table.page.columns_hint", cx),
        };
        self.open_dialog(Dialog::Page {
            tab: id,
            landscape: layout.landscape,
            scale: layout.scale,
            layout: Box::new(layout),
            fields: Box::new(fields),
        });
        cx.notify();
    }

    /// Applies the page setup dialog's changes.
    pub(crate) fn page_asked(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Page {
            tab,
            layout,
            landscape,
            scale,
            fields,
        }) = &self.dialog
        else {
            return;
        };
        let text = |field: &Entity<TextInput>| field.read(cx).text().to_string();
        let read = (
            areas(&text(&fields.area)),
            span(&text(&fields.rows), row_number),
            span(&text(&fields.columns), column_number),
        );
        let (Some(area), Some(rows), Some(columns)) = read else {
            return self.warn_key("table.page.bad_reference", cx);
        };
        let (tab, landscape, scale) = (*tab, *landscape, *scale);
        if scale == Scale::Fit(0, 0) {
            return self.warn_key("table.page.bad_fit", cx);
        }
        let printed = area != layout.areas.join(",") || rows != layout.rows || columns != layout.columns;
        let paged = landscape != layout.landscape || scale != layout.scale;
        self.dialog = None;
        cx.notify();
        if printed {
            let (rows, columns) = (sheet::rows_ref(rows), sheet::columns_ref(columns));
            self.run_in(Some(tab), "print.area", &[&area, &rows, &columns], cx);
        }
        if paged {
            self.run_in(Some(tab), "page.setup", &[&sheet::page_json(landscape, scale)], cx);
        }
    }

    /// Removes the cursor's break; none says so.
    fn remove_break(&mut self, row: bool, cx: &mut Context<Self>) {
        let Some(id) = self.core.active() else {
            return self.warn_key("command.blocked.no_document", cx);
        };
        if !self.settled(id, cx) {
            return self.when_settled(id, move |book, _, cx| book.remove_break(row, cx));
        }
        self.ask_in(
            id,
            |worker, doc| sheet::print_layout(&worker.values(doc, sheet::LAYOUT)?),
            move |book, read, _, cx| match read {
                Ok(layout) if row && layout.row_break => {
                    book.run_in(Some(id), "break.row_remove", &[], cx)
                }
                Ok(layout) if !row && layout.column_break => {
                    book.run_in(Some(id), "break.column_remove", &[], cx)
                }
                Ok(_) => book.warn_key("table.page.no_break", cx),
                Err(error) => book.engine_failed(&error.to_string(), cx),
            },
            cx,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_parse_like_excel() {
        assert_eq!(areas(" A1:B2; $d$4 ").as_deref(), Some("A1:B2,$d$4"));
        assert_eq!(areas("").as_deref(), Some(""));
        assert_eq!(areas("A1:B"), None);
        assert_eq!(areas("XFE1"), None);
        assert_eq!(areas("S!A1"), None);
        assert_eq!(areas("$A:$D, 1:3").as_deref(), Some("$A:$D,1:3"));
        assert_eq!(areas("A:3"), None);
        for bad in ["$$A:$$D", "$$1:$$3", "$A:$$D", "A:D,$$E:$$F", "+1:2", "D:A"] {
            assert_eq!(areas(bad), None, "{bad}");
        }
        assert_eq!(span("$1:$3", row_number), Some(Some((1, 3))));
        assert_eq!(span("2", row_number), Some(Some((2, 2))));
        assert_eq!(span("3:1", row_number), None);
        assert_eq!(span("a:ab", column_number), Some(Some((1, 28))));
        assert_eq!(span(" ", column_number), Some(None));
        assert_eq!(span("A1", column_number), None);
    }
}
