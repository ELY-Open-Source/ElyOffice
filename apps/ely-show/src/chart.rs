//! Charts on a slide: look and their own table (SHOW-07).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    feedback::InlineMessage,
    forms::{Input, TextInput},
    overlays::Dialog as Card,
    primitives::Severity,
    theme::ControlSize,
};
use ely_lok::{
    sheet::Look,
    show::{ChartData, Objects, SlideChart},
};
use ely_office::{
    chart::{LookFields, field, fresh, look_rows},
    in_window,
    prelude::*,
};
use gpui::{
    AnyElement, App, Context, Entity, IntoElement, ParentElement as _, SharedString, Styled as _,
    TaskExt as _, Window, div,
};

use crate::deck::Deck;
use crate::dialog::{Dialog, edit};

/// A new chart's table, as the kernel's sample has it.
const SAMPLE: [[f64; 3]; 2] = [[4.3, 2.5, 3.5], [2.4, 4.4, 1.8]];

/// The chart dialog: look, then its table if any.
pub struct Editing {
    pub tab: u64,
    /// The marked chart changes; otherwise a new one goes in.
    pub changing: bool,
    pub look: Look,
    pub fields: LookFields,
    pub table: Option<Cells>,
    pub error: Option<SharedString>,
}

/// A category by series table of fields.
pub struct Cells {
    pub categories: Vec<Entity<TextInput>>,
    pub series: Vec<(Entity<TextInput>, Vec<Entity<TextInput>>)>,
}

fn number(value: Option<f64>) -> String {
    value.map(|n| n.to_string()).unwrap_or_default()
}

impl Editing {
    fn new(tab: u64, chart: Option<SlideChart>, window: &mut Window, cx: &mut App) -> Self {
        let changing = chart.is_some();
        let text = |key: &str, n: usize, cx: &App| {
            Deck::text(cx, key, &[("n", &n.to_string())]).to_string()
        };
        let (look, data) = match chart {
            Some(chart) => (chart.look, chart.data),
            None => {
                let categories = (1..=3)
                    .map(|n| text("show.chart.category", n, cx))
                    .collect();
                let series = SAMPLE
                    .iter()
                    .enumerate()
                    .map(|(at, values)| {
                        let name = text("show.chart.series", at + 1, cx);
                        (name, values.iter().map(|v| Some(*v)).collect())
                    })
                    .collect();
                (fresh(), Some(ChartData { categories, series }))
            }
        };
        Editing {
            tab,
            changing,
            fields: LookFields::new(&look, window, cx),
            look,
            table: data.map(|data| Cells {
                categories: (data.categories.iter())
                    .map(|name| field(name, "show.chart.category_name", window, cx))
                    .collect(),
                series: (data.series.iter())
                    .map(|(name, values)| {
                        let cells = (values.iter())
                            .map(|value| field(&number(*value), "show.chart.value", window, cx))
                            .collect();
                        (field(name, "show.chart.series_name", window, cx), cells)
                    })
                    .collect(),
            }),
            error: None,
        }
    }

    /// The typed table, or the first bad cell.
    fn data(&self, kind: &str, cx: &App) -> Result<Option<ChartData>, SharedString> {
        let Some(table) = &self.table else {
            return Ok(None);
        };
        let text = |field: &Entity<TextInput>| field.read(cx).text().trim().to_string();
        let categories: Vec<String> = table.categories.iter().map(text).collect();
        // A scatter's categories are its X values.
        let number = |typed: &str| typed.parse::<f64>().ok().filter(|v| v.is_finite());
        if kind == "scatter"
            && let Some(bad) = categories
                .iter()
                .find(|category| number(category).is_none())
        {
            return Err(Deck::text(
                cx,
                "show.chart.x_not_a_number",
                &[("category", bad)],
            ));
        }
        let mut series = Vec::new();
        for (name, cells) in &table.series {
            let mut values = Vec::new();
            for (at, cell) in cells.iter().enumerate() {
                let typed = text(cell);
                // An empty cell is a missing point.
                if typed.is_empty() {
                    values.push(None);
                    continue;
                }
                let Some(value) = typed.parse::<f64>().ok().filter(|v| v.is_finite()) else {
                    let args = [("series", text(name)), ("category", categories[at].clone())];
                    let args: Vec<(&str, &str)> =
                        args.iter().map(|(k, v)| (*k, v.as_str())).collect();
                    return Err(Deck::text(cx, "show.chart.not_a_number", &args));
                };
                values.push(Some(value));
            }
            series.push((text(name), values));
        }
        Ok(Some(ChartData { categories, series }))
    }
}

impl Deck {
    /// Opens the chart dialog: marked chart, else new.
    pub fn ask_chart(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let (id, doc) = (tab.id, tab.doc.clone());
        let Some(asked) = marked_chart(self) else {
            return self.open_chart((id, None), None, cx);
        };
        ely_lok::read_off(
            &self.core.worker,
            &doc,
            |worker, doc| {
                let part = doc.sheets(worker)?.current;
                let objects = ely_lok::show::objects(&worker.values(doc, ely_lok::show::OBJECTS)?)?;
                let chart =
                    ely_lok::show::slide_chart(&worker.values(doc, ely_lok::sheet::CHART)?)?;
                Ok((marked(part, &objects), chart))
            },
            move |deck: &mut Self, read: std::io::Result<_>, cx| match read {
                // What the read saw must be what was asked for.
                Ok((seen, chart)) if seen == Some(asked) => {
                    deck.open_chart((id, Some(asked)), chart, cx)
                }
                Ok(_) => log::info!("show: the chart changed before its read came back"),
                Err(error) => deck.engine_failed(&format!("the chart: {error}"), cx),
            },
            cx,
        );
    }

    /// Opens the dialog if tab and marked chart still match.
    fn open_chart(
        &mut self,
        (tab, asked): (u64, Option<Marked>),
        chart: Option<SlideChart>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            in_window(&this, cx, |deck, window, cx| {
                let still = deck.core.active() == Some(tab)
                    && asked.is_none_or(|asked| marked_chart(deck) == Some(asked));
                if !still {
                    return log::info!("show: a chart dialog for tab {tab} came late");
                }
                let editing = Editing::new(tab, chart, window, cx);
                // Refused while another dialog is open; it says so.
                let _ = deck.open_dialog(Dialog::Chart(Box::new(editing)), cx);
            })
        })
        .detach_and_log_err(cx);
    }

    /// Inserts or changes the chart the dialog holds.
    pub(crate) fn chart_asked(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Chart(mut editing)) = self.dialog.take() else {
            return;
        };
        let look = editing.fields.read(&editing.look, cx);
        match editing.data(&look.kind, cx) {
            Ok(data) => {
                let json = ely_lok::show::chart_json(&look, data.as_ref());
                self.run_in(Some(editing.tab), "show.chart", &[&json], cx);
                self.force_reread(editing.tab, cx);
            }
            Err(error) => {
                editing.error = Some(error);
                self.dialog = Some(Dialog::Chart(editing));
            }
        }
        cx.notify();
    }

    pub fn chart_dialog(&self, editing: &Editing, cx: &mut Context<Self>) -> AnyElement {
        let this = cx.entity();
        let label = |key: &str| Deck::text(cx, key, &[]);
        let (picking, toggling) = (this.clone(), this.clone());
        let (rows, legend) = look_rows(
            "show-chart",
            DescriptionList::new(),
            (&editing.look, &editing.fields),
            move |kind, cx| edit(&picking, cx, |dialog, _| chart(dialog).look.kind = kind),
            move |on, cx| edit(&toggling, cx, |dialog, _| chart(dialog).look.legend = on),
            cx,
        );
        let (title, go) = match editing.changing {
            true => ("office.chart.edit_title", "office.chart.apply"),
            false => ("office.chart.insert_title", "office.chart.go"),
        };
        let closing = this.clone();
        let dismiss = move |_: &mut Window, cx: &mut App| {
            closing.update(cx, |deck, cx| {
                deck.dialog = None;
                cx.notify();
            })
        };
        let go = label(go);
        Card::new("show-chart", label(title), dismiss)
            .child(rows)
            .child(legend)
            .child(match &editing.table {
                Some(cells) => table(&this, cells, cx),
                None => InlineMessage::new(Severity::Info, label("show.chart.no_table"))
                    .into_any_element(),
            })
            .children(
                editing
                    .error
                    .clone()
                    .map(|error| InlineMessage::new(Severity::Danger, error)),
            )
            .action(ely_office::cancel(cx))
            .action(move |_| {
                Button::new("show-chart-go", go)
                    .variant(ButtonVariant::Primary)
                    .on_click(move |_, _, cx| this.update(cx, |deck, cx| deck.chart_asked(cx)))
            })
            .into_any_element()
    }
}

/// A marked chart: slide, place in the object list, frame.
type Marked = (usize, usize, (i64, i64, i64, i64));

/// The one marked object on slide `part`, if a chart.
fn marked(part: usize, objects: &Objects) -> Option<Marked> {
    let selection = objects
        .selection
        .as_ref()
        .filter(|s| s.kind == "chart" && s.count == 1)?;
    let mut picked = objects
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.selected);
    let (at, _) = picked.next().filter(|_| picked.next().is_none())?;
    Some((part, at, selection.rect))
}

/// The marked chart as the deck last read it.
fn marked_chart(deck: &Deck) -> Option<Marked> {
    let tab = deck.active_tab()?;
    marked(tab.extra.slides.current, tab.extra.objects.as_ref()?)
}

fn cells(dialog: &mut Dialog) -> &mut Cells {
    chart(dialog)
        .table
        .as_mut()
        .expect("the resize buttons need a table")
}

fn chart(dialog: &mut Dialog) -> &mut Editing {
    match dialog {
        Dialog::Chart(editing) => editing,
        _ => unreachable!("a chart dialog"),
    }
}

/// Series across, categories down, with grow and shrink buttons.
fn table(this: &Entity<Deck>, editing: &Cells, cx: &App) -> AnyElement {
    let cell = |input: &Entity<TextInput>| div().w_20().child(Input::new(input));
    let header = div()
        .flex()
        .gap_1()
        .child(div().w_20())
        .children(editing.series.iter().map(|(name, _)| cell(name)));
    let rows = editing.categories.iter().enumerate().map(|(at, name)| {
        div()
            .flex()
            .gap_1()
            .child(cell(name))
            .children(editing.series.iter().map(|(_, cells)| cell(&cells[at])))
    });
    let label = |key: &str| Deck::text(cx, key, &[]);
    let grow = |id: &'static str, key: &str, more: bool, across: bool, on: bool| {
        let this = this.clone();
        Button::new(id, label(key))
            .size(ControlSize::Sm)
            .disabled(!on)
            .on_click(move |_, window, cx| {
                edit(&this, cx, |dialog, cx| {
                    resize(cells(dialog), (more, across), window, cx)
                })
            })
    };
    let buttons = div()
        .flex()
        .gap_1()
        .child(grow(
            "show-chart-add-row",
            "show.chart.add_category",
            true,
            false,
            true,
        ))
        .child(grow(
            "show-chart-drop-row",
            "show.chart.drop_category",
            false,
            false,
            editing.categories.len() > 1,
        ))
        .child(grow(
            "show-chart-add-column",
            "show.chart.add_series",
            true,
            true,
            true,
        ))
        .child(grow(
            "show-chart-drop-column",
            "show.chart.drop_series",
            false,
            true,
            editing.series.len() > 1,
        ));
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(header)
        .children(rows)
        .child(buttons)
        .into_any_element()
}

/// Adds or drops the last category or series.
fn resize(editing: &mut Cells, (more, across): (bool, bool), window: &mut Window, cx: &mut App) {
    let blank = |key: &str, window: &mut Window, cx: &mut App| field("", key, window, cx);
    match (more, across) {
        (true, false) => {
            editing
                .categories
                .push(blank("show.chart.category_name", window, cx));
            for (_, cells) in &mut editing.series {
                cells.push(blank("show.chart.value", window, cx));
            }
        }
        (false, false) => {
            editing.categories.pop();
            for (_, cells) in &mut editing.series {
                cells.pop();
            }
        }
        (true, true) => {
            let cells = (0..editing.categories.len())
                .map(|_| blank("show.chart.value", window, cx))
                .collect();
            editing
                .series
                .push((blank("show.chart.series_name", window, cx), cells));
        }
        (false, true) => {
            editing.series.pop();
        }
    }
}
