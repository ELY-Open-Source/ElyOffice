//! Rules: validation, choices, input help, conditional formats (TABLE-09).

use ely_gpui_component::{forms::TextInput, primitives::Severity};
use ely_lok::{
    LokView, Request,
    sheet::{self, Validation},
};
use ely_office::prelude::*;
use gpui::{AppContext as _, Context, Div, Entity, InteractiveElement as _, Window, actions};

use crate::book::{Book, Spot};
use crate::dialog::Dialog;

mod aids;
mod dialogs;
pub use aids::Choices;
pub use dialogs::{Shown, conditional_dialog, formats_dialog, validation_dialog};

actions!(table, [Validate, Unvalidate, Conditional, Conditionals]);

pub fn bind(root: Div, cx: &mut Context<Book>) -> Div {
    root.on_action(cx.listener(|book, _: &Validate, _, cx| book.ask_rule(cx)))
        .on_action(cx.listener(|book, _: &Unvalidate, _, cx| book.clear_rule(cx)))
        .on_action(
            cx.listener(|book, _: &Conditional, window, cx| book.ask_conditional(window, cx)),
        )
        .on_action(cx.listener(|book, _: &Conditionals, _, cx| book.ask_formats(cx)))
}

/// The text fields of the validation dialog.
pub struct Fields {
    pub value1: Entity<TextInput>,
    pub value2: Entity<TextInput>,
    pub list: Entity<TextInput>,
    pub input_title: Entity<TextInput>,
    pub input_message: Entity<TextInput>,
    pub error_title: Entity<TextInput>,
    pub error_message: Entity<TextInput>,
}

/// List entries, one per line; a range starts "=".
fn list_text(value1: &str) -> String {
    if !value1.starts_with('"') {
        return format!("={value1}");
    }
    let mut entries = Vec::new();
    let mut chars = value1.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut entry = String::new();
        while let Some(c) = chars.next() {
            match (c, chars.peek()) {
                ('"', Some('"')) => {
                    entry.push('"');
                    chars.next();
                }
                ('"', _) => break,
                (c, _) => entry.push(c),
            }
        }
        entries.push(entry);
    }
    entries.join("\n")
}

/// No rule on the cell: the dialog's starting point.
fn unrestricted() -> Validation {
    Validation {
        kind: "any".into(),
        operator: "between".into(),
        dropdown: true,
        ignore_blank: true,
        show_error: true,
        error_style: "stop".into(),
        ..Validation::default()
    }
}

impl Book {
    /// Opens the validation of the current cell.
    pub(crate) fn ask_rule(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.core.active() else {
            let text = Book::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        self.rule_in(id, cx);
    }

    /// The validation dialog for tab `id`, once its input lands.
    pub(crate) fn rule_in(&mut self, id: u64, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |book, _, cx| book.rule_in(id, cx));
        }
        self.ask_in(
            id,
            |worker, doc| sheet::validation(&worker.values(doc, sheet::VALIDATION)?),
            move |book, read, window, cx| match read {
                Ok(rule) => book.rule_dialog(id, rule, window, cx),
                Err(error) => book.engine_failed(&error.to_string(), cx),
            },
            cx,
        );
    }

    fn rule_dialog(
        &mut self,
        id: u64,
        rule: Validation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut field = |text: &str, rows: Option<(usize, usize)>, cx: &mut Context<Self>| {
            let text = text.to_string();
            cx.new(|cx| {
                let input = TextInput::new(window, cx);
                let mut input = match rows {
                    Some((min, max)) => input.multi_line(min, max),
                    None => input,
                };
                input.set_text(text, cx);
                input
            })
        };
        let list = if rule.kind == "list" {
            list_text(&rule.value1)
        } else {
            String::new()
        };
        let fields = Fields {
            value1: field(&rule.value1, None, cx),
            value2: field(&rule.value2, None, cx),
            list: field(&list, Some((3, 8)), cx),
            input_title: field(&rule.input_title, None, cx),
            input_message: field(&rule.input_message, Some((2, 4)), cx),
            error_title: field(&rule.error_title, None, cx),
            error_message: field(&rule.error_message, Some((2, 4)), cx),
        };
        // A cell without a rule starts from the defaults.
        let rule = if rule.operator.is_empty() {
            unrestricted()
        } else {
            rule
        };
        self.open_dialog(Dialog::Validation {
            tab: id,
            rule: Box::new(rule),
            fields: Box::new(fields),
        });
        cx.notify();
    }

    /// Removes the selection's validation.
    pub(crate) fn clear_rule(&mut self, cx: &mut Context<Self>) {
        let rule = unrestricted().to_json();
        self.run_active("rules.validation", &[&rule], cx);
    }

    /// Sets the validation the dialog describes.
    pub(crate) fn rule_asked(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Validation { tab, rule, fields }) = self.dialog.take() else {
            return;
        };
        let text = |field: &Entity<TextInput>| field.read(cx).text().to_string();
        let mut rule = Validation {
            value1: text(&fields.value1).trim().to_string(),
            value2: text(&fields.value2).trim().to_string(),
            input_title: text(&fields.input_title),
            input_message: text(&fields.input_message),
            error_title: text(&fields.error_title),
            error_message: text(&fields.error_message),
            ..*rule
        };
        if rule.kind == "list" {
            let list = text(&fields.list);
            match list.trim().strip_prefix('=') {
                Some(source) => rule.value1 = source.trim().to_string(),
                None => {
                    rule.value1.clear();
                    rule.list = list
                        .lines()
                        .map(str::trim)
                        .filter(|entry| !entry.is_empty())
                        .map(str::to_string)
                        .collect();
                }
            }
        }
        self.run_in(Some(tab), "rules.validation", &[&rule.to_json()], cx);
    }

    /// Takes the kernel's new request on tab `id`.
    pub(crate) fn requested(&mut self, id: u64, view: &Entity<LokView>, cx: &mut Context<Self>) {
        let cell = view.read(cx).cell();
        let (count, request) = (cell.requests, cell.request.clone());
        let Some(tab) = self.tab_mut(id) else {
            return;
        };
        if tab.extra.requests == count {
            return;
        }
        tab.extra.requests = count;
        match request {
            Some(request @ Request::Filter { .. }) => self.ask_column(id, &request, cx),
            Some(Request::Choices { column, row }) => self.offer_choices(id, (column, row), cx),
            Some(Request::Invalid {
                title,
                message,
                style,
                accepted,
            }) => {
                let fate = if accepted { "noted" } else { "refused" };
                log::info!("table: a {style} entry was {fate}");
                let severity = match style.as_str() {
                    "info" => Severity::Info,
                    "warning" => Severity::Warning,
                    _ => Severity::Danger,
                };
                // The document's own words: shown, never logged.
                if let Some(root) = self.core.root.upgrade() {
                    let text: gpui::SharedString = format!("{title}: {message}").into();
                    root.update(cx, |root, cx| root.toast(text, severity, cx));
                }
            }
            None => {}
        }
    }

    /// Rereads the current cell's rule after the cursor moves.
    pub(crate) fn follow_rule(&mut self, id: u64, view: &Entity<LokView>, cx: &mut Context<Self>) {
        let read = view.read(cx);
        let spot: Spot = (
            read.cell().sheets,
            read.cell().address.clone(),
            read.edited(),
        );
        let Some(tab) = self.tab(id) else {
            return;
        };
        let asked = tab.extra.ruled.as_ref() == Some(&spot);
        if spot.1.is_empty() || tab.extra.ruling || asked {
            return;
        }
        let doc = tab.doc.clone();
        // Asked now: a failure waits for the next change.
        let extra = &mut self.tab_mut(id).expect("the tab is open").extra;
        (extra.ruling, extra.ruled) = (true, Some(spot.clone()));
        ely_lok::read_off(
            &self.core.worker,
            &doc,
            |worker, doc| sheet::validation(&worker.values(doc, sheet::VALIDATION)?),
            move |book: &mut Self, read, cx| {
                let Some(tab) = book.tab_mut(id) else {
                    return;
                };
                tab.extra.ruling = false;
                let view = tab.view.clone();
                match read {
                    Ok(rule) => tab.extra.rule = Some((spot, rule)),
                    Err(error) => book.engine_failed(&error.to_string(), cx),
                }
                cx.notify();
                // The cursor may have moved meanwhile.
                book.follow_rule(id, &view, cx);
            },
            cx,
        );
    }

    pub(crate) fn ask_conditional(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.core.active() else {
            let text = Book::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        let value1 = cx.new(|cx| TextInput::new(window, cx));
        let value2 = cx.new(|cx| TextInput::new(window, cx));
        self.open_dialog(Dialog::Conditional {
            tab: id,
            kind: "cell",
            operator: "greater",
            style: "Good",
            preset: 0,
            value1,
            value2,
        });
        cx.notify();
    }

    /// Adds the conditional format the dialog describes.
    pub(crate) fn conditional_asked(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Conditional {
            tab,
            kind,
            operator,
            style,
            preset,
            value1,
            value2,
        }) = self.dialog.take()
        else {
            return;
        };
        let rule = match kind {
            "scale" => serde_json::json!({ "kind": "scale", "colors": dialogs::SCALES[preset].1 }),
            "databar" => serde_json::json!({ "kind": "databar", "color": dialogs::BARS[preset].1 }),
            _ => serde_json::json!({
                "kind": "cell",
                "operator": operator,
                "style": style,
                "value1": value1.read(cx).text().trim(),
                "value2": value2.read(cx).text().trim(),
            }),
        };
        self.run_in(Some(tab), "rules.conditional", &[&rule.to_string()], cx);
    }

    /// Lists the shown sheet's conditional formats.
    pub(crate) fn ask_formats(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.core.active() else {
            let text = Book::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        self.formats_in(id, cx);
    }

    /// The format list of tab `id`, once its input lands.
    pub(crate) fn formats_in(&mut self, id: u64, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |book, _, cx| book.formats_in(id, cx));
        }
        self.ask_in(
            id,
            |worker, doc| sheet::formats(&worker.values(doc, sheet::FORMATS)?),
            move |book, read, _, cx| match read {
                Ok(formats) if formats.is_empty() => {
                    let text = Book::text(cx, "table.rules.no_formats", &[]);
                    book.toast(text, Severity::Info, cx);
                }
                Ok(formats) => {
                    book.open_dialog(Dialog::Formats {
                        tab: id,
                        formats,
                        chosen: 0,
                    });
                    cx.notify();
                }
                Err(error) => book.engine_failed(&error.to_string(), cx),
            },
            cx,
        );
    }

    /// Removes the chosen conditional format.
    pub(crate) fn format_removed(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Formats {
            tab,
            formats,
            chosen,
        }) = self.dialog.take()
        else {
            return;
        };
        let remove = serde_json::json!({ "remove": formats[chosen].key }).to_string();
        self.run_in(Some(tab), "rules.conditional", &[&remove], cx);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn list_sources_read_as_lines() {
        assert_eq!(
            super::list_text(r#""Red","Green ""G""""#),
            "Red\nGreen \"G\""
        );
        assert_eq!(super::list_text("$A$1:$A$3"), "=$A$1:$A$3");
    }
}
