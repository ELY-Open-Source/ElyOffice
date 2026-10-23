//! The format panel: numbers, alignment, borders, fill (TABLE-07).

use std::sync::{Arc, Mutex};

use ely_gpui_component::{
    buttons::{Button, ToggleButton, ToggleItem},
    data_display::{PropertyGrid, PropertyGroup},
    forms::{Choice, ColorPalette, ColorSwatch, Input, InputEvent, Select, TextInput},
    layout::ScrollArea,
    overlays::Popover,
    primitives::{IconName, Severity},
    theme::ControlSize,
};
use ely_lok::{Doc, LokView, Reported, Worker, sheet, show::Paint};
use ely_office::prelude::*;
use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, Hsla, IntoElement, Menu, MenuItem,
    ParentElement as _, Render, SharedString, Styled as _, Subscription, WeakEntity, Window,
    div, rgb,
};

use crate::book::{Book, Spot};

mod colors;
pub(crate) use colors::paint;
use colors::{FILL, Pick, TEXT};

pub fn menu(cx: &App) -> Menu {
    let label = |key: &str| Book::text(cx, key, &[]);
    Menu::new(label("menu.format")).items([
        MenuItem::action(label("table.format.panel"), ely_ui::commands::ToggleRightPanel),
        MenuItem::separator(),
        MenuItem::action(label("table.rules.conditional"), crate::rules::Conditional),
        MenuItem::action(
            label("table.rules.conditionals"),
            crate::rules::Conditionals,
        ),
        MenuItem::separator(),
        MenuItem::action(label("table.protect.lock"), crate::protect::LockCells),
        MenuItem::action(label("table.protect.unlock"), crate::protect::UnlockCells),
    ])
}

/// Number kinds the panel sets, their commands and values.
const KINDS: [(&str, &str, &str, &[&str]); 6] = [
    ("general", "number.general", "table.format.general", &[]),
    ("number", "number.decimal", "table.format.decimal", &[]),
    ("currency", "number.currency", "table.format.currency", &[]),
    ("percent", "number.percent", "table.format.percent", &[]),
    ("date", "number.date", "table.format.date", &[]),
    // Digits stay as typed: IDs, codes, leading zeros (#15).
    ("text", "number.code", "table.format.text", &["@"]),
];

/// The workbook the panel formats.
struct Shown {
    view: Entity<LokView>,
    doc: Doc,
    _observe: Subscription,
}

pub struct Panel {
    book: WeakEntity<Book>,
    worker: Arc<Mutex<Worker>>,
    shown: Option<Shown>,
    code: Entity<TextInput>,
    /// The code field has the user's typing.
    editing: bool,
    /// The cell of the last read.
    read: Option<Spot>,
    /// A read is on its way.
    reading: bool,
    /// The cell's format code and kind.
    pub(crate) number: Option<(String, String)>,
    _code: Subscription,
}

impl Panel {
    pub fn new(
        book: WeakEntity<Book>,
        worker: Arc<Mutex<Worker>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = Book::text(cx, "table.format.code", &[]);
        let code = cx.new(|cx| TextInput::new(window, cx).label(name));
        let subscription = cx.subscribe(&code, |panel, code, event, cx| match event {
            InputEvent::Focus => panel.editing = true,
            InputEvent::Blur => {
                panel.editing = false;
                panel.read = None;
                panel.reread(cx);
            }
            InputEvent::Submit => {
                let text = code.read(cx).text().trim().to_string();
                panel.apply_code(&text, cx);
            }
            InputEvent::Changed => {}
        });
        Panel {
            book,
            worker,
            shown: None,
            code,
            editing: false,
            read: None,
            reading: false,
            number: None,
            _code: subscription,
        }
    }

    /// Follows the active workbook.
    pub fn show(&mut self, tab: Option<(&Entity<LokView>, &Doc)>, cx: &mut Context<Self>) {
        let id = tab.map(|(view, _)| view.entity_id());
        if self.shown.as_ref().map(|shown| shown.view.entity_id()) == id {
            return;
        }
        self.shown = tab.map(|(view, doc)| Shown {
            view: view.clone(),
            doc: doc.clone(),
            _observe: cx.subscribe(view, |panel, _, _: &Reported, cx| {
                panel.reread(cx);
                cx.notify();
            }),
        });
        self.read = None;
        self.reread(cx);
    }

    /// Rereads the number format after a move or edit.
    fn reread(&mut self, cx: &mut Context<Self>) {
        let Some(shown) = &self.shown else {
            return;
        };
        let view = shown.view.read(cx);
        let at: Spot = (
            view.cell().sheets,
            view.cell().address.clone(),
            view.edited(),
        );
        if self.reading || !view.settled() || self.read.as_ref() == Some(&at) {
            return;
        }
        (self.read, self.reading) = (Some(at), true);
        let reader = shown.view.entity_id();
        ely_lok::read_off(
            &self.worker,
            &shown.doc,
            |worker, doc| sheet::number_format(&worker.values(doc, sheet::NUMBER)?),
            move |panel: &mut Self, read, cx| {
                panel.reading = false;
                if panel.shown.as_ref().map(|shown| shown.view.entity_id()) != Some(reader) {
                    return panel.reread(cx);
                }
                match read {
                    Ok(number) => {
                        if !panel.editing {
                            let code = number.0.clone();
                            panel.code.update(cx, |input, cx| input.set_text(code, cx));
                        }
                        panel.number = Some(number);
                    }
                    Err(error) => panel.toast(&format!("{error}"), cx),
                }
                cx.notify();
                // The cell may have moved meanwhile.
                panel.reread(cx);
            },
            cx,
        );
    }

    fn toast(&self, error: &str, cx: &mut Context<Self>) {
        if let Some(book) = self.book.upgrade() {
            book.update(cx, |book, cx| book.engine_failed(error, cx));
        }
    }

    /// Sets a typed format code once the kernel takes it.
    pub(crate) fn apply_code(&mut self, code: &str, cx: &mut Context<Self>) {
        let Some(shown) = &self.shown else {
            return;
        };
        // The workbook that asked, not the one active later.
        let Some(tab) = self
            .book
            .upgrade()
            .and_then(|book| book.read(cx).core.active)
        else {
            return;
        };
        let code = code.to_string();
        let asked = code.clone();
        ely_lok::read_off(
            &self.worker,
            &shown.doc,
            move |worker, doc| {
                sheet::number_valid(&worker.values(doc, &sheet::number_check_query(&asked))?)
            },
            move |panel: &mut Self, check, cx| match check {
                Ok(true) => {
                    if let Some(book) = panel.book.upgrade() {
                        book.update(cx, |book, cx| {
                            book.run_in(Some(tab), "number.code", &[&code], cx)
                        });
                    }
                }
                Ok(false) => {
                    if let Some(book) = panel.book.upgrade() {
                        book.update(cx, |book, cx| {
                            let text = Book::text(cx, "table.format.invalid", &[("code", &code)]);
                            book.toast(text, Severity::Warning, cx);
                        });
                    }
                }
                Err(error) => panel.toast(&error.to_string(), cx),
            },
            cx,
        );
    }

    fn state(&self, uno: &str, cx: &App) -> Option<String> {
        let view = self.shown.as_ref()?.view.read(cx);
        view.state(uno).map(str::to_string)
    }

    /// A kernel-lit toggle for `command`.
    fn toggle(
        &self,
        command: &'static str,
        icon: Option<IconName>,
        key: &str,
        cx: &Context<Self>,
    ) -> AnyElement {
        let uno = ely_lok::commands::find(command)
            .expect("a table command")
            .uno;
        let on = self.state(uno, cx).as_deref() == Some("true");
        let mut item = ToggleItem::new(command).tooltip(Book::text(cx, key, &[]));
        item = match icon {
            Some(icon) => item.icon(icon),
            None => item.label(Book::text(cx, key, &[])),
        };
        let book = self.book.clone();
        ToggleButton::new(SharedString::from(command), item, on)
            .size(ControlSize::Sm)
            .on_toggle(move |_, _, cx| {
                let wrap = (!on).to_string();
                let values: &[&str] = if command == "cells.wrap" {
                    &[&wrap]
                } else {
                    &[]
                };
                run(&book, command, values, cx);
            })
            .into_any_element()
    }

    fn button(&self, command: &'static str, key: &str, cx: &Context<Self>) -> AnyElement {
        let book = self.book.clone();
        Button::new(SharedString::from(command), Book::text(cx, key, &[]))
            .size(ControlSize::Sm)
            .on_click(move |_, _, cx| run(&book, command, &[], cx))
            .into_any_element()
    }

    fn number_group(&self, cx: &Context<Self>) -> PropertyGroup {
        let label = |key: &str| Book::text(cx, key, &[]);
        let kind = self
            .number
            .as_ref()
            .map_or("general", |(_, kind)| kind.as_str());
        let listed = KINDS.iter().any(|(value, ..)| *value == kind);
        let mut choices: Vec<Choice> = KINDS
            .iter()
            .map(|(value, _, key, _)| Choice::new(*value, label(key)))
            .collect();
        if !listed {
            choices.push(Choice::new(kind.to_string(), label("table.format.custom")));
        }
        let book = self.book.clone();
        let kinds = Select::new("table-format-kind", choices)
            .label(label("table.format.kind"))
            .selected(kind.to_string())
            .size(ControlSize::Sm)
            .on_change(move |value, _, cx| {
                if let Some((_, command, _, values)) =
                    KINDS.iter().find(|(kind, ..)| *kind == value.as_ref())
                {
                    run(&book, command, values, cx);
                }
            });
        PropertyGroup::new(label("table.format.number"))
            .row(label("table.format.kind"), kinds)
            .row(
                label("table.format.code"),
                Input::new(&self.code).size(ControlSize::Sm),
            )
    }

    fn align_group(&self, cx: &Context<Self>) -> PropertyGroup {
        let label = |key: &str| Book::text(cx, key, &[]);
        let across = row([
            self.toggle(
                "align.left",
                Some(IconName::AlignLeft),
                "table.format.left",
                cx,
            ),
            self.toggle(
                "align.center",
                Some(IconName::AlignCenter),
                "table.format.center",
                cx,
            ),
            self.toggle(
                "align.right",
                Some(IconName::AlignRight),
                "table.format.right",
                cx,
            ),
            self.toggle(
                "align.justify",
                Some(IconName::AlignJustify),
                "table.format.justify",
                cx,
            ),
        ]);
        let down = row([
            self.toggle(
                "align.top",
                Some(IconName::AlignStartHorizontal),
                "table.format.top",
                cx,
            ),
            self.toggle(
                "align.middle",
                Some(IconName::AlignCenterHorizontal),
                "table.format.middle",
                cx,
            ),
            self.toggle(
                "align.bottom",
                Some(IconName::AlignEndHorizontal),
                "table.format.bottom",
                cx,
            ),
        ]);
        PropertyGroup::new(label("table.format.alignment"))
            .row(label("table.format.across"), across)
            .row(label("table.format.down"), down)
            .row(
                label("table.format.text"),
                self.toggle("cells.wrap", None, "table.format.wrap", cx),
            )
    }

    fn cell_group(&self, cx: &Context<Self>) -> PropertyGroup {
        let label = |key: &str| Book::text(cx, key, &[]);
        let borders = row([
            self.button("cells.borders_all", "table.format.borders_all", cx),
            self.button("cells.borders_outer", "table.format.borders_outer", cx),
            self.button("cells.borders_none", "table.format.borders_none", cx),
        ]);
        PropertyGroup::new(label("table.format.cell"))
            .row(label("table.format.borders"), borders)
            .row(label("table.format.fill"), self.colors(&FILL, cx))
            .row(label("table.format.text_color"), self.colors(&TEXT, cx))
    }

    /// A cell color as a swatch; it opens the palette.
    fn colors(&self, pick: &'static Pick, cx: &Context<Self>) -> AnyElement {
        let paint = paint(self.state(pick.state, cx).as_deref());
        let current = match paint {
            Paint::Color(color) => Some(color),
            _ => None,
        };
        let none = Book::text(cx, pick.none, &[]);
        let (shown, tip) = match paint {
            Paint::Color(color) => (Hsla::from(rgb(color)), format!("#{color:06X}").into()),
            Paint::Mixed => (gpui::transparent_black(), Book::text(cx, "office.props.mixed", &[])),
            Paint::None => (gpui::transparent_black(), none.clone()),
        };
        let label = (paint == Paint::Mixed).then(|| ely_office::panel::muted(tip.clone(), cx));
        let book = self.book.clone();
        let popover = Popover::with_opener(
            SharedString::from(format!("{}-popover", pick.id)),
            move |toggle| {
                ColorSwatch::new(SharedString::from(format!("{}-swatch", pick.id)), shown)
                    .tooltip(tip)
                    .on_click(move |window, cx| toggle(window, cx))
            },
            move |close, _, _| {
                let swatches = pick.colors.map(|color| (format!("#{color:06X}"), Hsla::from(rgb(color))));
                let picked = book.clone();
                let mut palette = ColorPalette::new(pick.id, swatches.clone())
                    .columns(5)
                    .on_change({
                        let close = close.clone();
                        move |color, window, cx| {
                            let at = swatches
                                .iter()
                                .position(|(_, swatch)| *swatch == color)
                                .expect("the palette hands back its own colors");
                            close(window, cx);
                            run(&picked, pick.command, &[&pick.colors[at].to_string()], cx);
                        }
                    });
                if let Some(color) = current.filter(|color| pick.colors.contains(color)) {
                    palette = palette.selected(rgb(color));
                }
                let book = book.clone();
                let clear = Button::new(SharedString::from(format!("{}-none", pick.id)), none.clone())
                    .size(ControlSize::Sm)
                    .on_click(move |_, window, cx| {
                        close(window, cx);
                        run(&book, pick.command, &["-1"], cx);
                    });
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_2()
                    .child(palette)
                    .child(clear)
            },
        );
        row([popover.into_any_element()].into_iter().chain(label))
    }
}

/// Runs a command on the active workbook.
fn run(book: &WeakEntity<Book>, id: &str, values: &[&str], cx: &mut App) {
    if let Some(book) = book.upgrade() {
        book.update(cx, |book, cx| book.run_active(id, values, cx));
    }
}

/// Controls side by side, wrapping when narrow.
fn row(items: impl IntoIterator<Item = AnyElement>) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_1()
        .children(items)
        .into_any_element()
}

impl Render for Panel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let grid = PropertyGrid::new("table-format")
            .group(self.number_group(cx))
            .group(self.align_group(cx))
            .group(self.cell_group(cx));
        div().size_full().child(
            ScrollArea::new("table-format-scroll").vertical()
                .size_full()
                .child(grid),
        )
    }
}
