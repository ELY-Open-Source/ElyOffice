//! Tables on a slide (SHOW-07).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::{DescriptionList, PropertyGroup},
    forms::NumberInput,
    overlays::Dialog as Card,
};
use ely_office::{
    panel::{button, row},
    prelude::*,
};
use gpui::{
    AnyElement, App, Context, Entity, IntoElement, ParentElement as _, Styled as _, Window, div,
};

use crate::deck::Deck;
use crate::design::Panel;
use crate::dialog::{Dialog, edit};

impl Deck {
    pub fn ask_table(&mut self, cx: &mut Context<Self>) {
        if let Some(tab) = self.core.active() {
            // Refused while another dialog is open; it says so.
            let _ = self.open_dialog(Dialog::Table { tab, columns: 3, rows: 3 }, cx);
        }
    }

    pub(crate) fn table_asked(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Table { tab, columns, rows }) = self.dialog.take() else {
            return;
        };
        let (columns, rows) = (columns.to_string(), rows.to_string());
        self.run_in(Some(tab), "table.insert", &[&columns, &rows], cx);
        self.force_reread(tab, cx);
        cx.notify();
    }

    pub fn table_dialog(&self, (columns, rows): (u16, u16), cx: &mut Context<Self>) -> AnyElement {
        let this = cx.entity();
        let label = |key: &str| Deck::text(cx, key, &[]);
        let count = |id: &'static str, key: &str, now: u16, max: f64, across: bool| {
            // A typed draft counts before blur commits it.
            let set = move |this: &Entity<Deck>, n: f64, cx: &mut App| {
                edit(this, cx, |dialog, _| {
                    if let Dialog::Table { columns, rows, .. } = dialog {
                        *(if across { columns } else { rows }) = n as u16;
                    }
                })
            };
            let (typed, committed) = (this.clone(), this.clone());
            let field = NumberInput::new(id, f64::from(now))
                .label(label(key))
                .range(1.0, max)
                .on_change(move |n, _, cx| set(&typed, n, cx))
                .on_commit(move |n, _, cx| set(&committed, n, cx));
            div().w_full().max_w_32().child(field)
        };
        let rows = DescriptionList::new()
            .item(
                label("show.table.columns"),
                count(
                    "show-table-columns",
                    "show.table.columns",
                    columns,
                    20.0,
                    true,
                ),
            )
            .item(
                label("show.table.rows"),
                count("show-table-rows", "show.table.rows", rows, 50.0, false),
            );
        let closing = this.clone();
        let dismiss = move |_: &mut Window, cx: &mut App| {
            closing.update(cx, |deck, cx| {
                deck.dialog = None;
                cx.notify();
            })
        };
        let go = label("show.table.go");
        Card::new("show-table", label("show.table.title"), dismiss)
            .child(rows)
            .action(ely_office::cancel(cx))
            .action(move |_| {
                Button::new("show-table-go", go)
                    .variant(ButtonVariant::Primary)
                    .on_click(move |_, _, cx| this.update(cx, |deck, cx| deck.table_asked(cx)))
            })
            .into_any_element()
    }
}

impl Panel {
    /// Rows, columns, cells and the table itself.
    pub(crate) fn table_group(&self, cx: &mut Context<Self>) -> PropertyGroup {
        let why = "show.table.why";
        let rows = row([
            button(self, "table.row_above", "show.table.row_above", why, cx),
            button(self, "table.row_below", "show.table.row_below", why, cx),
            button(self, "table.delete_rows", "show.table.delete", why, cx),
        ]);
        let columns = row([
            button(self, "table.column_left", "show.table.column_left", why, cx),
            button(
                self,
                "table.column_right",
                "show.table.column_right",
                why,
                cx,
            ),
            button(self, "table.delete_columns", "show.table.delete", why, cx),
        ]);
        let cells = row([
            button(self, "table.merge", "show.table.merge", why, cx),
            button(self, "table.delete", "show.table.delete_table", why, cx),
        ]);
        let tip = |key: &str| Deck::text(cx, key, &[]);
        PropertyGroup::new(tip("show.table.group"))
            .row(tip("show.table.rows"), rows)
            .row(tip("show.table.columns"), columns)
            .row(tip("show.table.cells"), cells)
    }
}
