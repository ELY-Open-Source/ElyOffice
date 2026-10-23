//! The canvas's right-click menus, laid out as PowerPoint's (SHOW-03).

use ely_gpui_component::{
    menus::{ContextMenu, Menu, MenuItem},
    primitives::Severity,
};
use ely_lok::{
    LokView,
    menu::{Runs, Target, runs},
    show::LAYOUTS,
};
use ely_office::menu::{self, Entry};
use ely_ui::menu::Offer;
use ely_office::prelude::*;
use gpui::{App, Context, Entity, Window};

use crate::deck::Deck;

/// What was right-clicked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// The slide, nothing selected.
    Slide,
    /// Text being edited.
    Text,
    /// A shape or a text box.
    Object,
    Picture,
    Table,
}

/// A menu's kind and what the kernel lets run.
pub struct What {
    pub kind: Kind,
    runs: Runs,
}

/// One thing a menu does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pick {
    Cut,
    Copy,
    Paste,
    /// An index into `LAYOUTS`.
    Layout(usize),
    Comment,
    Bullets,
    Numbering,
    Group,
    Ungroup,
    Front,
    Forward,
    Back,
    Backward,
    Replace,
    Crop,
    RowAbove,
    RowBelow,
    ColumnLeft,
    ColumnRight,
    DeleteRows,
    DeleteColumns,
    DeleteTable,
    Merge,
}

const CLIPBOARD: [Pick; 3] = [Pick::Cut, Pick::Copy, Pick::Paste];

const LAYOUT_PICKS: [Pick; 12] = [
    Pick::Layout(0),
    Pick::Layout(1),
    Pick::Layout(2),
    Pick::Layout(3),
    Pick::Layout(4),
    Pick::Layout(5),
    Pick::Layout(6),
    Pick::Layout(7),
    Pick::Layout(8),
    Pick::Layout(9),
    Pick::Layout(10),
    Pick::Layout(11),
];

const SLIDE: &[Entry<Pick>] = &[
    Entry::Line,
    Entry::Sub("show.design.layout", &LAYOUT_PICKS),
    Entry::Line,
    Entry::Row(Pick::Comment),
];

const TEXT: &[Entry<Pick>] = &[
    Entry::Line,
    Entry::Row(Pick::Bullets),
    Entry::Row(Pick::Numbering),
    Entry::Line,
    Entry::Row(Pick::Comment),
];

const OBJECT: &[Entry<Pick>] = &[
    Entry::Line,
    Entry::Sub("show.object.grouping", &[Pick::Group, Pick::Ungroup]),
    Entry::Sub("office.menu.front", &[Pick::Front, Pick::Forward]),
    Entry::Sub("office.menu.back", &[Pick::Back, Pick::Backward]),
    Entry::Line,
    Entry::Row(Pick::Comment),
];

const PICTURE: &[Entry<Pick>] = &[
    Entry::Line,
    Entry::Row(Pick::Replace),
    Entry::Row(Pick::Crop),
    Entry::Line,
    Entry::Sub("show.object.grouping", &[Pick::Group, Pick::Ungroup]),
    Entry::Sub("office.menu.front", &[Pick::Front, Pick::Forward]),
    Entry::Sub("office.menu.back", &[Pick::Back, Pick::Backward]),
    Entry::Line,
    Entry::Row(Pick::Comment),
];

const TABLE: &[Entry<Pick>] = &[
    Entry::Line,
    Entry::Sub(
        "doc.menu.insert",
        &[Pick::RowAbove, Pick::RowBelow, Pick::ColumnLeft, Pick::ColumnRight],
    ),
    Entry::Sub("doc.menu.delete", &[Pick::DeleteRows, Pick::DeleteColumns, Pick::DeleteTable]),
    Entry::Row(Pick::Merge),
    Entry::Line,
    Entry::Row(Pick::Comment),
];

/// Each layout's label, in `LAYOUTS` order.
const LAYOUT_KEYS: [&str; 12] = [
    "show.layout.20",
    "show.layout.0",
    "show.layout.1",
    "show.layout.3",
    "show.layout.19",
    "show.layout.32",
    "show.layout.12",
    "show.layout.15",
    "show.layout.14",
    "show.layout.16",
    "show.layout.18",
    "show.layout.34",
];

impl menu::Pick for Pick {
    fn row(self) -> (&'static str, &'static str) {
        match self {
            Pick::Cut => ("office.menu.cut", ".uno:Cut"),
            Pick::Copy => ("office.menu.copy", ".uno:Copy"),
            Pick::Paste => ("office.menu.paste", ".uno:Paste"),
            Pick::Layout(index) => (LAYOUT_KEYS[index], ".uno:AssignLayout"),
            Pick::Comment => ("doc.menu.comment", ".uno:InsertAnnotation"),
            Pick::Bullets => ("doc.props.bullets", ".uno:DefaultBullet"),
            Pick::Numbering => ("doc.props.numbering", ".uno:DefaultNumbering"),
            Pick::Group => ("show.object.group", ".uno:FormatGroup"),
            Pick::Ungroup => ("show.object.ungroup", ".uno:FormatUngroup"),
            Pick::Front => ("office.menu.front", ".uno:BringToFront"),
            Pick::Forward => ("office.menu.forward", ".uno:ObjectForwardOne"),
            Pick::Back => ("office.menu.back", ".uno:SendToBack"),
            Pick::Backward => ("office.menu.backward", ".uno:ObjectBackOne"),
            Pick::Replace => ("show.object.replace", ".uno:ChangePicture"),
            Pick::Crop => ("show.object.crop", ".uno:Crop"),
            Pick::RowAbove => ("doc.menu.row_above", ".uno:InsertRowsBefore"),
            Pick::RowBelow => ("doc.menu.row_below", ".uno:InsertRowsAfter"),
            Pick::ColumnLeft => ("doc.menu.column_left", ".uno:InsertColumnsBefore"),
            Pick::ColumnRight => ("doc.menu.column_right", ".uno:InsertColumnsAfter"),
            Pick::DeleteRows => ("doc.menu.delete_rows", ".uno:DeleteRows"),
            Pick::DeleteColumns => ("doc.menu.delete_columns", ".uno:DeleteColumns"),
            Pick::DeleteTable => ("doc.menu.delete_table", ".uno:DeleteTable"),
            Pick::Merge => ("doc.menu.merge", ".uno:MergeCells"),
        }
    }
}

impl What {
    /// The kernel lets the pick's own command run now.
    pub(crate) fn on(&self, pick: Pick) -> bool {
        self.runs.allows(menu::Pick::row(pick).1)
    }
}

impl Kind {
    fn of(target: Target) -> Option<Kind> {
        match target {
            Target::Slide => Some(Kind::Slide),
            Target::Text => Some(Kind::Text),
            Target::Object => Some(Kind::Object),
            Target::Picture => Some(Kind::Picture),
            Target::Table => Some(Kind::Table),
            Target::Frame | Target::Cell => None,
        }
    }

    /// Its rows under the clipboard row.
    fn rows(self) -> &'static [Entry<Pick>] {
        match self {
            Kind::Slide => SLIDE,
            Kind::Text => TEXT,
            Kind::Object => OBJECT,
            Kind::Picture => PICTURE,
            Kind::Table => TABLE,
        }
    }

    /// Every command its rows run.
    fn commands(self) -> Vec<&'static str> {
        let rows = self.rows().iter().flat_map(|entry| match entry {
            Entry::Row(pick) => std::slice::from_ref(pick),
            Entry::Sub(_, picks) => picks,
            Entry::Line => &[],
        });
        let mut commands: Vec<_> = CLIPBOARD.iter().chain(rows).map(|pick| menu::Pick::row(*pick).1).collect();
        commands.sort_unstable();
        commands.dedup();
        commands
    }
}

impl Deck {
    /// Turns the kernel's context menu into Ely's.
    pub(crate) fn offer_menu(&mut self, id: u64, view: &Entity<LokView>, cx: &mut Context<Self>) {
        let (offered, at) = match menu::take(view, cx) {
            None => return,
            Some(Ok(taken)) => taken,
            Some(Err(error)) => return self.engine_failed(&error.to_string(), cx),
        };
        let Some(kind) = Kind::of(offered.target) else {
            let error = format!("a {:?} menu in a presentation", offered.target);
            return self.toast(Deck::text(cx, "office.error.engine", &[("error", &error)]), Severity::Danger, cx);
        };
        let commands = kind.commands();
        self.ask_in(
            id,
            move |worker, doc| runs(worker, doc, &commands),
            move |deck, read, _, cx| match read {
                Ok(runs) => {
                    deck.menu = Some(Offer::new(deck.menu.as_ref(), id, What { kind, runs }, at));
                    cx.notify();
                }
                Err(error) => deck.engine_failed(&error.to_string(), cx),
            },
            cx,
        );
    }

    /// The menu's host, for the active deck.
    pub(crate) fn context_menu(&self, cx: &mut Context<Self>) -> Option<ContextMenu> {
        let offer = self.menu.as_ref().filter(|offer| Some(offer.tab) == self.core.active)?;
        Some(offer.host("show-menu", menu_of(&cx.entity(), &offer.what, cx)))
    }

    /// Runs one menu row.
    pub(crate) fn choose(&mut self, pick: Pick, window: &mut Window, cx: &mut Context<Self>) {
        log::info!("show: the menu picks {pick:?}");
        let id = match pick {
            Pick::Cut | Pick::Copy | Pick::Paste => return self.clipboard(pick, cx),
            Pick::Comment => return self.ask_remark(None, window, cx),
            Pick::Replace => return self.pick_picture(true, cx),
            Pick::Layout(index) => {
                return self.design_command("slide.layout", &[&LAYOUTS[index].to_string()], cx);
            }
            Pick::Bullets => "list.bullets",
            Pick::Numbering => "list.numbering",
            Pick::Group => "object.group",
            Pick::Ungroup => "object.ungroup",
            Pick::Front => "image.front",
            Pick::Forward => "image.forward",
            Pick::Back => "image.back",
            Pick::Backward => "image.backward",
            Pick::Crop => "image.crop",
            Pick::RowAbove => "table.row_above",
            Pick::RowBelow => "table.row_below",
            Pick::ColumnLeft => "table.column_left",
            Pick::ColumnRight => "table.column_right",
            Pick::DeleteRows => "table.delete_rows",
            Pick::DeleteColumns => "table.delete_columns",
            Pick::DeleteTable => "table.delete",
            Pick::Merge => "table.merge",
        };
        self.object_command(id, &[], cx);
    }

    fn clipboard(&mut self, pick: Pick, cx: &mut Context<Self>) {
        let Some(view) = self.active_tab().map(|tab| tab.view.clone()) else {
            log::warn!("show: {pick:?} with no deck");
            return;
        };
        view.update(cx, |view, _| match pick {
            Pick::Paste => view.paste(),
            _ => view.copy(pick == Pick::Cut),
        });
    }
}

/// The clipboard row, then the target's rows.
pub(crate) fn menu_of(this: &Entity<Deck>, what: &What, cx: &App) -> Menu {
    let item = |pick: Pick| {
        let this = this.clone();
        MenuItem::new(Deck::text(cx, menu::Pick::row(pick).0, &[]))
            .disabled(!what.on(pick))
            .on_click(move |window, cx| this.update(cx, |deck, cx| deck.choose(pick, window, cx)))
    };
    let menu = menu::clipboard(Menu::new(), CLIPBOARD, item);
    menu::entries(menu, what.kind.rows(), cx, |pick| what.on(pick), item)
}
