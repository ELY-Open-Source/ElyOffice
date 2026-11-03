//! Explicit, experimental R macros over the selected range.

mod files;
#[cfg(test)]
mod integration;
mod protocol;
mod runner;

use crate::{book::Book, dialog::Dialog};
use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::{Input, TextInput},
    overlays::Dialog as Card,
};
use ely_office::prelude::*;
use gpui::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, actions, div,
};

const EXAMPLE: &str = include_str!("../../../../examples/macros/limpiar-textos.R");
actions!(ely_table, [RunRMacro]);

pub struct Editor {
    pub tab: u64,
    pub code: Entity<TextInput>,
    pub output: String,
    pub running: bool,
}

pub fn menu(cx: &App) -> gpui::Menu {
    gpui::Menu::new(Book::text(cx, "table.r.menu", &[])).items([gpui::MenuItem::action(
        Book::text(cx, "table.r.title", &[]),
        RunRMacro,
    )])
}

impl Book {
    pub fn on_r_macro(&mut self, _: &RunRMacro, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active else {
            return self.warn_key("command.blocked.no_document", cx);
        };
        let label = Self::text(cx, "table.r.code", &[]);
        let code = cx.new(|cx| {
            let mut field = TextInput::new(window, cx).multi_line(10, 16).label(label);
            field.set_text(EXAMPLE, cx);
            field
        });
        self.open_dialog(Dialog::RMacro(Box::new(Editor {
            tab,
            code,
            output: String::new(),
            running: false,
        })));
        cx.notify();
    }

    fn r_output(
        &mut self,
        code: &Entity<TextInput>,
        text: String,
        running: bool,
        cx: &mut Context<Self>,
    ) {
        if let Some(Dialog::RMacro(editor)) = &mut self.dialog
            && editor.code == *code
        {
            editor.output.push_str(&text);
            editor.running = running;
            cx.notify();
        }
    }

    fn execute_r(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::RMacro(editor)) = &mut self.dialog else {
            return;
        };
        if editor.running {
            return;
        }
        let (tab, code, script) = (
            editor.tab,
            editor.code.clone(),
            editor.code.read(cx).text().to_string(),
        );
        editor.running = true;
        editor.output.clear();
        cx.notify();
        if !runner::available() {
            self.r_output(&code, runner::MISSING.into(), false, cx);
            return;
        }
        if !self.settled(tab, cx) {
            self.r_output(
                &code,
                Self::text(cx, "table.r.busy", &[]).to_string(),
                false,
                cx,
            );
            return;
        }
        log::info!("table: R macro snapshot for tab {tab}");
        self.read_in(tab, |worker, doc| {
            let at = worker.now(doc)?;
            let snapshot = protocol::Snapshot::parse(&worker.values(doc, ".uno:ElyMacroSelection")?)?;
            Ok((at, snapshot))
        }, move |book, read, _, cx| {
            let (at, snapshot) = match read {
                Ok(read) => read,
                Err(error) => { book.r_output(&code, error.to_string(), false, cx); return; }
            };
            cx.spawn(async move |this, cx| {
                let outcome = cx.background_executor().spawn(async move { runner::run(snapshot, script) }).await;
                let _ = this.update(cx, |book, cx| {
                    let same = matches!(&book.dialog, Some(Dialog::RMacro(editor)) if editor.code == code && editor.running);
                    if !same || book.tab(tab).is_none() { return; }
                    book.r_output(&code, outcome.log, true, cx);
                    let batch = match outcome.batch {
                        Ok(batch) => batch,
                        Err(error) => { book.r_output(&code, format!("\n{error}"), false, cx); return; }
                    };
                    log::info!("table: R macro applies to tab {tab}");
                    book.read_in(tab, move |worker, _| worker.run(&at, "macro.apply", &[&batch]),
                        move |book, result, _, cx| {
                            let text = match result {
                                Ok(()) => Self::text(cx, "table.r.done", &[]).to_string(),
                                Err(error) => error.to_string(),
                            };
                            book.r_output(&code, format!("\n{text}"), false, cx);
                        }, cx);
                });
            }).detach();
        }, cx);
    }
}

pub fn dialog(
    this: Entity<Book>,
    editor: &Editor,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let content = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(Book::text(cx, "table.r.permissions", &[]))
        .child(Input::new(&editor.code))
        .child(
            div()
                .id("r-output")
                .max_h_40()
                .overflow_y_scroll()
                .child(editor.output.clone()),
        );
    let open = this.clone();
    let save = this.clone();
    let disabled = editor.running;
    let open_label = Book::text(cx, "table.r.open", &[]);
    let save_label = Book::text(cx, "table.r.save", &[]);
    let run_label = Book::text(
        cx,
        if disabled {
            "table.r.running"
        } else {
            "table.r.run"
        },
        &[],
    );
    Card::new(
        "table-r-macro",
        Book::text(cx, "table.r.title", &[]),
        dismiss,
    )
    .child(content)
    .action(move |_| {
        Button::new("r-open", open_label)
            .disabled(disabled)
            .on_click(move |_, _, cx| open.update(cx, |book, cx| book.r_file(false, cx)))
    })
    .action(move |_| {
        Button::new("r-save", save_label)
            .disabled(disabled)
            .on_click(move |_, _, cx| save.update(cx, |book, cx| book.r_file(true, cx)))
    })
    .action(move |_| {
        Button::new("r-run", run_label)
            .variant(ButtonVariant::Primary)
            .disabled(disabled)
            .on_click(move |_, _, cx| this.update(cx, |book, cx| book.execute_r(cx)))
    })
}
