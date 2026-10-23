//! PDFs in the shared shell, one worker behind (PDF-01).

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use ely_gpui_component::{
    feedback::EmptyState,
    primitives::IconName,
    shell::{StatusBar, StatusBarItem, ZoomControl},
};
use ely_office::{Core, Host, Kind, prelude::*};
use ely_platform::AppId;
use ely_ui::{Root, Shell, commands};
use gpui::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, PathPromptOptions, Render, SharedString, Styled as _, Window, div,
};

use crate::engine::{Mupdf, Pdf};
use crate::view::PdfView;

pub struct Viewer {
    pub core: Core<Viewer>,
    pub side: Entity<crate::side::Side>,
    /// Crop or split, while asked.
    pub ask: Option<crate::ask::Ask>,
    pub edit_panel: Entity<crate::editing::EditPanel>,
    pub note_panel: Entity<crate::notes::NotePanel>,
    /// A layout change shown before it is made.
    pub preview: Option<crate::editing::Preview>,
    /// Preview pictures no longer drawn, freed next render.
    pub retired: ely_gpui_component::primitives::Retired,
    pub find: Option<crate::search::Find>,
    /// A bookmark title being asked.
    pub naming: Option<crate::marks::Naming>,
    pub props: Option<crate::props::Props>,
    pub ocr: Option<crate::ocr::Ocr>,
    /// OCR runs started so far.
    pub ocr_runs: u64,
    pub removal: Option<crate::redact::Removal>,
    pub secure: Option<crate::protect::Secure>,
    pub signer: Option<crate::sign::Signer>,
    pub making: Option<crate::visible::Making>,
    pub signs_open: bool,
    pub exporting: Option<crate::output::Exporting>,
    pub repair_open: bool,
    /// Find boxes opened so far.
    pub finds: u64,
}

impl Host for Viewer {
    type Extra = ();
    type Engine = Pdf;
    const KIND: Kind = Kind {
        app: AppId::Pdf,
        kept: &["pdf"],
        save_as: &["pdf"],
        macros: None,
    };

    fn core(&self) -> &Core<Self> {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core<Self> {
        &mut self.core
    }

    fn read_at(&self, tab: u64, cx: &gpui::App) -> Option<ely_pdf_engine::Doc> {
        self.tab(tab).map(|tab| tab.view.read(cx).doc)
    }

    fn observed(&mut self, id: u64, view: &Entity<PdfView>, cx: &mut Context<Self>) {
        // Damage found while drawing: no overwrite from now on.
        if view.read(cx).damage_seen {
            view.update(cx, |view, _| view.damage_seen = false);
            if let Some(tab) = self.tab_mut(id) {
                log::warn!("pdf: tab {id} turned out damaged");
                tab.risk = Some(ely_office::Risk::Repaired);
            }
        }
        // Words found before an edit are searched again.
        let version = view.read(cx).version;
        if self.find.as_ref().is_some_and(|find| find.tab == id && find.version != version) {
            self.search(cx);
        }
        // A preview from before an edit is void.
        if self.preview.as_ref().is_some_and(|preview| preview.tab == id && preview.version != version) {
            self.end_preview();
        }
        cx.notify();
    }
}

/// The window root with no document.
pub fn open(app: AppId, window: &mut Window, cx: &mut App) -> Entity<Root> {
    crate::keys::bind(cx);
    let work = ely_platform::data_dir(app)
        .map_err(std::io::Error::other)
        .and_then(|data| ely_office::fresh_work(&data));
    let work = match work {
        Ok(work) => work,
        Err(error) => ely_platform::startup::fail(app, &format!("{error}")),
    };
    // The worker starts with the first file (PERF-01).
    let viewer = cx.new(|cx| {
        let mut host = Viewer::new(Arc::default(), work, cx);
        // Recovery points, crash return, autosave (§6.2, §6.3).
        ely_office::Recovering::start_recovery(&mut host, cx);
        <Viewer as ely_office::Autosaving>::start_autosave(cx);
        host
    });
    let root = ely_ui::document_root(app, viewer.clone().into(), &commands::FILE, crate::keys::menus(cx), window, cx);
    viewer.update(cx, |viewer, cx| {
        viewer.core.root = root.downgrade();
        viewer.own_window(window, cx);
        window.focus(&viewer.core.focus, cx);
    });
    for path in ely_ui::launch_files(cx) {
        viewer.update(cx, |viewer, cx| viewer.open_file(path, cx));
    }
    let opening = viewer.clone();
    ely_ui::on_open(cx, move |path, cx| {
        opening.update(cx, |viewer, cx| viewer.open_file(path, cx))
    });
    let closing = viewer.clone();
    ely_gpui_component::shell::guard_close(window, cx, move |window, cx| {
        closing.update(cx, |viewer, cx| viewer.should_close(window, cx))
    });
    root
}

impl Viewer {
    pub fn new(worker: Arc<Mutex<Mupdf>>, work: PathBuf, cx: &mut Context<Self>) -> Self {
        let (this, weak) = (cx.entity(), cx.weak_entity());
        Viewer {
            core: Core::new(worker, work, cx),
            side: cx.new(|cx| crate::side::Side::new(&this, cx)),
            ask: None,
            edit_panel: cx.new(|_| crate::editing::EditPanel::new(weak.clone())),
            note_panel: cx.new(|_| crate::notes::NotePanel::new(weak.clone())),
            preview: None,
            retired: Default::default(),
            find: None,
            naming: None,
            props: None,
            ocr: None,
            ocr_runs: 0,
            removal: None,
            secure: None,
            signer: None,
            making: None,
            signs_open: false,
            exporting: None,
            repair_open: false,
            finds: 0,
        }
    }

    pub fn text(cx: &App, key: &str, args: &[(&str, &str)]) -> SharedString {
        ely_ui::i18n::text(cx, key, args)
    }

    /// The active tab's view.
    pub fn active_view(&self) -> Option<Entity<PdfView>> {
        self.active_tab().map(|tab| tab.view.clone())
    }

    /// Runs `change` on the active view.
    pub fn on_view(&self, change: impl FnOnce(&mut PdfView, &mut Context<PdfView>), cx: &mut App) {
        if let Some(view) = self.active_view() {
            view.update(cx, change);
        }
    }

    /// Asks for files; each opens in its own tab.
    pub fn open_picked(&mut self, _: &commands::Open, _: &mut Window, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: None,
        });
        cx.spawn(async move |this, cx| {
            let picked = match picked.await {
                Ok(Ok(paths)) => Ok(paths),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            let done = this.update(cx, |viewer, cx| match picked {
                Ok(Some(paths)) => paths.into_iter().for_each(|path| viewer.open_file(path, cx)),
                Ok(None) => log::info!("pdf: the open panel was cancelled"),
                Err(error) => viewer.panel_failed(&error, cx),
            });
            if done.is_err() {
                log::info!("pdf: the viewer closed before the panel");
            }
        })
        .detach();
    }

    fn status(&self, cx: &mut Context<Self>) -> StatusBar {
        let state = StatusBarItem::new("pdf-state").label(self.save_state(cx));
        let Some(view) = self.active_view() else {
            return StatusBar::new().left(state);
        };
        let view = view.read(cx);
        let (n, count) = ((view.current + 1).to_string(), view.sizes.len().to_string());
        let page = Self::text(cx, "pdf.status.page", &[("n", &n), ("count", &count)]);
        let percent = view.percent();
        let this = cx.entity().downgrade();
        let zoom = ZoomControl::new("pdf-zoom").owned(percent, move |step, _, cx| {
            if let Err(error) = this.update(cx, |viewer, cx| crate::keys::zoom(viewer, step, cx)) {
                log::warn!("pdf: zoom without a viewer: {error}");
            }
        });
        StatusBar::new()
            .left(state)
            .left(StatusBarItem::new("pdf-page").label(page))
            .right(div().debug_selector(|| "pdf-zoom".into()).child(zoom))
    }
}

impl Render for Viewer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.retired.release(window);
        let title = self.active_tab().map_or_else(|| AppId::Pdf.name().into(), |tab| tab.name.clone());
        let body = match self.active_view() {
            Some(view) => view.into_any_element(),
            None => EmptyState::new("pdf-empty", IconName::File, Self::text(cx, "command.blocked.no_document", &[])).into_any_element(),
        };
        let shell = Shell::new(title, &self.core.focus)
            .tabs(self.tab_strip(cx))
            .left(self.active_view().map(|_| self.side.clone().into()))
            .right(self.active_view().and_then(|view| view.read(cx).panel_aim()).map(|aim| match aim {
                crate::content::Aim::Note(_) => self.note_panel.clone().into(),
                _ => self.edit_panel.clone().into(),
            }))
            .status(self.status(cx))
            .children(self.file_banner(cx))
            .children(self.find_bar(cx))
            .child(body);
        // Esc leaves a tool from pages and panel alike.
        let editing = self.active_view().is_some_and(|view| view.read(cx).tool != crate::content::Tool::Read);
        let context = match editing {
            true => format!("{} {}", commands::CONTEXT, crate::writes::EDITING),
            false => commands::CONTEXT.to_string(),
        };
        let root = div()
            .size_full()
            .key_context(context.as_str())
            .track_focus(&self.core.focus)
            .on_action(cx.listener(Self::open_picked))
            .on_action(cx.listener(Self::on_save))
            .on_action(cx.listener(Self::on_save_as))
            .on_action(cx.listener(Self::on_export))
            .on_action(cx.listener(Self::on_print))
            .on_action(cx.listener(Self::on_close_tab))
            .child(shell)
            .children(self.core_dialog(cx))
            .children(self.ask_element(cx))
            .children(self.preview_element(cx))
            .children(self.naming_element(cx))
            .children(self.props_element(cx))
            .children(self.ocr_element(cx))
            .children(self.removal_element(cx))
            .children(self.secure_element(cx))
            .children(self.sign_element(cx))
            .children(self.visible_element(cx))
            .children(self.signs_element(cx))
            .children(self.export_element(cx))
            .children(self.repair_element(cx));
        let root = crate::writes::on(crate::organize::on(crate::keys::on(root, cx), cx), cx);
        crate::document::on(crate::marking::on(root, cx), cx)
    }
}
