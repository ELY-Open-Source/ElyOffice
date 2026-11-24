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
mod fonts;
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

/// Border line colors, 0xRRGGBB.
const LINES: [u32; 10] = [
    0x000000, 0x595959, 0x7F7F7F, 0xBFBFBF, 0xC00000, 0xED7D31, 0xFFC000, 0x70AD47, 0x4472C4,
    0x7030A0,
];

/// Border line styles: the kernel's number, the sample, the label key.
const EDGE_STYLES: [(i64, &str, &str); 7] = [
    (0, "────", "table.format.line_solid"),
    (1, "┈┈┈┈", "table.format.line_dotted"),
    (2, "─ ─ ─", "table.format.line_dashed"),
    (14, "╌╌╌╌", "table.format.line_fine"),
    (16, "─ · ─", "table.format.line_dash_dot"),
    (17, "─ ·· ─", "table.format.line_dash_dot_dot"),
    (3, "════", "table.format.line_double"),
];

/// Border line widths in 1/100 mm and their label keys.
const EDGE_WIDTHS: [(i64, &str); 5] = [
    (9, "table.format.width_hair"),
    (18, "table.format.width_thin"),
    (35, "table.format.width_medium"),
    (53, "table.format.width_thick"),
    (88, "table.format.width_heavy"),
];

/// A double line shows nothing narrower.
const DOUBLE_MIN: i64 = 35;

/// Where a border button draws: left, right, top, bottom, inside
/// horizontal, inside vertical; `1` sets, `x` clears, `0` leaves.
const EDGE_PRESETS: [(&str, &str); 11] = [
    ("111111", "table.format.borders_all"),
    ("111100", "table.format.borders_outer"),
    ("000011", "table.format.borders_inner"),
    ("000010", "table.format.borders_inner_h"),
    ("000001", "table.format.borders_inner_v"),
    ("001000", "table.format.borders_top"),
    ("000100", "table.format.borders_bottom"),
    ("100000", "table.format.borders_left"),
    ("010000", "table.format.borders_right"),
    ("001100", "table.format.borders_rows"),
    ("110000", "table.format.borders_columns"),
];

/// The line the border buttons draw.
#[derive(Clone, Copy)]
struct Line {
    style: i64,
    width: i64,
    color: u32,
}

impl Line {
    /// The edge value for a preset mark.
    fn edge(self, mark: char) -> String {
        match mark {
            '1' => {
                let width = if self.style == 3 { self.width.max(DOUBLE_MIN) } else { self.width };
                format!("{}:{}:{}", self.style, width, self.color)
            }
            'x' => "none".into(),
            _ => "keep".into(),
        }
    }
}

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
    /// The kernel's font families.
    pub(crate) fonts: Vec<SharedString>,
    /// The line the border buttons draw.
    line: Line,
    /// The cell and the border buttons last drawn there; a new style
    /// restyles them at once.
    drawn: Option<(Spot, &'static str)>,
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
            fonts: Vec::new(),
            line: Line { style: 0, width: 18, color: 0 },
            drawn: None,
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
        if let Some(shown) = self.shown.as_ref().filter(|_| self.fonts.is_empty()) {
            let doc = shown.doc.clone();
            self.read_fonts(&doc, cx);
        }
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
        if self.drawn.as_ref().is_some_and(|(spot, _)| spot.0 != at.0 || spot.1 != at.1) {
            self.drawn = None;
        }
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
            .and_then(|book| book.read(cx).core.active())
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

    fn cell_group(&self, window: &mut Window, cx: &mut Context<Self>) -> PropertyGroup {
        let [font, size] = self.font_rows(window, cx);
        let label = |key: &str| Book::text(cx, key, &[]);
        let mut buttons: Vec<AnyElement> = EDGE_PRESETS
            .iter()
            .map(|(marks, key)| self.edge_button(marks, key, cx))
            .collect();
        buttons.push(self.edge_button("xxxxxx", "table.format.borders_none", cx));
        PropertyGroup::new(label("table.format.cell"))
            .row(label("table.format.font"), font)
            .row(label("table.format.size"), size)
            .row(label("table.format.line_style"), self.line_style(cx))
            .row(label("table.format.line_width"), self.line_width(cx))
            .row(label("table.format.line_color"), self.line_color(cx))
            .row(label("table.format.borders"), row(buttons))
            .row(label("table.format.fill"), self.colors(&FILL, cx))
            .row(label("table.format.text_color"), self.colors(&TEXT, cx))
    }

    /// A button that draws the line on the edges `marks` names.
    fn edge_button(&self, marks: &'static str, key: &str, cx: &Context<Self>) -> AnyElement {
        let panel = cx.entity().downgrade();
        Button::new(SharedString::from(format!("edges-{marks}")), Book::text(cx, key, &[]))
            .size(ControlSize::Sm)
            .on_click(move |_, _, cx| {
                panel
                    .update(cx, |panel, cx| {
                        // Clearing leaves nothing to restyle.
                        panel.drawn = (!marks.contains('x'))
                            .then(|| panel.read.clone().map(|spot| (spot, marks)))
                            .flatten();
                        panel.draw(marks, cx);
                    })
                    .ok();
            })
            .into_any_element()
    }

    /// Draws the current line on the edges `marks` names.
    fn draw(&self, marks: &str, cx: &mut Context<Self>) {
        let values: Vec<String> = marks.chars().map(|mark| self.line.edge(mark)).collect();
        let values: Vec<&str> = values.iter().map(String::as_str).collect();
        run(&self.book, "cells.border", &values, cx);
    }

    /// A new line restyles what the buttons last drew.
    fn restyle(&mut self, cx: &mut Context<Self>) {
        if let Some((_, marks)) = self.drawn.clone() {
            self.draw(marks, cx);
        }
        cx.notify();
    }

    fn line_style(&self, cx: &Context<Self>) -> AnyElement {
        let choices: Vec<Choice> = EDGE_STYLES
            .iter()
            .map(|(style, sample, key)| {
                let name = Book::text(cx, key, &[]);
                Choice::new(style.to_string(), format!("{sample}  {name}"))
            })
            .collect();
        let panel = cx.entity().downgrade();
        Select::new("table-line-style", choices)
            .selected(self.line.style.to_string())
            .size(ControlSize::Sm)
            .on_change(move |value, _, cx| {
                if let Ok(style) = value.parse() {
                    panel.update(cx, |panel, cx| { panel.line.style = style; panel.restyle(cx); }).ok();
                }
            })
            .into_any_element()
    }

    fn line_width(&self, cx: &Context<Self>) -> AnyElement {
        let choices: Vec<Choice> = EDGE_WIDTHS
            .iter()
            .map(|(width, key)| Choice::new(width.to_string(), Book::text(cx, key, &[])))
            .collect();
        let panel = cx.entity().downgrade();
        Select::new("table-line-width", choices)
            .selected(self.line.width.to_string())
            .size(ControlSize::Sm)
            .on_change(move |value, _, cx| {
                if let Ok(width) = value.parse() {
                    panel.update(cx, |panel, cx| { panel.line.width = width; panel.restyle(cx); }).ok();
                }
            })
            .into_any_element()
    }

    /// The border color as a swatch; it opens the palette.
    fn line_color(&self, cx: &Context<Self>) -> AnyElement {
        let current = self.line.color;
        let panel = cx.entity().downgrade();
        Popover::with_opener(
            "table-line-popover",
            move |toggle| {
                ColorSwatch::new("table-line-swatch", Hsla::from(rgb(current)))
                    .tooltip(format!("#{current:06X}"))
                    .on_click(move |window, cx| toggle(window, cx))
            },
            move |close, _, _| {
                let swatches = LINES.map(|color| (format!("#{color:06X}"), Hsla::from(rgb(color))));
                let panel = panel.clone();
                ColorPalette::new("table-line", swatches.clone())
                    .columns(5)
                    .selected(rgb(current))
                    .on_change(move |color, window, cx| {
                        if let Some(at) = swatches.iter().position(|(_, swatch)| *swatch == color) {
                            close(window, cx);
                            panel
                                .update(cx, |panel, cx| {
                                    panel.line.color = LINES[at];
                                    panel.restyle(cx);
                                })
                                .ok();
                        }
                    })
            },
        )
        .into_any_element()
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let grid = PropertyGrid::new("table-format")
            .group(self.number_group(cx))
            .group(self.align_group(cx))
            .group(self.cell_group(window, cx));
        div().size_full().child(
            ScrollArea::new("table-format-scroll").vertical()
                .size_full()
                .child(grid),
        )
    }
}
