//! The Insert menu and its dialog (DOC-05 to 08).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::NumberInput,
    overlays::{ConfirmDialog, Dialog as Card},
    primitives::Severity,
};
use gpui::{
    App, Context, Div, Entity, InteractiveElement as _, Menu, MenuItem, ParentElement as _,
    Styled as _, Window, actions, div,
};

use crate::actions::Dialog;
use crate::docs::Docs;
use ely_office::prelude::*;

actions!(
    doc,
    [
        InsertTable,
        InsertImage,
        InsertPageBreak,
        InsertSectionBreak,
        InsertPageNumber,
        InsertPageCount
    ]
);

/// ElyDoc's Insert menu, after File.
pub fn menu(cx: &App) -> Menu {
    let label = |key: &str| Docs::text(cx, key, &[]);
    let mut items = vec![
        MenuItem::action(label("doc.insert.table"), InsertTable),
        MenuItem::action(label("doc.insert.image"), InsertImage),
        MenuItem::separator(),
        MenuItem::action(label("doc.insert.page_break"), InsertPageBreak),
        MenuItem::action(label("doc.insert.section_break"), InsertSectionBreak),
        MenuItem::separator(),
        MenuItem::action(label("doc.insert.page_number"), InsertPageNumber),
        MenuItem::action(label("doc.insert.page_count"), InsertPageCount),
    ];
    items.extend(crate::refs::items(cx));
    Menu::new(label("menu.insert")).items(items)
}

/// Binds the Insert and Review actions on the window root.
pub fn bind(root: Div, cx: &mut Context<Docs>) -> Div {
    root.on_action(cx.listener(Docs::on_insert_table))
        .on_action(cx.listener(Docs::on_insert_image))
        .on_action(cx.listener(Docs::on_page_break))
        .on_action(cx.listener(Docs::on_section_break))
        .on_action(cx.listener(Docs::on_page_number))
        .on_action(cx.listener(Docs::on_page_count))
        .on_action(cx.listener(Docs::on_contents))
        .on_action(cx.listener(Docs::on_update))
        .on_action(cx.listener(Docs::on_footnote))
        .on_action(cx.listener(Docs::on_endnote))
        .on_action(cx.listener(Docs::on_bookmark))
        .on_action(cx.listener(Docs::on_link))
        .on_action(cx.listener(Docs::on_reference))
        .on_action(cx.listener(Docs::on_caption))
        .on_action(cx.listener(Docs::on_formula))
        .on_action(cx.listener(Docs::on_comment))
        .on_action(cx.listener(Docs::on_track))
        .on_action(cx.listener(Docs::on_accept_all))
        .on_action(cx.listener(Docs::on_reject_all))
        .on_action(cx.listener(Docs::on_show_review))
        .on_action(cx.listener(Docs::on_find))
        .on_action(cx.listener(Docs::on_compare))
        .on_action(cx.listener(Docs::on_merge))
        .on_action(cx.listener(Docs::on_export))
        .on_action(cx.listener(Docs::on_back))
        .on_action(cx.listener(Docs::on_print))
}

impl Docs {
    /// Runs a command on the active document.
    pub(crate) fn run_active(&mut self, id: &str, values: &[&str], cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            let text = Docs::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        log::info!("doc: runs {id}");
        let sent = tab.view.update(cx, |view, _| view.run(id, values));
        if let Err(error) = sent {
            let text = Docs::text(cx, "office.error.engine", &[("error", &error.to_string())]);
            self.toast(text, Severity::Danger, cx);
        }
    }

    pub fn on_insert_table(&mut self, _: &InsertTable, _: &mut Window, cx: &mut Context<Self>) {
        self.dialog = Some(Dialog::Table {
            columns: 2.0,
            rows: 2.0,
        });
        cx.notify();
    }

    pub fn on_insert_image(&mut self, _: &InsertImage, _: &mut Window, cx: &mut Context<Self>) {
        self.pick(false, cx, |docs, path, cx| {
            let url = ely_lok::file_url(&path);
            docs.run_active("image.insert", &[&url], cx)
        });
    }

    pub fn on_page_break(&mut self, _: &InsertPageBreak, _: &mut Window, cx: &mut Context<Self>) {
        self.run_active("page.break", &[], cx);
    }

    /// Asks first: a new section clears the undo history.
    pub fn on_section_break(
        &mut self,
        _: &InsertSectionBreak,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dialog = Some(Dialog::Section);
        cx.notify();
    }

    /// Copies this page's style, then breaks into it.
    pub fn insert_section(&mut self, cx: &mut Context<Self>) {
        self.dialog = None;
        let Some(tab) = self.active_tab() else {
            let text = Docs::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        let id = tab.id;
        // Page style names are read off the window thread.
        self.ask_in(
            id,
            |worker, doc| ely_lok::commands::styles(&worker.values(doc, ".uno:StyleApply")?, "8"),
            move |docs, listed, _, cx| {
                let names = match listed {
                    Ok(names) => names,
                    Err(error) => return docs.engine_failed(&error.to_string(), cx),
                };
                let name = (2..)
                    .map(|n| Docs::text(cx, "doc.section.name", &[("n", &n.to_string())]).to_string())
                    .find(|name| !names.contains(name))
                    .expect("a free section name");
                log::info!("doc: a new section");
                docs.run_in(Some(id), "style.new", &[&name, "8"], cx);
                docs.run_in(Some(id), "page.section", &["3", &name], cx);
                docs.props.update(cx, |props, _| props.refresh_lists());
                cx.notify();
            },
            cx,
        );
    }

    pub fn on_page_number(&mut self, _: &InsertPageNumber, _: &mut Window, cx: &mut Context<Self>) {
        self.run_active("field.page", &[], cx);
    }

    pub fn on_page_count(&mut self, _: &InsertPageCount, _: &mut Window, cx: &mut Context<Self>) {
        self.run_active("field.pages", &[], cx);
    }

    /// Inserts the dialog's table and closes it.
    pub fn insert_table(&mut self, columns: f64, rows: f64, cx: &mut Context<Self>) {
        self.dialog = None;
        let (columns, rows) = (columns.round().to_string(), rows.round().to_string());
        self.run_active("table.insert", &[&columns, &rows], cx);
        cx.notify();
    }
}

/// The section break's warning: undo clears.
pub fn section_dialog(
    this: Entity<Docs>,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> ConfirmDialog {
    ConfirmDialog::new(
        "doc-insert-section",
        Docs::text(cx, "doc.section.title", &[]),
        Docs::text(cx, "doc.section.detail", &[]),
        dismiss,
    )
    .confirm(Docs::text(cx, "doc.insert.insert", &[]))
    .on_confirm(move |_, cx| this.update(cx, |docs, cx| docs.insert_section(cx)))
}

/// The Insert Table dialog: columns and rows.
pub fn table_dialog(
    this: Entity<Docs>,
    (columns, rows): (f64, f64),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let field = |id: &'static str, value: f64, max: f64, tall: bool| {
        let this = this.clone();
        let input = NumberInput::new(id, value)
            .label(Docs::text(cx, if tall { "doc.insert.rows" } else { "doc.insert.columns" }, &[]))
            .range(1.0, max)
            .on_change(move |value, _, cx| {
                this.update(cx, |docs, cx| {
                    if let Some(Dialog::Table { columns, rows }) = &mut docs.dialog {
                        *if tall { rows } else { columns } = value;
                    }
                    cx.notify();
                })
            });
        div().w_full().max_w_32().child(input)
    };
    let fields = DescriptionList::new()
        .item(
            Docs::text(cx, "doc.insert.columns", &[]),
            field("doc-table-columns", columns, 99.0, false),
        )
        .item(
            Docs::text(cx, "doc.insert.rows", &[]),
            field("doc-table-rows", rows, 999.0, true),
        );
    let (label, cancel) = (
        Docs::text(cx, "doc.insert.insert", &[]),
        Docs::text(cx, "office.cancel", &[]),
    );
    Card::new(
        "doc-insert-table",
        Docs::text(cx, "doc.insert.table_title", &[]),
        dismiss,
    )
    .child(fields)
    .action(move |close| {
        Button::new("doc-insert-table-cancel", cancel)
            .on_click(move |_, window, cx| close(window, cx))
    })
    .action(move |_| {
        Button::new("doc-insert-table-go", label)
            .variant(ButtonVariant::Primary)
            .on_click(move |_, _, cx| {
                this.update(cx, |docs, cx| docs.insert_table(columns, rows, cx))
            })
    })
}
