//! Copying a file's name and path, from tab or row.

use std::path::{Path, PathBuf};

use ely_gpui_component::{
    menus::{ContextMenu, Menu, MenuItem},
    navigation::EditorTabs,
};
use ely_office::prelude::*;
use ely_ui::{i18n::text, menu::Offer};
use gpui::{App, ClipboardItem, Context, Pixels, Point};

use crate::desk::Desk;

/// Copy File Name, then Copy Path; roots have no name.
pub fn copies(menu: Menu, path: &Path, cx: &App) -> Menu {
    let menu = match path.file_name() {
        Some(name) => {
            let name = name.to_string_lossy().into_owned();
            menu.item(copy(text(cx, "mark.copy.name", &[]), name, "name"))
        }
        None => menu,
    };
    let whole = path.display().to_string();
    menu.item(copy(text(cx, "mark.copy.path", &[]), whole, "path"))
}

fn copy(label: gpui::SharedString, words: String, what: &'static str) -> MenuItem {
    MenuItem::new(label).on_click(move |_, cx| {
        cx.write_to_clipboard(ClipboardItem::new_string(words.clone()));
        log::info!("mark: copied a file's {what}");
    })
}

impl Desk {
    /// The tab strip; a right click offers the tab's file.
    pub(crate) fn tabs(&self, cx: &mut Context<Self>) -> EditorTabs {
        let desk = cx.weak_entity();
        self.tab_strip(cx).on_context(move |id, at, _, cx| {
            let id = id.parse().expect("tab ids are numbers");
            desk.update(cx, |desk, cx| desk.ask_tab(id, at, cx)).ok();
        })
    }

    fn ask_tab(&mut self, id: u64, at: Point<Pixels>, cx: &mut Context<Self>) {
        let path = self.core.tabs.iter().find(|tab| tab.id == id).and_then(|tab| tab.session.path());
        let Some(path) = path.map(PathBuf::from) else {
            return log::info!("mark: tab {id} has no file to copy");
        };
        self.tab_asked = Some(Offer::new(self.tab_asked.as_ref(), id, path, at));
        cx.notify();
    }

    /// The open tab menu's host, if one was asked.
    pub(crate) fn tab_menu(&self, cx: &App) -> Option<ContextMenu> {
        let offer = self.tab_asked.as_ref()?;
        Some(offer.host("mark-tab-menu", copies(Menu::new(), &offer.what, cx)))
    }
}
