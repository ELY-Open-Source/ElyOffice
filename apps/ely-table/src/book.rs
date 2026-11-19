//! ElyTable's window: workbooks over one LOK worker (§9.1).

use ely_gpui_component::secondary;
use std::sync::Arc;

use ely_gpui_component::{
    primitives::{IconName, Severity},
    shell::{StatusBar, StatusBarItem},
};
use ely_lok::{LokView, Sheets, sheet::Geometry};
use ely_office::{Core, Kind, prelude::*};
use ely_platform::AppId;
use ely_ui::{Root, Shell, commands};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, AnyView, App, AppContext as _, Context, Entity, InteractiveElement as _,
    IntoElement, KeyBinding, ParentElement as _, Render, SharedString, Styled as _, Window, div,
};

use crate::bars::Bars;
use crate::format::Panel;
use crate::grid::Drag;

/// A cell: sheet count, address, edit count.
pub type Spot = (u64, String, u64);

/// What ElyTable keeps per workbook.
#[derive(Default)]
pub struct Extra {
    pub sheets: Sheets,
    /// The document as the last read saw it.
    pub read_at: Option<ely_lok::Doc>,
    pub geometry: Geometry,
    /// The view's sheet and header counts at the last read.
    pub read: Option<(u64, u64)>,
    /// A sheet read is on its way.
    pub reading: bool,
    /// The kernel requests already taken.
    pub requests: u64,
    /// The cell the rule belongs to, and the rule.
    pub rule: Option<(Spot, ely_lok::sheet::Validation)>,
    /// The cell whose rule was last asked for.
    pub ruled: Option<Spot>,
    /// A rule read is on its way.
    pub ruling: bool,
    /// Cells showing the file's results: unknown functions.
    pub kept: Vec<String>,
    /// The edit count the links were last asked at.
    pub linked: Option<u64>,
    /// A links read is on its way.
    pub linking: bool,
}

pub struct Book {
    pub core: Core<Book>,
    pub dialog: Option<crate::dialog::Dialog>,
    pub bars: Bars,
    pub drag: Option<Drag>,
    /// The sheet menu's request: number and place.
    pub sheet_menu: Option<(u64, gpui::Point<gpui::Pixels>)>,
    /// The grid or header menu.
    pub menu: Option<ely_ui::menu::Offer<crate::menu::What>>,
    pub panel: Entity<Panel>,
    pub choices: Option<crate::rules::Choices>,
    /// Read text files waiting for the import dialog.
    pub imports: std::collections::VecDeque<crate::text::Pending>,
    /// Undo and Redo as the menus show them.
    pub undo_menu: (bool, bool),
    /// The R editor's code, kept between openings.
    pub r_code: Option<Entity<ely_gpui_component::forms::TextInput>>,
    /// Whether the user agreed to run the R code.
    pub r_consent: crate::macros::Consent,
}

impl Host for Book {
    type Extra = Extra;
    type Engine = ely_office::Lok;
    const KIND: Kind = Kind {
        app: AppId::Table,
        kept: &["xlsx", "ods", "xls", "fods"],
        save_as: &["xlsx", "ods"],
        macros: Some(&["xlsm", "xltm"]),
    };

    fn core(&self) -> &Core<Self> {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core<Self> {
        &mut self.core
    }

    fn read_at(&self, tab: u64, _: &gpui::App) -> Option<ely_lok::Doc> {
        self.tab(tab).and_then(|tab| tab.extra.read_at.clone())
    }

    fn observed(&mut self, id: u64, view: &Entity<LokView>, cx: &mut Context<Self>) {
        self.reread(id, view, cx);
        self.freeze(id, view, cx);
        self.found(id, view, cx);
        self.requested(id, view, cx);
        self.offer_menu(id, view, cx);
        self.follow_rule(id, view, cx);
        self.read_links(id, view, cx);
        if self.core.active() == Some(id) {
            self.sync_bars(view, cx);
        }
    }

    fn tab_shown(&mut self, id: u64, cx: &mut Context<Self>) {
        if let Some(view) = self.tab(id).map(|tab| tab.view.clone()) {
            self.reread(id, &view, cx);
        }
    }

    fn asks_about(&self, id: u64) -> bool {
        self.dialog.as_ref().and_then(crate::dialog::Dialog::tab) == Some(id) || self.choices.as_ref().is_some_and(|choices| choices.tab == id)
    }

    /// A closed tab's dialog and list go with it.
    fn tab_closed(&mut self, tab: &ely_office::Tab<Extra, ely_office::Lok>, cx: &mut Context<Self>) {
        if self.dialog.as_ref().and_then(crate::dialog::Dialog::tab) == Some(tab.id) {
            self.dialog = None;
        }
        if self.choices.as_ref().is_some_and(|choices| choices.tab == tab.id) {
            self.choices = None;
        }
        cx.notify();
    }
}

/// ElyTable's menus after File; `undo` greys Undo and Redo.
pub(crate) fn menus(undo: (bool, bool), cx: &App) -> Vec<gpui::Menu> {
    vec![
        gpui::Menu::new(ely_ui::i18n::text(cx, "menu.edit", &[])).items(ely_office::undo_items(undo, cx)),
        gpui::Menu::new(ely_ui::i18n::text(cx, "menu.view", &[]))
            .items(ely_office::zoom_items(false, cx).into_iter().chain([gpui::MenuItem::separator(), ely_ui::commands::right_panel_item(cx)])),
        crate::sheets::menu(cx),
        crate::cells::menu(cx),
        crate::analysis::menu(cx),
        crate::format::menu(cx),
        crate::formulas::menu(cx),
        crate::data::menu(cx),
        crate::page::menu(cx),
        crate::macros::menu(cx),
    ]
}

/// The window root with a blank workbook.
pub fn open(app: AppId, window: &mut Window, cx: &mut App) -> Entity<Root> {
    ely_lok::init(cx);
    let (worker, stop, work) = match ely_office::start(app, cx) {
        Ok(started) => started,
        Err(error) => ely_platform::startup::fail(app, &format!("{error}")),
    };
    let book = crate::window::book(|cx| Core::new(worker, stop, work, cx), window, cx);
    cx.bind_keys([
        KeyBinding::new(
            secondary!("f"),
            ely_office::FindReplace,
            Some(commands::CONTEXT),
        ),
        KeyBinding::new(
            secondary!("shift-v"),
            crate::data::PasteValues,
            Some(commands::CONTEXT),
        ),
    ]);
    ely_office::bind_zoom_keys(cx);
    ely_office::bind_undo_keys(cx);
    let root = crate::window::root(&book, window, cx);
    let launched = ely_ui::launch_files(cx);
    book.update(cx, |book, cx| match launched.is_empty() {
        true => book.new_blank(cx),
        false => launched.into_iter().for_each(|path| book.open_asked(path, cx)),
    });
    ely_ui::on_ask(cx, |ask, cx| ely_office::remote::route(ask, cx, Book::answer));
    // Files go to their window, else the one in front.
    ely_ui::on_open(cx, |path, window, cx| match crate::window::front(window, cx) {
        Some(front) => front.update(cx, |book, cx| book.open_asked(path, cx)),
        None => log::error!("table: a file came with no window open"),
    });
    root
}

impl Book {
    pub fn new(core: Core<Self>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let book = cx.weak_entity();
        let panel = cx.new(|cx| Panel::new(book, Arc::clone(&core.worker), window, cx));
        // A closed dialog lets the next import ask.
        cx.observe_self(|book, cx| book.next_import(cx)).detach();
        // The window that comes to the front shows its menus.
        cx.observe_global::<ely_ui::tray::Front>(|book: &mut Book, cx| {
            if ely_ui::tray::main(cx) == book.core.window {
                let able = book.active_tab().map_or((false, false), |tab| ely_office::undo_able(tab.view.read(cx)));
                book.install_menus(able, cx);
            }
        })
        .detach();
        Book {
            core,
            dialog: None,
            bars: Bars::new(window, cx),
            drag: None,
            sheet_menu: None,
            menu: None,
            panel,
            choices: None,
            imports: Default::default(),
            undo_menu: (false, false),
            r_code: None,
            r_consent: crate::macros::Consent::First,
        }
    }

    /// Greys Undo and Redo as the front workbook's kernel says.
    fn sync_menus(&mut self, cx: &mut Context<Self>) {
        // The app has one menu: the front window's.
        if ely_ui::tray::main(cx) != self.core.window {
            return;
        }
        let able = self.active_tab().map_or((false, false), |tab| ely_office::undo_able(tab.view.read(cx)));
        if able != self.undo_menu {
            self.install_menus(able, cx);
        }
    }

    pub fn text(cx: &App, key: &str, args: &[(&str, &str)]) -> SharedString {
        ely_ui::i18n::text(cx, key, args)
    }

    /// A new untitled workbook (TABLE-01).
    pub fn new_blank(&mut self, cx: &mut Context<Self>) {
        self.new_from_text("blank.fods", &crate::template::blank(cx), cx);
    }

    /// Rereads sheets and geometry when the kernel moves them.
    fn reread(&mut self, id: u64, view: &Entity<LokView>, cx: &mut Context<Self>) {
        let cell = view.read(cx).cell();
        let seen = (cell.sheets, cell.headers);
        let Some(tab) = self.tab(id) else {
            return;
        };
        // Numbered commands need the latest read; hidden books wait.
        let behind = self.core.active() == Some(id)
            && tab.extra.read_at.as_ref().is_some_and(|at| view.read(cx).read_at().newer_than(at));
        if tab.extra.reading || tab.extra.read == Some(seen) && !behind {
            return;
        }
        let doc = tab.doc.clone();
        // Marked read now: a failure waits for the next change.
        let extra = &mut self.tab_mut(id).expect("the tab is open").extra;
        (extra.reading, extra.read) = (true, Some(seen));
        self.read_for_tab(
            id,
            &doc,
            |worker, doc| {
                let at = worker.now(doc)?;
                let sheets = doc.sheets(worker)?;
                let json = worker.values(doc, ely_lok::sheet::GEOMETRY)?;
                Ok((sheets, ely_lok::sheet::geometry(&json)?, at))
            },
            move |book: &mut Self, read, cx| {
                let Some(tab) = book.tab_mut(id) else {
                    return;
                };
                tab.extra.reading = false;
                let view = tab.view.clone();
                match read {
                    Ok((sheets, geometry, at)) => {
                        log::info!("table: tab {id} has {} sheets", sheets.list.len());
                        view.update(cx, |view, cx| {
                            view.set_part(sheets.current);
                            view.set_geometry(geometry.clone(), cx);
                        });
                        let extra = &mut tab.extra;
                        extra.sheets = sheets;
                        extra.geometry = geometry;
                        extra.read_at = Some(at);
                    }
                    Err(error) => book.engine_failed(&error.to_string(), cx),
                }
                cx.notify();
                // The kernel may have moved again meanwhile.
                book.reread(id, &view, cx);
            },
            cx,
        );
    }

    /// Rereads linked sources and kept results after each edit.
    fn read_links(&mut self, id: u64, view: &Entity<LokView>, cx: &mut Context<Self>) {
        let edited = view.read(cx).edited();
        let Some(tab) = self.tab_mut(id) else {
            return;
        };
        if tab.extra.linking || tab.extra.linked == Some(edited) {
            return;
        }
        (tab.extra.linking, tab.extra.linked) = (true, Some(edited));
        let doc = tab.doc.clone();
        self.read_for_tab(
            id,
            &doc,
            |worker, doc| {
                let links = ely_lok::links::sources(&worker.values(doc, ely_lok::links::LIST)?)?;
                Ok((links, ely_lok::sheet::kept(&worker.values(doc, ely_lok::sheet::KEPT)?)?))
            },
            move |book: &mut Self, read, cx| {
                let Some(tab) = book.tab_mut(id) else {
                    return;
                };
                tab.extra.linking = false;
                let view = tab.view.clone();
                match read {
                    Ok((links, kept)) => {
                        log::info!("table: tab {id} links {} sources, {} kept results", links.len(), kept.len());
                        tab.links.named(links);
                        tab.extra.kept = kept;
                    }
                    Err(error) => book.engine_failed(&error.to_string(), cx),
                }
                cx.notify();
                // An edit may have landed meanwhile.
                book.read_links(id, &view, cx);
            },
            cx,
        );
    }

    /// Results Calc cannot compute stay as saved (§9.3, D-02).
    fn kept_item(&self, cx: &mut Context<Self>) -> Option<StatusBarItem> {
        let count = self.active_tab()?.extra.kept.len();
        if count == 0 {
            return None;
        }
        let label = Self::text(cx, "table.kept.count", &[("count", &count.to_string())]);
        let this = cx.entity();
        let item = StatusBarItem::new("table-kept").icon(IconName::Info).label(label);
        Some(item.on_click(move |_, _, cx| this.update(cx, |book, cx| book.list_kept(cx))))
    }

    /// The kept cells, in a toast.
    pub fn list_kept(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return self.toast(Self::text(cx, "command.blocked.no_document", &[]), Severity::Warning, cx);
        };
        let text = match tab.extra.kept.is_empty() {
            true => Self::text(cx, "table.kept.none", &[]),
            false => Self::text(cx, "table.kept.stale", &[("cells", &listed(&tab.extra.kept))]),
        };
        log::info!("table: lists {} kept results", tab.extra.kept.len());
        self.toast_unlogged(text, Severity::Info, cx);
    }

    /// Holds the kernel's frozen rows and columns in place.
    fn freeze(&mut self, id: u64, view: &Entity<LokView>, cx: &mut Context<Self>) {
        let Some(tab) = self.tab(id) else {
            return;
        };
        let read = view.read(cx);
        let index = |uno: &str| frozen_index(read.state(uno));
        let geometry = &tab.extra.geometry;
        let frozen = (
            index(".uno:FreezePanesColumn").map_or(0, |at| geometry.columns.start(at)),
            index(".uno:FreezePanesRow").map_or(0, |at| geometry.rows.start(at)),
        );
        view.update(cx, |view, cx| view.set_frozen(frozen, cx));
    }

    /// The save state, and the kernel's statistics of the selection.
    fn status(&self, cx: &mut Context<Self>) -> StatusBar {
        let mut status = StatusBar::new().left(self.state_item("table-state", cx));
        status = status.when_some(self.links_item(cx), |status, item| status.left(item));
        status = status.when_some(self.kept_item(cx), |status, item| status.left(item));
        if self.automatic(cx) == Some(false) {
            let manual = Self::text(cx, "table.calc.manual", &[]);
            status = status.left(StatusBarItem::new("table-manual").label(manual));
        }
        let figures = self
            .active_tab()
            .and_then(|tab| tab.view.read(cx).state(".uno:StateTableCell"))
            .filter(|figures| !figures.is_empty())
            .map(|figures| crate::figures::label(figures, cx));
        let status = match figures {
            Some(figures) => status.right(StatusBarItem::new("table-figures").label(figures)),
            None => status,
        };
        status.when_some(self.zoom_control(cx), |status, zoom| status.right(zoom))
    }

    fn body(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if self.active_tab().is_none() {
            return self.empty_body("table-empty", IconName::Table, cx);
        }
        if let Some(page) = self.stopped_page(cx) {
            return page;
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(self.bars.render(cx))
            .child(self.grid(window, cx))
            .child(self.sheet_strip(cx))
            .into_any_element()
    }
}

/// The first five cells, then how many in all.
pub(crate) fn listed(cells: &[String]) -> String {
    let mut listed = cells.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
    if cells.len() > 5 {
        listed.push_str(&format!(", … ({})", cells.len()));
    }
    listed
}

impl Render for Book {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Another workbook may have become active.
        self.refill(cx);
        self.sync_menus(cx);
        let title = self
            .active_tab()
            .map_or_else(|| AppId::Table.name().into(), |tab| tab.name.clone());
        let shell = Shell::new(title, &self.core.focus)
            .tabs(self.tab_strip(cx))
            .right(self.side_view(cx))
            .status(self.status(cx))
            .child(self.body(window, cx));
        let root = div()
            .size_full()
            .key_context(commands::CONTEXT)
            .track_focus(&self.core.focus)
            .on_action(cx.listener(|book, _: &commands::New, _, cx| book.new_blank(cx)))
            .on_action(cx.listener(Self::on_open_file))
            .on_action(cx.listener(Self::on_r_macro))
            .on_action(cx.listener(Self::on_save))
            .on_action(cx.listener(Self::on_save_as))
            .on_action(cx.listener(Self::on_close_tab))
            .on_action(cx.listener(Self::on_export))
            .on_action(cx.listener(Self::on_print))
            .child(shell)
            .children(self.core_dialog(cx))
            .children(self.dialog_element(cx))
            .children(self.context_menu(cx))
            .children(self.tab_menu(cx));
        let root = Self::bind_zoom(crate::cells::bind(crate::sheets::bind(root, cx), cx), cx);
        let root = Self::bind_undo(root, cx);
        let root = crate::data::bind(crate::formulas::bind(root, cx), cx);
        let root = crate::page::bind(crate::protect::bind(root, cx), cx);
        crate::analysis::bind(crate::rules::bind(root, cx), cx)
            .on_action(cx.listener(Self::on_find))
            .on_action(cx.listener(Self::on_update_links))
            .on_action(cx.listener(Self::on_show_risk))
    }
}

impl Book {
    /// The format panel for the active workbook, while shown.
    fn side_view(&mut self, cx: &mut Context<Self>) -> Option<AnyView> {
        let shown = self
            .active_tab()
            .map(|tab| (tab.view.clone(), tab.doc.clone()));
        let tab = shown.as_ref().map(|(view, doc)| (view, doc));
        self.panel.update(cx, |panel, cx| panel.show(tab, cx));
        shown.is_some().then(|| self.panel.clone().into())
    }
}

/// The first unfrozen index; None for none.
fn frozen_index(state: Option<&str>) -> Option<u32> {
    let index: u32 = state?.split('/').next()?.trim().parse().ok()?;
    (index > 0).then_some(index)
}

impl Zooming for Book {}

#[cfg(test)]
mod tests {
    #[test]
    fn freeze_states_read_their_index() {
        assert_eq!(super::frozen_index(Some("2 / 0")), Some(2));
        assert_eq!(super::frozen_index(Some("0 / 1")), None);
        assert_eq!(super::frozen_index(None), None);
    }
}
