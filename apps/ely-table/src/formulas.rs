//! The Formula menu: calculation, names, tracing (TABLE-03..06).

use ely_gpui_component::{forms::TextInput, primitives::Severity};
use ely_lok::sheet;
use ely_office::prelude::*;
use gpui::{App, AppContext as _, Context, Div, InteractiveElement as _, Menu, MenuItem, actions};

use crate::book::Book;
use crate::dialog::Dialog;

mod dialogs;
pub use dialogs::{define_dialog, names_dialog};

actions!(
    table,
    [
        AutoCalculate,
        Recalculate,
        RecalculateAll,
        DefineName,
        ManageNames,
        TraceError,
        TracePrecedents,
        TraceDependents,
        RemoveArrows
    ]
);

pub fn menu(cx: &App) -> Menu {
    let label = |key: &str| Book::text(cx, key, &[]);
    Menu::new(label("menu.formula")).items([
        MenuItem::action(label("table.calc.auto"), AutoCalculate),
        MenuItem::action(label("table.calc.now"), Recalculate),
        MenuItem::action(label("table.calc.all"), RecalculateAll),
        MenuItem::separator(),
        MenuItem::action(label("table.names.define"), DefineName),
        MenuItem::action(label("table.names.manage"), ManageNames),
        MenuItem::separator(),
        MenuItem::action(label("table.trace.error"), TraceError),
        MenuItem::action(label("table.trace.precedents"), TracePrecedents),
        MenuItem::action(label("table.trace.dependents"), TraceDependents),
        MenuItem::action(label("table.trace.clear"), RemoveArrows),
    ])
}

/// Binds an action to a command without values.
macro_rules! run {
    ($root:expr, $cx:expr, $action:ty, $id:literal) => {
        $root.on_action($cx.listener(|book, _: &$action, _, cx| book.run_active($id, &[], cx)))
    };
}

pub fn bind(root: Div, cx: &mut Context<Book>) -> Div {
    let root = run!(root, cx, Recalculate, "calc.now");
    let root = run!(root, cx, RecalculateAll, "calc.all");
    let root = run!(root, cx, TraceError, "trace.error");
    let root = run!(root, cx, TracePrecedents, "trace.precedents");
    let root = run!(root, cx, TraceDependents, "trace.dependents");
    let root = run!(root, cx, RemoveArrows, "trace.clear");
    root.on_action(cx.listener(|book, _: &AutoCalculate, _, cx| book.toggle_calculation(cx)))
        .on_action(cx.listener(|book, _: &DefineName, _, cx| book.ask_define(cx)))
        .on_action(cx.listener(|book, _: &ManageNames, _, cx| book.ask_names(cx)))
}

impl Book {
    /// Whether the active workbook calculates by itself.
    pub fn automatic(&self, cx: &App) -> Option<bool> {
        let tab = self.active_tab()?;
        match tab.view.read(cx).state(".uno:AutomaticCalculation")? {
            "true" => Some(true),
            "false" => Some(false),
            other => {
                log::warn!("table: calculation state {other:?}");
                None
            }
        }
    }

    fn toggle_calculation(&mut self, cx: &mut Context<Self>) {
        let Some(automatic) = self.automatic(cx) else {
            let text = Book::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        let next = (!automatic).to_string();
        self.run_active("calc.auto", &[&next], cx);
    }

    /// Asks for a name, prefilled with the selection.
    pub(crate) fn ask_define(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.core.active() else {
            let text = Book::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        self.define_in(id, cx);
    }

    /// The define dialog for tab `id`, once its input lands.
    fn define_in(&mut self, id: u64, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |book, _, cx| book.define_in(id, cx));
        }
        self.ask_in(
            id,
            |worker, doc| sheet::selected_range(&worker.values(doc, sheet::SELECTED)?),
            move |book, read, window, cx| {
                let refers = match read {
                    Ok(Some(picked)) => picked.range,
                    Ok(None) => {
                        // Several ranges: the person types it.
                        let text = Book::text(cx, "table.names.unresolved", &[]);
                        book.toast(text, Severity::Info, cx);
                        String::new()
                    }
                    Err(error) => return book.engine_failed(&error.to_string(), cx),
                };
                let name = cx.new(|cx| TextInput::new(window, cx));
                let range = cx.new(|cx| TextInput::new(window, cx));
                range.update(cx, |range, cx| range.set_text(refers, cx));
                book.open_dialog(Dialog::Define {
                    tab: id,
                    name,
                    range,
                    scope: String::new(),
                });
                cx.notify();
            },
            cx,
        );
    }

    /// Defines the asked name once the kernel takes it (TABLE-06).
    pub(crate) fn define(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Define {
            tab,
            name,
            range,
            scope,
        }) = &self.dialog
        else {
            return;
        };
        let (tab, scope) = (*tab, scope.clone());
        let name = name.read(cx).text().trim().to_string();
        let range = range.read(cx).text().trim().to_string();
        let asked = name.clone();
        self.read_in(
            tab,
            move |worker, doc| {
                let names = sheet::names(&worker.values(doc, sheet::NAMES)?)?;
                let check =
                    sheet::name_check(&worker.values(doc, &sheet::name_check_query(&asked))?)?;
                Ok((names, check))
            },
            move |book, read, _, cx| {
                let (names, check) = match read {
                    Ok(read) => read,
                    Err(error) => return book.engine_failed(&error.to_string(), cx),
                };
                let lower = name.to_lowercase();
                let taken = names
                    .iter()
                    .any(|known| known.scope == scope && known.name.to_lowercase() == lower);
                let refused = match check {
                    _ if taken => Some("table.names.taken"),
                    "valid" => None,
                    "address" => Some("table.names.address"),
                    _ => Some("table.names.characters"),
                };
                if let Some(key) = refused {
                    let text = Book::text(cx, key, &[("name", &name)]);
                    return book.toast(text, Severity::Warning, cx);
                }
                book.dialog = None;
                book.run_in(Some(tab), "names.define", &[&name, &range, &scope], cx);
            },
            cx,
        );
    }

    /// Lists the names to change, jump to or remove.
    fn ask_names(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.core.active() else {
            let text = Book::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        self.names_in(id, cx);
    }

    /// The names dialog for tab `id`, once its input lands.
    pub(crate) fn names_in(&mut self, id: u64, cx: &mut Context<Self>) {
        if !self.settled(id, cx) {
            return self.when_settled(id, move |book, _, cx| book.names_in(id, cx));
        }
        self.ask_in(
            id,
            |worker, doc| sheet::names(&worker.values(doc, sheet::NAMES)?),
            move |book, read, window, cx| {
                let names = match read {
                    Ok(names) => names,
                    Err(error) => return book.engine_failed(&error.to_string(), cx),
                };
                let Some(first) = names.first() else {
                    let text = Book::text(cx, "table.names.none", &[]);
                    return book.toast(text, Severity::Info, cx);
                };
                let range = cx.new(|cx| TextInput::new(window, cx));
                let formula = first.formula.clone();
                range.update(cx, |range, cx| range.set_text(formula, cx));
                book.open_dialog(Dialog::Names {
                    tab: id,
                    names,
                    chosen: 0,
                    range,
                });
                cx.notify();
            },
            cx,
        );
    }

    /// Picks name `index` in the open list.
    pub(crate) fn choose_name(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(Dialog::Names {
            names,
            chosen,
            range,
            ..
        }) = &mut self.dialog
        {
            *chosen = index;
            let formula = names[index].formula.clone();
            range.update(cx, |range, cx| range.set_text(formula, cx));
        }
        cx.notify();
    }

    /// Applies, jumps to or removes the chosen name.
    pub(crate) fn act_on_name(&mut self, act: NameAct, cx: &mut Context<Self>) {
        let Some(Dialog::Names {
            tab,
            names,
            chosen,
            range,
        }) = self.dialog.take()
        else {
            return;
        };
        let name = &names[chosen];
        let tab = Some(tab);
        match act {
            NameAct::Apply => {
                let range = range.read(cx).text().trim().to_string();
                self.run_in(tab, "names.define", &[&name.name, &range, &name.scope], cx);
            }
            NameAct::Go => {
                // The jump shows its own workbook.
                self.core.set_active(tab);
                self.run_in(tab, "cell.goto", &[&name.range], cx);
            }
            NameAct::Remove => self.run_in(tab, "names.remove", &[&name.name, &name.scope], cx),
        }
        cx.notify();
    }
}

/// What the names list does with the chosen name.
#[derive(Clone, Copy)]
pub enum NameAct {
    Apply,
    Go,
    Remove,
}
