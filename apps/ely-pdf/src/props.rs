//! The document properties dialog (PDF-07).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::{Input, TextInput},
    overlays::Dialog as Card,
};
use ely_office::{Closing as _, Tabs as _};
use ely_pdf_engine::Properties;
use gpui::{AnyElement, App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, SharedString, Styled as _, Window, div};

use crate::viewer::Viewer;

/// The keys a person may change, with their label keys.
const EDITABLE: [(&str, &str); 4] =
    [("Title", "pdf.props.title"), ("Author", "pdf.props.author"), ("Subject", "pdf.props.subject"), ("Keywords", "pdf.props.keywords")];

/// The properties as read, and their fields.
pub struct Props {
    pub tab: u64,
    pub read: Properties,
    pub fields: [Entity<TextInput>; 4],
}

/// A PDF date (D:YYYYMMDDHHmmSS±HH'mm') as `YYYY-MM-DD HH:MM ±HH:MM`.
pub fn date(raw: &str) -> String {
    let body = raw.trim_start_matches("D:");
    let digits: String = body.chars().take_while(char::is_ascii_digit).collect();
    if digits.len() < 12 {
        return raw.to_string();
    }
    let shown = format!("{}-{}-{} {}:{}", &digits[0..4], &digits[4..6], &digits[6..8], &digits[8..10], &digits[10..12]);
    let zone = &body[digits.len()..];
    let hours: String = zone.chars().skip(1).filter(char::is_ascii_digit).collect();
    match zone.chars().next() {
        Some('Z') => format!("{shown} UTC"),
        Some(sign @ ('+' | '-')) if hours.len() == 4 => format!("{shown} {sign}{}:{}", &hours[0..2], &hours[2..4]),
        _ => shown,
    }
}

fn now(read: &Properties, key: &str) -> Option<String> {
    match key {
        "Title" => read.title.clone(),
        "Author" => read.author.clone(),
        "Subject" => read.subject.clone(),
        _ => read.keywords.clone(),
    }
}

impl Viewer {
    pub fn ask_properties(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active() else {
            return self.no_document(cx);
        };
        self.read_in(
            tab,
            |worker, doc| worker.get()?.properties(*doc),
            move |viewer, read, window, cx| match read {
                Ok(read) => {
                    let fields = EDITABLE.map(|(key, _)| {
                        let value = now(&read, key).unwrap_or_default();
                        cx.new(|cx| {
                            let mut field = TextInput::new(window, cx);
                            field.set_text(value, cx);
                            field
                        })
                    });
                    viewer.props = Some(Props { tab, read, fields });
                    cx.notify();
                }
                Err(error) => viewer.failed(&error, cx),
            },
            cx,
        );
    }

    /// Writes the fields that changed, one step.
    pub(crate) fn set_properties(&mut self, cx: &mut Context<Self>) {
        let Some(Props { tab, read, fields }) = self.props.take() else {
            return;
        };
        cx.notify();
        let changed: Vec<(&'static str, String)> = (EDITABLE.iter().zip(&fields))
            .map(|((key, _), field)| (*key, field.read(cx).text().to_string()))
            .filter(|(key, typed)| now(&read, key).unwrap_or_default() != *typed)
            .collect();
        let Some(view) = self.tab(tab).map(|tab| tab.view.clone()).filter(|_| !changed.is_empty()) else {
            return log::info!("pdf: the properties are as they were");
        };
        view.update(cx, |view, cx| {
            view.edit(
                "pdf.doing.properties",
                move |worker, doc| {
                    let pairs: Vec<(&str, &str)> = changed.iter().map(|(key, value)| (*key, value.as_str())).collect();
                    worker.set_properties(doc, &pairs)
                },
                cx,
            )
        });
    }

    /// The properties dialog, while open.
    pub fn props_element(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let props = self.props.as_ref()?;
        let this = cx.entity();
        let label = |key: &str| Self::text(cx, key, &[]);
        let mut rows = DescriptionList::new();
        for ((_, key), field) in EDITABLE.iter().zip(&props.fields) {
            rows = rows.item(label(key), div().w_full().child(Input::new(field)));
        }
        let shown = |value: &Option<String>, dated: bool| -> SharedString {
            match value {
                Some(value) if dated => date(value).into(),
                Some(value) => value.clone().into(),
                None => label("pdf.props.none"),
            }
        };
        let read = &props.read;
        rows = rows
            .item(label("pdf.props.creator"), shown(&read.creator, false))
            .item(label("pdf.props.producer"), shown(&read.producer, false))
            .item(label("pdf.props.created"), shown(&read.created, true))
            .item(label("pdf.props.modified"), shown(&read.modified, true));
        let closed = this.clone();
        let dismiss = move |_: &mut Window, cx: &mut App| {
            closed.update(cx, |viewer, cx| {
                viewer.props = None;
                cx.notify();
            })
        };
        let go = label("pdf.text.apply");
        let card = Card::new("pdf-props", label("pdf.props.heading"), dismiss).child(rows).action(ely_office::cancel(cx)).action(move |_| {
            Button::new("pdf-props-go", go)
                .variant(ButtonVariant::Primary)
                .on_click(move |_, _, cx| this.update(cx, |viewer, cx| viewer.set_properties(cx)))
        });
        Some(card.into_any_element())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn pdf_dates_read_as_day_and_minute() {
        assert_eq!(super::date("D:20261006153000+08'00'"), "2026-10-06 15:30 +08:00");
        assert_eq!(super::date("D:20261006225959Z"), "2026-10-06 22:59 UTC");
        assert_eq!(super::date("D:202610061530"), "2026-10-06 15:30", "no zone given");
        assert_eq!(super::date("yesterday"), "yesterday", "as written when not a date");
    }
}
