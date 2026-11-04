//! The Insert menu: pivot tables and charts (TABLE-10, 11).

use ely_gpui_component::forms::TextInput;
use ely_lok::sheet::{self, Chart, Layout, Pivot, Selection};
use ely_office::prelude::*;
use gpui::{
    App, Context, Div, Entity, InteractiveElement as _, Menu, MenuItem, Window,
    actions,
};

use crate::book::Book;
use crate::dialog::Dialog;

mod dialogs;
pub use dialogs::{chart_dialog, pivot_dialog};

actions!(table, [MakePivot, RefreshPivot, MakeChart]);

pub fn menu(cx: &App) -> Menu {
    let label = |key: &str| Book::text(cx, key, &[]);
    Menu::new(label("menu.insert")).items([
        MenuItem::action(label("table.pivot.make"), MakePivot),
        MenuItem::action(label("table.pivot.refresh"), RefreshPivot),
        MenuItem::separator(),
        MenuItem::action(label("table.chart.make"), MakeChart),
    ])
}

pub fn bind(root: Div, cx: &mut Context<Book>) -> Div {
    root.on_action(cx.listener(|book, _: &MakePivot, _, cx| book.ask(Asked::Pivot, cx)))
        .on_action(cx.listener(|book, _: &RefreshPivot, _, cx| book.ask(Asked::Refresh, cx)))
        .on_action(cx.listener(|book, _: &MakeChart, _, cx| book.ask(Asked::Chart, cx)))
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Asked {
    Pivot,
    Refresh,
    Chart,
}

/// A chart dialog's text fields.
pub struct ChartFields {
    pub range: Entity<TextInput>,
    pub look: ely_office::chart::LookFields,
}

/// What the kernel holds where the user asked.
enum Found {
    Pivot(Option<Pivot>),
    Chart(Option<Chart>),
}

impl Book {
    pub(crate) fn ask(&mut self, asked: Asked, cx: &mut Context<Self>) {
        let Some(id) = self.core.active() else {
            return self.warn_key("command.blocked.no_document", cx);
        };
        self.ask_here(id, asked, cx);
    }

    /// Reads the pivot table or chart here, after input.
    fn ask_here(&mut self, id: u64, asked: Asked, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |book, _, cx| book.ask_here(id, asked, cx));
        }
        self.ask_in(
            id,
            move |worker, doc| match asked {
                Asked::Chart => Ok(Found::Chart(sheet::chart(
                    &worker.values(doc, sheet::CHART)?,
                )?)),
                _ => Ok(Found::Pivot(sheet::pivot(
                    &worker.values(doc, sheet::PIVOT)?,
                )?)),
            },
            move |book, read, window, cx| match (asked, read) {
                (_, Err(error)) => book.engine_failed(&error.to_string(), cx),
                (Asked::Refresh, Ok(Found::Pivot(Some(_)))) => {
                    book.run_in(Some(id), "pivot.refresh", &[], cx)
                }
                (Asked::Refresh, Ok(_)) => book.warn_key("table.pivot.none", cx),
                (_, Ok(Found::Pivot(Some(pivot)))) => {
                    book.open_dialog(Dialog::Pivot {
                        tab: id,
                        source: pivot.source,
                        fields: pivot.fields,
                        layout: pivot.layout,
                        editing: true,
                    });
                    cx.notify();
                }
                (_, Ok(Found::Chart(Some(chart)))) => {
                    book.chart_dialog(id, Some(chart), "", window, cx)
                }
                (_, Ok(_)) => book.data_of(
                    id,
                    true,
                    move |book, picked, _, window, cx| book.new_dialog(id, asked, picked, window, cx),
                    cx,
                ),
            },
            cx,
        );
    }

    /// A create dialog over the picked data.
    fn new_dialog(
        &mut self,
        id: u64,
        asked: Asked,
        picked: Selection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        log::info!("table: {asked:?} from {}", picked.range);
        if let Asked::Chart = asked {
            return self.chart_dialog(id, None, &picked.data, window, cx);
        }
        if picked.top.iter().any(String::is_empty) {
            return self.warn_key("table.pivot.headers", cx);
        }
        self.open_dialog(Dialog::Pivot {
            tab: id,
            source: picked.data,
            fields: picked.top,
            layout: Layout::default(),
            editing: false,
        });
        cx.notify();
    }

    fn chart_dialog(
        &mut self,
        id: u64,
        chart: Option<Chart>,
        range: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let look = chart
            .as_ref()
            .map_or_else(ely_office::chart::fresh, |chart| chart.look.clone());
        let range = chart.as_ref().map_or(range, |chart| &chart.range);
        let fields = ChartFields {
            range: ely_office::chart::field(range, "table.chart.range", window, cx),
            look: ely_office::chart::LookFields::new(&look, window, cx),
        };
        self.open_dialog(Dialog::Chart {
            tab: id,
            chart: chart.map(Box::new),
            look: Box::new(look),
            fields: Box::new(fields),
        });
        cx.notify();
    }

    /// Makes or changes the pivot table the dialog describes.
    pub(crate) fn pivot_asked(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Pivot {
            tab,
            source,
            layout,
            editing,
            ..
        }) = self.dialog.take()
        else {
            return;
        };
        let source = if editing { "" } else { source.as_str() };
        self.run_in(Some(tab), "pivot.make", &[&layout.to_json(source, "")], cx);
    }

    /// Inserts or changes the chart the dialog describes.
    pub(crate) fn chart_asked(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Chart {
            tab,
            chart,
            look,
            fields,
        }) = self.dialog.take()
        else {
            return;
        };
        let look = fields.look.read(&look, cx).to_json();
        let range = fields.range.read(cx).text().trim().to_string();
        match chart {
            Some(chart) => {
                let headers = |on: bool| if on { "true" } else { "false" };
                let values = [
                    chart.name.as_str(),
                    &range,
                    headers(chart.col_headers),
                    headers(chart.row_headers),
                    &look,
                ];
                self.run_in(Some(tab), "chart.change", &values, cx)
            }
            None => self.run_in(Some(tab), "chart.insert", &[&range, "false", &look], cx),
        }
    }
}
