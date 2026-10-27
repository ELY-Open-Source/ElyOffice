//! The page setup dialog.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    data_display::DescriptionList,
    forms::{Input, NumberInput},
    overlays::Dialog as Card,
    theme::ControlSize,
};
use ely_lok::sheet::{PrintLayout, Scale};
use gpui::{App, Entity, ParentElement as _, Styled as _, Window, div};

use super::PageFields;
use crate::book::Book;
use crate::dialog::{Dialog, edit};

/// Changes the dialog's orientation or scale.
fn set(this: &Entity<Book>, cx: &mut App, change: impl FnOnce(&mut bool, &mut Scale)) {
    edit(this, cx, |dialog, _, _| {
        if let Dialog::Page {
            landscape, scale, ..
        } = dialog
        {
            change(landscape, scale);
        }
    })
}

fn number(id: &'static str, this: &Entity<Book>, value: u16, range: (u16, u16), apply: fn(&mut Scale, u16)) -> NumberInput {
    let this = this.clone();
    NumberInput::new(id, f64::from(value))
        .range(f64::from(range.0), f64::from(range.1))
        .on_commit(move |value, _, cx| {
            let value = value.round() as u16;
            set(&this, cx, |_, scale| apply(scale, value))
        })
}

/// The scale's mode, then its numbers under the mode's name.
fn scale_rows(
    rows: DescriptionList,
    this: &Entity<Book>,
    scale: Scale,
    cx: &App,
) -> DescriptionList {
    let label = |key: &str| Book::text(cx, key, &[]);
    let mode = match scale {
        Scale::Percent(_) => "percent",
        Scale::Fit(..) => "fit",
        Scale::Pages(_) => "pages",
    };
    let picking = this.clone();
    let modes = SegmentedControl::new("table-page-scale", mode)
        .size(ControlSize::Sm)
        .segment("percent", label("table.page.percent"), None)
        .segment("fit", label("table.page.fit"), None)
        .segment("pages", label("table.page.pages"), None)
        .on_change(move |value, _, cx| {
            let chosen = match value.as_ref() {
                "fit" => Scale::Fit(1, 1),
                "pages" => Scale::Pages(1),
                _ => Scale::Percent(100),
            };
            set(&picking, cx, |_, scale| *scale = chosen)
        });
    let field = |input: NumberInput| div().w_full().max_w_24().child(input);
    let numbers = match scale {
        Scale::Percent(percent) => div().child(field(number(
            "table-page-percent",
            this,
            percent,
            (10, 400),
            |scale, value| *scale = Scale::Percent(value),
        ).label(label("table.page.percent")))),
        Scale::Fit(wide, tall) => div()
            .flex()
            .items_center()
            .gap_1()
            .child(field(number("table-page-wide", this, wide, (0, 1000), |scale, value| {
                if let Scale::Fit(wide, _) = scale {
                    *wide = value;
                }
            }).label(label("table.page.wide"))))
            .child("×")
            .child(field(number("table-page-tall", this, tall, (0, 1000), |scale, value| {
                if let Scale::Fit(_, tall) = scale {
                    *tall = value;
                }
            }).label(label("table.page.tall")))),
        Scale::Pages(pages) => div().child(field(number(
            "table-page-count",
            this,
            pages,
            (1, 1000),
            |scale, value| *scale = Scale::Pages(value),
        ).label(label("table.page.pages")))),
    };
    let named = label(&format!("table.page.{mode}"));
    rows.item(label("table.page.scale"), modes)
        .item(named, numbers)
}

pub fn page_dialog(
    this: Entity<Book>,
    (layout, landscape, scale, fields): (&PrintLayout, bool, Scale, &PageFields),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let label = |key: &str| Book::text(cx, key, &[]);
    let picking = this.clone();
    let orientation = SegmentedControl::new(
        "table-page-orientation",
        if landscape { "landscape" } else { "portrait" },
    )
    .size(ControlSize::Sm)
    .segment("portrait", label("table.page.portrait"), None)
    .segment("landscape", label("table.page.landscape"), None)
    .on_change(move |value, _, cx| {
        let on = value.as_ref() == "landscape";
        set(&picking, cx, |landscape, _| *landscape = on)
    });
    let input = |field| div().w_full().max_w_48().child(Input::new(field));
    let rows = DescriptionList::new()
        .item(label("table.page.area"), input(&fields.area))
        .item(label("table.page.rows"), input(&fields.rows))
        .item(label("table.page.columns"), input(&fields.columns))
        .item(label("table.page.orientation"), orientation);
    let rows = scale_rows(rows, &this, scale, cx);
    let mut card = Card::new("table-page", label("table.page.title"), dismiss);
    // The page style may govern other sheets too.
    if layout.sheets.len() > 1 {
        let sheets = layout.sheets.join(", ");
        card = card.detail(Book::text(cx, "table.page.shared", &[("sheets", &sheets)]));
    }
    let go = label("table.page.apply");
    card.child(rows).action(ely_office::cancel(cx)).action(move |_| {
        Button::new("table-page-go", go)
            .variant(ButtonVariant::Primary)
            .on_click(move |_, _, cx| this.update(cx, |book, cx| book.page_asked(cx)))
    })
}
