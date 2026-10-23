//! ElyDoc's window: document tabs over one LOK worker (§8.1).

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use ely_lok::{LokError, LokView, Worker, review::Counts};
use ely_office::{Core, Kind, prelude::*};
use ely_platform::AppId;
use ely_ui::{Root, Shell, commands};
use gpui::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, Styled as _, Window, div,
};

use crate::actions::Dialog;
use crate::files::Template;
use crate::props::Props;
use crate::review::Review;

/// What ElyDoc keeps per tab.
#[derive(Default)]
pub struct Extra {
    /// The kernel's counts, read when its word count moves.
    pub counts: Option<Counts>,
    /// The word count state last asked about.
    pub counted: Option<Option<String>>,
    /// A count is on its way.
    pub counting: bool,
}

pub struct Docs {
    pub core: Core<Docs>,
    pub dialog: Option<Dialog>,
    pub props: Entity<Props>,
    pub review: Entity<Review>,
    /// What the right column shows.
    pub side: Side,
    pub offer: Option<ely_ui::menu::Offer<ely_lok::menu::Offered>>,
    pub merge: Entity<crate::merge::Merge>,
    pub navigator: Entity<crate::navigate::Navigator>,
    /// Where jumps left from, latest last.
    pub back: Vec<(u64, ely_lok::Rect)>,
    /// Last jump: tab, input, prior cursor moves, target.
    pub going: Option<(u64, u64, u64, ely_lok::Rect)>,
    /// Dialog reads asked; a dialog or tab change voids older.
    pub dialog_asked: u64,
    dialog_seen: Seen,
    /// Undo and Redo as the menus show them.
    undo_menu: (bool, bool),
}

/// Open dialog kinds, Ely's and the shell's, and the tab.
type Seen = (Option<std::mem::Discriminant<Dialog>>, Option<std::mem::Discriminant<ely_office::Dialog>>, Option<u64>);

/// The right column's panels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Side {
    Props,
    Review,
    Merge,
}

impl Host for Docs {
    type Extra = Extra;
    type Engine = ely_office::Lok;
    const KIND: Kind = Kind {
        app: AppId::Doc,
        kept: &["docx", "odt", "doc", "rtf", "fodt", "txt"],
        save_as: &["docx", "odt"],
        macros: Some(&["docm", "dotm"]),
    };

    fn core(&self) -> &Core<Self> {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core<Self> {
        &mut self.core
    }

    /// Tab stops come with the events: the view's state.
    fn read_at(&self, tab: u64, cx: &gpui::App) -> Option<ely_lok::Doc> {
        self.tab(tab).map(|tab| tab.view.read(cx).read_at())
    }

    fn observed(&mut self, id: u64, view: &Entity<LokView>, cx: &mut Context<Self>) {
        self.count(id, view, cx);
        self.found(id, view, cx);
        self.offer_menu(id, view, cx);
    }

    /// A dialog about a closed tab goes with it.
    fn tab_closed(&mut self, tab: &ely_office::Tab<Extra, ely_office::Lok>, cx: &mut Context<Self>) {
        if matches!(self.dialog, Some(Dialog::Output { tab: of, .. }) if of == tab.id) {
            self.dialog = None;
            cx.notify();
        }
    }

    fn search_leaves(&mut self, cx: &mut Context<Self>) {
        self.mark_place(cx);
    }
}

/// ElyDoc's menus after File; `undo` greys Undo and Redo.
fn menus(undo: (bool, bool), cx: &App) -> Vec<gpui::Menu> {
    vec![
        gpui::Menu::new(ely_ui::i18n::text(cx, "menu.edit", &[])).items(ely_office::undo_items(undo, cx)),
        gpui::Menu::new(ely_ui::i18n::text(cx, "menu.view", &[])).items(
            ely_office::zoom_items(false, cx)
                .into_iter()
                .chain([gpui::MenuItem::separator(), ely_ui::commands::left_panel_item(cx), ely_ui::commands::right_panel_item(cx)]),
        ),
        crate::insert::menu(cx),
        crate::review::menu(cx),
        crate::merge::menu(cx),
    ]
}

/// The window root with a blank document.
pub fn open(app: AppId, window: &mut Window, cx: &mut App) -> Entity<Root> {
    ely_lok::init(cx);
    let (worker, work) = match ely_office::start(app, cx) {
        Ok(started) => started,
        Err(error) => ely_platform::startup::fail(app, &format!("{error}")),
    };
    let docs = cx.new(|cx| {
        let mut host = Docs::new(worker, work, cx);
        // Recovery points, crash return, autosave (§6.2, §6.3).
        ely_office::Recovering::start_recovery(&mut host, cx);
        <Docs as ely_office::Autosaving>::start_autosave(cx);
        host
    });
    cx.bind_keys([
        gpui::KeyBinding::new(
            "secondary-f",
            ely_office::FindReplace,
            Some(commands::CONTEXT),
        ),
        gpui::KeyBinding::new(
            "secondary-[",
            crate::navigate::GoBack,
            Some(commands::CONTEXT),
        ),
    ]);
    ely_office::bind_zoom_keys(cx);
    ely_office::bind_undo_keys(cx);
    let root = ely_ui::document_root(app, docs.clone().into(), &ely_ui::commands::FILE, menus((false, false), cx), window, cx);
    docs.update(cx, |docs, cx| {
        docs.core.root = root.downgrade();
        docs.own_window(window, cx);
        window.focus(&docs.core.focus, cx);
    });
    let launched = ely_ui::launch_files(cx);
    docs.update(cx, |docs, cx| match launched.is_empty() {
        true => docs.new_from(Template::Blank, cx),
        false => launched.into_iter().for_each(|path| docs.open_file(path, cx)),
    });
    let opening = docs.clone();
    ely_ui::on_open(cx, move |path, cx| {
        opening.update(cx, |docs, cx| docs.open_file(path, cx))
    });
    let closing = docs.clone();
    ely_gpui_component::shell::guard_close(window, cx, move |window, cx| {
        closing.update(cx, |docs, cx| docs.should_close(window, cx))
    });
    root
}

impl Docs {
    pub fn new(worker: Arc<Mutex<Worker>>, work: PathBuf, cx: &mut Context<Self>) -> Self {
        let props = cx.new(|_| Props::new(Arc::clone(&worker)));
        let review = cx.new(|_| Review::new(Arc::clone(&worker)));
        let weak = cx.weak_entity();
        let merge = cx.new(|_| crate::merge::Merge::new(weak.clone()));
        let navigator = cx.new(|_| crate::navigate::Navigator::new(weak, Arc::clone(&worker)));
        let failed = |docs: &mut Docs, error: &LokError, cx: &mut Context<Docs>| {
            docs.engine_failed(&error.0, cx)
        };
        cx.subscribe(&props, move |docs, _, error, cx| failed(docs, error, cx))
            .detach();
        cx.subscribe(&review, move |docs, _, error, cx| failed(docs, error, cx))
            .detach();
        cx.subscribe(&navigator, move |docs, _, error, cx| {
            failed(docs, error, cx)
        })
        .detach();
        cx.observe_self(|docs, _| docs.watch_dialogs()).detach();
        Docs {
            core: Core::new(worker, work, cx),
            dialog: None,
            props,
            review,
            side: Side::Props,
            offer: None,
            merge,
            navigator,
            back: Vec::new(),
            going: None,
            dialog_asked: 0,
            dialog_seen: (None, None, None),
            undo_menu: (false, false),
        }
    }

    /// Shows `side` in the right column, opening it.
    pub fn show_side(&mut self, side: Side, cx: &mut Context<Self>) {
        self.side = side;
        ely_ui::settings::panels::show(false, true, cx);
        cx.notify();
    }

    /// A dialog or tab change ends earlier dialog reads.
    fn watch_dialogs(&mut self) {
        let ely = self.dialog.as_ref().map(std::mem::discriminant);
        let now = (ely, self.core.dialog.as_ref().map(std::mem::discriminant), self.core.active);
        if self.dialog_seen != now {
            self.dialog_seen = now;
            self.dialog_asked += 1;
        }
    }

    pub fn text(cx: &App, key: &str, args: &[(&str, &str)]) -> SharedString {
        ely_ui::i18n::text(cx, key, args)
    }

    /// A new untitled document from a bundled template (DOC-01).
    pub fn new_from(&mut self, template: Template, cx: &mut Context<Self>) {
        let name = format!("{}.fodt", template.key());
        self.new_from_text(&name, &template.fodt(cx), cx);
    }
}

impl Docs {
    /// Greys Undo and Redo as the active document's kernel says.
    fn sync_menus(&mut self, cx: &mut Context<Self>) {
        let able = self.active_tab().map_or((false, false), |tab| ely_office::undo_able(tab.view.read(cx)));
        if able != self.undo_menu {
            self.undo_menu = able;
            log::info!("doc: undo {}, redo {}", able.0, able.1);
            cx.defer(move |cx| {
                ely_ui::set_document_menus(AppId::Doc, &ely_ui::commands::FILE, menus(able, cx), cx);
                cx.refresh_windows();
            });
        }
    }
}

impl Render for Docs {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_menus(cx);
        let title = self
            .active_tab()
            .map_or_else(|| AppId::Doc.name().into(), |tab| tab.name.clone());
        let side = self.side_view(cx);
        let shell = Shell::new(title, &self.core.focus)
            .tabs(self.tab_strip(cx))
            .left(self.left_view(cx))
            .right(side)
            .status(self.status(cx))
            .child(self.body(cx));
        let root = div()
            .size_full()
            .key_context(commands::CONTEXT)
            .track_focus(&self.core.focus)
            .on_action(cx.listener(Self::on_new))
            .on_action(cx.listener(Self::on_open))
            .on_action(cx.listener(Self::on_save))
            .on_action(cx.listener(Self::on_save_as))
            .on_action(cx.listener(Self::on_close_tab))
            .child(shell)
            .children(self.core_dialog(cx))
            .children(self.dialog_element(window, cx));
        Self::bind_undo(Self::bind_zoom(crate::insert::bind(root, cx), cx), cx)
    }
}

impl Zooming for Docs {}
