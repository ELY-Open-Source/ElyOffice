//! Searching the mail held here (C12.16).

use std::time::Duration;

use ely_gpui_component::{
    buttons::{ButtonVariant, IconButton},
    forms::{Choice, DateRangePicker, InputEvent, SearchInput, Select, TextInput},
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, TextSize},
};
use ely_mail_engine::{Field, Filter, MessageId, Search};
use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Styled as _, Subscription, Window, div,
};
use jiff::{civil::Date, tz::TimeZone};

use crate::mail::{Mail, text};

/// Typing waits this long before a search.
const PAUSE: Duration = Duration::from_millis(200);

/// Each place to look: its key, its words.
const FIELDS: [(&str, &str, Field); 6] = [
    ("all", "mail.search.all", Field::All),
    ("from", "mail.search.from", Field::From),
    ("to", "mail.search.to", Field::To),
    ("subject", "mail.search.subject", Field::Subject),
    ("body", "mail.search.body", Field::Body),
    ("files", "mail.search.files", Field::Files),
];

pub struct Seeking {
    pub input: Entity<TextInput>,
    pub field: Field,
    /// The first and last day picked.
    pub days: Option<(Date, Date)>,
    /// The last search's bodies not held here; None while listing.
    pub unsearched: Option<u64>,
    /// Numbers edits; only the last one searches.
    edits: u64,
    _typed: Subscription,
}

impl Seeking {
    pub fn new(window: &mut Window, cx: &mut Context<Mail>) -> Self {
        let (input, typed) = Self::input(String::new(), window, cx);
        Seeking {
            input,
            field: Field::All,
            days: None,
            unsearched: None,
            edits: 0,
            _typed: typed,
        }
    }

    /// A new window needs a new box: focus is per-window.
    pub fn rehome(&mut self, window: &mut Window, cx: &mut Context<Mail>) {
        let words = self.input.read(cx).text().to_owned();
        (self.input, self._typed) = Self::input(words, window, cx);
    }

    fn input(words: String, window: &mut Window, cx: &mut Context<Mail>) -> (Entity<TextInput>, Subscription) {
        let hint = text(cx, "mail.search", &[]);
        let input = cx.new(|cx| {
            let mut input = TextInput::new(window, cx).placeholder(hint.clone()).label(hint);
            input.set_text(words, cx);
            input
        });
        let typed = cx.subscribe(&input, |mail: &mut Mail, _, event: &InputEvent, cx| {
            if *event == InputEvent::Changed {
                mail.typed(cx);
            }
        });
        (input, typed)
    }

    /// What is asked; None while nothing is.
    pub fn search(&self, filter: Filter, held: &[MessageId], cx: &App) -> Result<Option<Search>, String> {
        let words = self.input.read(cx).text().trim().to_owned();
        if words.is_empty() && self.days.is_none() {
            return Ok(None);
        }
        let dates = match self.days {
            Some(days) => Some(seconds(days).map_err(|error| error.to_string())?),
            None => None,
        };
        let field = self.field;
        Ok(Some(Search {
            words,
            field,
            dates,
            filter,
            held: held.to_vec(),
        }))
    }
}

/// First day's midnight to the day after the last, here.
fn seconds((first, last): (Date, Date)) -> Result<(i64, i64), jiff::Error> {
    let start = |day: Date| -> Result<i64, jiff::Error> {
        Ok(day.to_zoned(TimeZone::system())?.timestamp().as_second())
    };
    Ok((start(first)?, start(last.tomorrow()?)?))
}

impl Mail {
    /// Searches once typing stops.
    fn typed(&mut self, cx: &mut Context<Self>) {
        self.seeking.edits += 1;
        let edits = self.seeking.edits;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(PAUSE).await;
            this.update(cx, |mail, cx| {
                if mail.seeking.edits == edits {
                    mail.read_mail(cx);
                }
            })
            .ok();
        })
        .detach();
    }

    pub fn set_field(&mut self, field: Field, cx: &mut Context<Self>) {
        log::info!("mail: search in {field:?}");
        self.seeking.field = field;
        self.read_mail(cx);
    }

    pub fn set_days(&mut self, days: Option<(Date, Date)>, cx: &mut Context<Self>) {
        log::info!(
            "mail: search dates {}",
            if days.is_some() { "set" } else { "cleared" }
        );
        self.seeking.days = days;
        self.read_mail(cx);
    }

    /// The box, the field, the days, and the unsearched note.
    pub fn search_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let choices = FIELDS.map(|(key, words, _)| Choice::new(key, text(cx, words, &[])));
        let chosen = FIELDS
            .iter()
            .find(|(_, _, field)| *field == self.seeking.field);
        let this = cx.weak_entity();
        let field = Select::new("mail-search-in", choices)
            .label(text(cx, "mail.search.in", &[]))
            .selected(chosen.map_or("all", |(key, _, _)| key).to_owned())
            .size(ControlSize::Sm)
            .on_change(move |value, _, cx| {
                let Some((_, _, field)) = FIELDS.iter().find(|(key, _, _)| *key == value.as_ref())
                else {
                    return log::error!("mail: no search field {value}");
                };
                this.update(cx, |mail, cx| mail.set_field(*field, cx)).ok();
            });
        let this = cx.weak_entity();
        let days = DateRangePicker::new("mail-search-days", self.seeking.days)
            .size(ControlSize::Sm)
            .on_change(move |days, _, cx| {
                this.update(cx, |mail, cx| mail.set_days(Some(days), cx))
                    .ok();
            });
        let this = cx.weak_entity();
        let any_time = self.seeking.days.map(|_| {
            IconButton::new("mail-search-any-time", IconName::X)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(text(cx, "mail.search.any_time", &[]))
                .on_click(move |_, _, cx| {
                    this.update(cx, |mail, cx| mail.set_days(None, cx)).ok();
                })
        });
        let note = self.seeking.unsearched.map(|count| {
            let words = match count {
                0 => text(cx, "mail.search.local", &[]),
                _ => text(cx, "mail.search.headers", &[("count", &count.to_string())]),
            };
            let theme = cx.theme();
            div()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(theme.colors.fg_muted)
                .child(words)
        });
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .child(SearchInput::new("mail-search", &self.seeking.input).size(ControlSize::Sm))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(div().flex_1().min_w_24().child(field))
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .child(
                                div()
                                    .debug_selector(|| "mail-search-days".into())
                                    .flex_1()
                                    .min_w_0()
                                    .child(days),
                            )
                            .children(any_time.map(|button| {
                                div()
                                    .debug_selector(|| "mail-search-any-time".into())
                                    .flex_shrink_0()
                                    .child(button)
                            })),
                    ),
            )
            .children(note)
            .into_any_element()
    }

    /// Back to the open list.
    pub fn stop_seeking(&mut self, cx: &mut Context<Self>) {
        self.seeking.field = Field::All;
        self.seeking.days = None;
        let input = self.seeking.input.clone();
        input.update(cx, |input, cx| input.set_text("", cx));
    }
}
