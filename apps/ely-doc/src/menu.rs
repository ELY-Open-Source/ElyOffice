//! The right-click menu, laid out as Word's (DOC-02, DOC-07, DOC-12).

use ely_gpui_component::{
    menus::{ContextMenu, Menu, MenuItem},
    primitives::Severity,
};
use ely_lok::{
    LokView,
    menu::{Offered, Target},
};
use ely_office::menu::{self, Entry};
use ely_ui::menu::Offer;
use gpui::{App, Context, Entity, Window};

use crate::docs::Docs;
use crate::refs::{InsertCaption, InsertComment, InsertLink};
use ely_office::prelude::*;

/// One thing the menu does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pick {
    Cut,
    Copy,
    Paste,
    Link,
    Comment,
    ColumnLeft,
    ColumnRight,
    RowAbove,
    RowBelow,
    DeleteRows,
    DeleteColumns,
    DeleteTable,
    Merge,
    SplitColumns,
    SplitRows,
    EvenRows,
    EvenColumns,
    Inline,
    Square,
    TopBottom,
    InFront,
    Front,
    Forward,
    Back,
    Backward,
    Caption,
}

const TABLE: &[Entry<Pick>] = &[
    Entry::Line,
    Entry::Sub(
        "doc.menu.insert",
        &[Pick::ColumnLeft, Pick::ColumnRight, Pick::RowAbove, Pick::RowBelow],
    ),
    Entry::Sub(
        "doc.menu.delete",
        &[Pick::DeleteRows, Pick::DeleteColumns, Pick::DeleteTable],
    ),
    Entry::Row(Pick::Merge),
    Entry::Sub("doc.menu.split", &[Pick::SplitColumns, Pick::SplitRows]),
    Entry::Row(Pick::EvenRows),
    Entry::Row(Pick::EvenColumns),
    Entry::Line,
    Entry::Row(Pick::Link),
    Entry::Row(Pick::Comment),
];

const FRAME: &[Entry<Pick>] = &[
    Entry::Line,
    Entry::Sub(
        "doc.menu.wrap",
        &[Pick::Inline, Pick::Square, Pick::TopBottom, Pick::InFront],
    ),
    Entry::Sub("office.menu.front", &[Pick::Front, Pick::Forward]),
    Entry::Sub("office.menu.back", &[Pick::Back, Pick::Backward]),
    Entry::Line,
    Entry::Row(Pick::Caption),
    Entry::Row(Pick::Comment),
];

const TEXT: &[Entry<Pick>] = &[Entry::Line, Entry::Row(Pick::Link), Entry::Row(Pick::Comment)];

impl menu::Pick for Pick {
    fn row(self) -> (&'static str, &'static str) {
        match self {
            Pick::Cut => ("office.menu.cut", ".uno:Cut"),
            Pick::Copy => ("office.menu.copy", ".uno:Copy"),
            Pick::Paste => ("office.menu.paste", ".uno:Paste"),
            Pick::Link => ("doc.menu.link", ".uno:InsertHyperlink"),
            Pick::Comment => ("doc.menu.comment", ".uno:InsertAnnotation"),
            Pick::ColumnLeft => ("doc.menu.column_left", ".uno:InsertColumnsBefore"),
            Pick::ColumnRight => ("doc.menu.column_right", ".uno:InsertColumnsAfter"),
            Pick::RowAbove => ("doc.menu.row_above", ".uno:InsertRowsBefore"),
            Pick::RowBelow => ("doc.menu.row_below", ".uno:InsertRowsAfter"),
            Pick::DeleteRows => ("doc.menu.delete_rows", ".uno:DeleteRows"),
            Pick::DeleteColumns => ("doc.menu.delete_columns", ".uno:DeleteColumns"),
            Pick::DeleteTable => ("doc.menu.delete_table", ".uno:DeleteTable"),
            Pick::Merge => ("doc.menu.merge", ".uno:MergeCells"),
            Pick::SplitColumns => ("doc.menu.split_columns", ".uno:SplitCell"),
            Pick::SplitRows => ("doc.menu.split_rows", ".uno:SplitCell"),
            Pick::EvenRows => ("doc.menu.even_rows", ".uno:DistributeRows"),
            Pick::EvenColumns => ("doc.menu.even_columns", ".uno:DistributeColumns"),
            Pick::Inline => ("doc.menu.inline", ".uno:SetAnchorToChar"),
            Pick::Square => ("doc.menu.square", ".uno:WrapOn"),
            Pick::TopBottom => ("doc.menu.top_bottom", ".uno:WrapOff"),
            Pick::InFront => ("doc.menu.in_front", ".uno:WrapThrough"),
            Pick::Front => ("office.menu.front", ".uno:BringToFront"),
            Pick::Forward => ("office.menu.forward", ".uno:ObjectForwardOne"),
            Pick::Back => ("office.menu.back", ".uno:SendToBack"),
            Pick::Backward => ("office.menu.backward", ".uno:ObjectBackOne"),
            Pick::Caption => ("doc.menu.caption", ".uno:InsertCaptionDialog"),
        }
    }
}

impl Pick {
    /// The kernel offers it and it runs now.
    pub(crate) fn on(self, offered: &Offered) -> bool {
        offered.allows(menu::Pick::row(self).1)
    }
}

impl Docs {
    /// Turns the kernel's context menu into Ely's.
    pub(crate) fn offer_menu(&mut self, id: u64, view: &Entity<LokView>, cx: &mut Context<Self>) {
        let (offered, at) = match menu::take(view, cx) {
            None => return,
            Some(Ok(taken)) => taken,
            Some(Err(error)) => {
                let text = Docs::text(cx, "office.error.engine", &[("error", &error.to_string())]);
                return self.toast(text, Severity::Danger, cx);
            }
        };
        if matches!(offered.target, Target::Cell | Target::Object | Target::Picture | Target::Slide) {
            let error = format!("a {:?} menu in a text document", offered.target);
            let text = Docs::text(cx, "office.error.engine", &[("error", &error)]);
            return self.toast(text, Severity::Danger, cx);
        }
        let suggestions = offered.spelling.as_ref().map_or(0, |spelling| spelling.suggestions.len());
        log::info!("doc: {suggestions} suggestions");
        self.offer = Some(Offer::new(self.offer.as_ref(), id, offered, at));
        cx.notify();
    }

    /// The menu's host, for the active tab.
    pub(crate) fn context_menu(&self, cx: &mut Context<Self>) -> Option<ContextMenu> {
        let offer = self
            .offer
            .as_ref()
            .filter(|offer| Some(offer.tab) == self.core.active)?;
        Some(offer.host("doc-menu", menu_of(&cx.entity(), &offer.what, cx)))
    }

    /// Runs one menu row.
    pub(crate) fn choose(&mut self, pick: Pick, window: &mut Window, cx: &mut Context<Self>) {
        log::info!("doc: the menu picks {pick:?}");
        let (id, values): (&str, &[&str]) = match pick {
            Pick::Cut | Pick::Copy | Pick::Paste => return self.clipboard(pick, cx),
            Pick::Link => return self.on_link(&InsertLink, window, cx),
            Pick::Comment => return self.on_comment(&InsertComment, window, cx),
            Pick::Caption => return self.on_caption(&InsertCaption, window, cx),
            Pick::ColumnLeft => ("table.column_left", &[]),
            Pick::ColumnRight => ("table.column_right", &[]),
            Pick::RowAbove => ("table.row_above", &[]),
            Pick::RowBelow => ("table.row_below", &[]),
            Pick::DeleteRows => ("table.delete_rows", &[]),
            Pick::DeleteColumns => ("table.delete_columns", &[]),
            Pick::DeleteTable => ("table.delete", &[]),
            Pick::Merge => ("table.merge", &[]),
            Pick::SplitColumns => ("table.split", &["2", "false"]),
            Pick::SplitRows => ("table.split", &["2", "true"]),
            Pick::EvenRows => ("table.even_rows", &[]),
            Pick::EvenColumns => ("table.even_columns", &[]),
            Pick::Inline => ("image.anchor_as_char", &[]),
            Pick::Square => ("image.wrap_on", &[]),
            Pick::TopBottom => ("image.wrap_off", &[]),
            Pick::InFront => ("image.wrap_through", &[]),
            Pick::Front => ("image.front", &[]),
            Pick::Forward => ("image.forward", &[]),
            Pick::Back => ("image.back", &[]),
            Pick::Backward => ("image.backward", &[]),
        };
        self.run_active(id, values, cx);
    }

    fn clipboard(&mut self, pick: Pick, cx: &mut Context<Self>) {
        let Some(view) = self.active_tab().map(|tab| tab.view.clone()) else {
            log::warn!("doc: {pick:?} with no document");
            return;
        };
        view.update(cx, |view, _| match pick {
            Pick::Paste => view.paste(),
            _ => view.copy(pick == Pick::Cut),
        });
    }
}

/// Spelling first, then the clipboard row, then the target's rows.
pub(crate) fn menu_of(this: &Entity<Docs>, offered: &Offered, cx: &App) -> Menu {
    let item = |pick: Pick| {
        let this = this.clone();
        MenuItem::new(Docs::text(cx, menu::Pick::row(pick).0, &[]))
            .disabled(!pick.on(offered))
            .on_click(move |window, cx| this.update(cx, |docs, cx| docs.choose(pick, window, cx)))
    };
    let mut menu = Menu::new();
    if let Some(spelling) = &offered.spelling {
        for (label, rule) in &spelling.suggestions {
            let (this, rule) = (this.clone(), rule.clone());
            menu = menu.item(MenuItem::new(label.clone()).on_click(move |_, cx| {
                this.update(cx, |docs, cx| docs.run_active("spell.apply", &[&rule], cx))
            }));
        }
        if spelling.suggestions.is_empty() {
            menu = menu.item(MenuItem::new(Docs::text(cx, "doc.spell.none", &[])).disabled(true));
        }
        let this = this.clone();
        let ignore = MenuItem::new(Docs::text(cx, "doc.spell.ignore_all", &[])).on_click(
            move |_, cx| {
                this.update(cx, |docs, cx| {
                    docs.run_active("spell.ignore_all", &["Spelling"], cx)
                })
            },
        );
        menu = menu.item(ignore).separator();
    }
    let menu = menu::clipboard(menu, [Pick::Cut, Pick::Copy, Pick::Paste], item);
    let rows = match offered.target {
        Target::Table => TABLE,
        Target::Frame => FRAME,
        Target::Text => TEXT,
        Target::Cell | Target::Object | Target::Picture | Target::Slide => unreachable!("offer_menu refuses them"),
    };
    menu::entries(menu, rows, cx, |pick| pick.on(offered), item)
}

#[cfg(test)]
mod tests;
