//! Sheet tabs and the workbook's sheet commands (TABLE-01).

use std::io;

use ely_gpui_component::{
    buttons::IconButton,
    forms::{Choice, TextInput},
    menus::{ContextMenu, Menu as Popup, MenuItem as PopupItem},
    navigation::{TabPlacement, Tabs as SheetTabs},
    primitives::{IconName, Severity},
};
use ely_office::prelude::*;
use gpui::{
    Action, AnyElement, App, AppContext as _, Context, Div, InteractiveElement as _, IntoElement,
    Menu, MenuItem, ParentElement as _, Styled as _, Window, actions, div,
};

use crate::book::Book;
use crate::dialog::Dialog;

actions!(
    table,
    [
        InsertSheet,
        RenameSheet,
        DuplicateSheet,
        MoveSheetLeft,
        MoveSheetRight,
        HideSheet,
        ShowSheet,
        DeleteSheet
    ]
);

pub fn menu(cx: &App) -> Menu {
    let label = |key: &str| Book::text(cx, key, &[]);
    Menu::new(label("menu.sheet")).items([
        MenuItem::action(label("table.sheet.insert"), InsertSheet),
        MenuItem::action(label("table.sheet.rename"), RenameSheet),
        MenuItem::action(label("table.sheet.duplicate"), DuplicateSheet),
        MenuItem::separator(),
        MenuItem::action(label("table.sheet.left"), MoveSheetLeft),
        MenuItem::action(label("table.sheet.right"), MoveSheetRight),
        MenuItem::separator(),
        MenuItem::action(label("table.sheet.hide"), HideSheet),
        MenuItem::action(label("table.sheet.show"), ShowSheet),
        MenuItem::separator(),
        MenuItem::action(label("table.protect.menu"), crate::protect::ProtectSheet),
        MenuItem::separator(),
        MenuItem::action(label("table.sheet.delete"), DeleteSheet),
    ])
}

/// The same commands on a right click.
fn popup(cx: &App) -> Popup {
    let item = |key: &str, action: Box<dyn Action>| {
        PopupItem::new(Book::text(cx, key, &[]))
            .on_click(move |window, cx| window.dispatch_action(action.boxed_clone(), cx))
    };
    Popup::new()
        .item(item("table.sheet.insert", Box::new(InsertSheet)))
        .item(item("table.sheet.rename", Box::new(RenameSheet)))
        .item(item("table.sheet.duplicate", Box::new(DuplicateSheet)))
        .separator()
        .item(item("table.sheet.left", Box::new(MoveSheetLeft)))
        .item(item("table.sheet.right", Box::new(MoveSheetRight)))
        .separator()
        .item(item("table.sheet.hide", Box::new(HideSheet)))
        .item(item("table.sheet.show", Box::new(ShowSheet)))
        .separator()
        .item(item("table.sheet.delete", Box::new(DeleteSheet)))
}

pub(crate) mod dialogs;

pub fn bind(root: Div, cx: &mut Context<Book>) -> Div {
    root.on_action(cx.listener(|book, _: &InsertSheet, _, cx| book.insert_sheet(cx)))
        .on_action(cx.listener(|book, _: &RenameSheet, window, cx| book.ask_rename(window, cx)))
        .on_action(cx.listener(|book, _: &DuplicateSheet, _, cx| book.duplicate_sheet(cx)))
        .on_action(cx.listener(|book, _: &MoveSheetLeft, _, cx| book.move_sheet(false, cx)))
        .on_action(cx.listener(|book, _: &MoveSheetRight, _, cx| book.move_sheet(true, cx)))
        .on_action(cx.listener(|book, _: &HideSheet, _, cx| book.hide_sheet(cx)))
        .on_action(cx.listener(|book, _: &ShowSheet, _, cx| book.ask_show(cx)))
        .on_action(
            cx.listener(|book, _: &DeleteSheet, _, cx| book.ask_delete(book.core.active(), cx)),
        )
}

impl Book {
    /// The shown sheet, its name, and every name.
    fn current_sheet(&self) -> Option<(usize, String, Vec<String>)> {
        self.sheet_in(self.core.active()?)
    }

    /// Tab `id`: shown sheet, its name, every name.
    pub(crate) fn sheet_in(&self, id: u64) -> Option<(usize, String, Vec<String>)> {
        let sheets = &self.tab(id)?.extra.sheets;
        let names: Vec<String> = sheets.list.iter().map(|sheet| sheet.name.clone()).collect();
        Some((sheets.current, names.get(sheets.current)?.clone(), names))
    }

    fn warn(&self, key: &str, cx: &mut Context<Self>) {
        let text = Book::text(cx, key, &[]);
        self.toast(text, Severity::Warning, cx);
    }

    /// The visible sheets as tabs, with an add button.
    pub fn sheet_strip(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(tab) = self.active_tab() else {
            return div().into_any_element();
        };
        let Some(tabs) = strip(tab.id, &tab.extra.sheets) else {
            return div().into_any_element();
        };
        let this = cx.entity();
        let picked = this.clone();
        let tabs = tabs.on_change(move |value, _, cx| {
            let index: usize = value.parse().expect("sheet tabs are indexes");
            picked.update(cx, |book, cx| book.show_sheet(index, cx));
        })
        .on_context(move |value, at, _, cx| {
            let index: usize = value.parse().expect("sheet tabs are indexes");
            this.update(cx, |book, cx| {
                book.show_sheet(index, cx);
                book.sheet_menu = Some((book.sheet_menu.map_or(0, |(n, _)| n + 1), at));
                cx.notify();
            });
        });
        let add = IconButton::new("table-sheet-add", IconName::Plus)
            .tooltip(Book::text(cx, "table.sheet.add", &[]))
            .on_click(cx.listener(|book, _, _, cx| book.insert_sheet(cx)));
        ContextMenu::new("table-sheet-menu", popup(cx))
            .manual(self.sheet_menu)
            .child(div().flex().items_center().px_2().child(tabs).child(add))
            .into_any_element()
    }

    /// Shows sheet `index` in order with the queued input.
    pub(crate) fn show_sheet(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let (id, view) = (tab.id, tab.view.clone());
        view.update(cx, |view, _| view.show_part(index));
        self.tab_mut(id)
            .expect("the tab is open")
            .extra
            .sheets
            .current = index;
        cx.notify();
    }

    fn insert_sheet(&mut self, cx: &mut Context<Self>) {
        let Some((current, _, names)) = self.current_sheet() else {
            return self.warn("command.blocked.no_document", cx);
        };
        let name = (1..)
            .map(|n| Book::text(cx, "table.sheet.default", &[("n", &n.to_string())]))
            .find(|name| !names.iter().any(|taken| taken == name.as_ref()))
            .expect("a free name");
        let at = (current + 2).to_string();
        self.run_active("sheet.insert", &[&name, &at], cx);
    }

    fn duplicate_sheet(&mut self, cx: &mut Context<Self>) {
        let Some((current, ..)) = self.current_sheet() else {
            return self.warn("command.blocked.no_document", cx);
        };
        self.run_active(
            "sheet.move",
            &[&(current + 2).to_string(), "true", "true"],
            cx,
        );
    }

    /// Moves the shown sheet; Index is its final place.
    fn move_sheet(&mut self, right: bool, cx: &mut Context<Self>) {
        let Some((current, _, names)) = self.current_sheet() else {
            return self.warn("command.blocked.no_document", cx);
        };
        if (right && current + 1 == names.len()) || (!right && current == 0) {
            return self.warn("table.sheet.edge", cx);
        }
        let at = if right { current + 2 } else { current };
        self.run_active("sheet.move", &[&at.to_string(), "false", "true"], cx);
    }

    fn visible_count(&self, id: u64) -> usize {
        self.tab(id).map_or(0, |tab| {
            tab.extra
                .sheets
                .list
                .iter()
                .filter(|sheet| sheet.visible)
                .count()
        })
    }

    fn hide_sheet(&mut self, cx: &mut Context<Self>) {
        let Some((current, ..)) = self.current_sheet() else {
            return self.warn("command.blocked.no_document", cx);
        };
        if self.core.active().is_none_or(|id| self.visible_count(id) < 2) {
            return self.warn("table.sheet.last", cx);
        }
        self.run_active("sheet.hide", &[&current.to_string()], cx);
    }

    fn ask_show(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return self.warn("command.blocked.no_document", cx);
        };
        let hidden: Vec<String> = tab
            .extra
            .sheets
            .list
            .iter()
            .filter(|sheet| !sheet.visible)
            .map(|sheet| sheet.name.clone())
            .collect();
        let Some(chosen) = hidden.first().cloned() else {
            return self.warn("table.show.none", cx);
        };
        let tab = tab.id;
        self.open_dialog(Dialog::Show {
            tab,
            hidden,
            chosen,
        });
        cx.notify();
    }

    pub(crate) fn ask_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(tab), Some((index, name, _))) = (self.core.active(), self.current_sheet()) else {
            return self.warn("command.blocked.no_document", cx);
        };
        // The list shown was read in this state.
        let Some(at) = self.read_at(tab, cx) else {
            return self.warn("command.blocked.no_document", cx);
        };
        let field = cx.new(|cx| TextInput::new(window, cx));
        field.update(cx, |field, cx| field.set_text(name, cx));
        self.open_dialog(Dialog::Rename { tab, at, index, field });
        cx.notify();
    }

    /// Asks before a delete, naming what refers (UX-07).
    pub(crate) fn ask_delete(&mut self, id: Option<u64>, cx: &mut Context<Self>) {
        let Some(id) = id else {
            return self.warn("command.blocked.no_document", cx);
        };
        if !self.settled(id, cx) {
            return self.when_settled(id, move |book, _, cx| book.ask_delete(Some(id), cx));
        }
        let Some((index, name, _)) = self.sheet_in(id) else {
            return self.warn("command.blocked.no_document", cx);
        };
        if self.visible_count(id) < 2 {
            return self.warn("table.sheet.last", cx);
        }
        self.ask_in(
            id,
            move |worker, doc| {
                // The sheet is checked and deleted in one state.
                let at = worker.now(doc)?;
                if doc.sheets(worker)?.list.get(index).map(|sheet| &sheet.name) != Some(&name) {
                    return Err(io::Error::other(format!("{}sheet {index} moved", ely_lok::STALE)));
                }
                let json = worker.values(doc, &ely_lok::sheet::references_query(index))?;
                Ok((ely_lok::sheet::references(&json)?, at, name))
            },
            move |book, read, _, cx| match read {
                Ok((references, at, name)) => {
                    book.open_dialog(Dialog::Delete {
                        tab: id,
                        at,
                        index,
                        name,
                        references,
                    });
                    cx.notify();
                }
                Err(error) if ely_lok::is_stale(&error) => book.warn("office.error.changed", cx),
                Err(error) => book.engine_failed(&error.to_string(), cx),
            },
            cx,
        );
    }

    /// Renames the sheet the open dialog asked about.
    pub(crate) fn rename_asked(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some(Dialog::Rename { tab, at, index, .. }) = self.dialog.take() else {
            return;
        };
        self.run_on(tab, &at, "sheet.rename", &[name, &(index + 1).to_string()], cx);
    }

    /// Deletes the sheet the open dialog asked about.
    pub(crate) fn delete_asked(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Delete { tab, at, index, .. }) = self.dialog.take() else {
            return;
        };
        self.run_on(tab, &at, "sheet.delete", &[&(index + 1).to_string()], cx);
    }
}

/// Workbook `id`'s sheet tabs; each workbook keeps its own scroll.
fn strip(id: u64, sheets: &ely_lok::Sheets) -> Option<SheetTabs> {
    let choices = visible(sheets)?;
    Some(
        SheetTabs::new(("table-sheets", id), choices, sheets.current.to_string())
            .placement(TabPlacement::Bottom),
    )
}

/// Visible sheets as tabs, once the shown one is known.
fn visible(sheets: &ely_lok::Sheets) -> Option<Vec<Choice>> {
    let shown = sheets.list.get(sheets.current)?;
    if !shown.visible {
        return None;
    }
    Some(
        sheets
            .list
            .iter()
            .enumerate()
            .filter(|(_, sheet)| sheet.visible)
            .map(|(index, sheet)| Choice::new(index.to_string(), sheet.name.clone()))
            .collect(),
    )
}

#[cfg(test)]
#[path = "tests/strip.rs"]
mod strip_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use ely_lok::{Sheet, Sheets};

    #[test]
    fn tabs_wait_for_a_known_shown_sheet() {
        assert!(visible(&Sheets::default()).is_none(), "no list yet");
        let sheet = |name: &str, visible| Sheet {
            name: name.into(),
            visible,
        };
        let hidden = Sheets {
            current: 1,
            list: vec![sheet("A", true), sheet("B", false)],
        };
        assert!(visible(&hidden).is_none(), "a hidden current waits");
        let shown = Sheets {
            current: 0,
            list: vec![sheet("A", true), sheet("B", false), sheet("C", true)],
        };
        let values: Vec<_> = visible(&shown)
            .expect("tabs")
            .into_iter()
            .map(|choice| choice.value)
            .collect();
        assert_eq!(values, ["0", "2"]);
    }
}
