//! Files, outline and search beside the document (MARK-05, MARK-10).

use std::path::PathBuf;

use ely_gpui_component::{
    buttons::{Button, SegmentedControl},
    feedback::EmptyState,
    forms::{InputEvent, SearchInput, TextInput},
    layout::ScrollArea,
    lists::{FileTree, ListItem, Outline, SelectableList},
    menus::{ContextMenu, Menu, MenuItem},
    primitives::IconName,
    theme::ControlSize,
};
use ely_office::{menu::Offer, prelude::*};
use ely_ui::{commands, i18n::text};
use gpui::{
    AnyElement, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, Styled as _, WeakEntity, Window, div,
};

use crate::desk::Desk;
use crate::folder::{Hit, MOST_HITS, lines_with};

#[derive(Clone, Copy, PartialEq)]
enum Pane {
    Files,
    Outline,
    Search,
}

pub struct Side {
    desk: WeakEntity<Desk>,
    pane: Pane,
    query: Entity<TextInput>,
    /// Search the whole folder, else the shown file.
    wide: bool,
    /// Each hit, with its tab when the file is open.
    hits: Vec<(Option<u64>, Hit)>,
    /// Searches started; a late answer is dropped.
    runs: u64,
    /// The words searched, and the tab when one file was.
    searched: String,
    tab: Option<u64>,
    /// The folder listing searched.
    listing: u64,
    /// Files that could not be read, and why.
    missed: Vec<(PathBuf, String)>,
    /// A recent row's menu; each opens once.
    asked: Option<ely_office::menu::Offer<PathBuf>>,
}

impl Side {
    pub fn new(desk: &Entity<Desk>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // A new shown file is searched again.
        cx.observe(desk, |side, desk, cx| {
            let (active, listing) = (desk.read(cx).core.active, desk.read(cx).listings);
            let shown = !side.wide && side.tab != active;
            let listed = side.wide && side.listing != listing;
            if !side.searched.is_empty() && (shown || listed) {
                side.search(cx);
            }
            cx.notify();
        })
        .detach();
        let query = cx.new(|cx| TextInput::new(window, cx));
        cx.subscribe(&query, |side, _, event: &InputEvent, cx| {
            if *event == InputEvent::Submit {
                side.search(cx);
            }
        })
        .detach();
        Side {
            desk: desk.downgrade(),
            pane: Pane::Files,
            query,
            wide: false,
            hits: Vec::new(),
            runs: 0,
            searched: String::new(),
            tab: None,
            listing: 0,
            missed: Vec::new(),
            asked: None,
        }
    }

    /// Lines holding the query, off the window.
    fn search(&mut self, cx: &mut Context<Self>) {
        let query = self.query.read(cx).text().trim().to_owned();
        let Some(desk) = self.desk.upgrade() else {
            return;
        };
        self.runs += 1;
        let run = self.runs;
        let desk = desk.read(cx);
        (self.tab, self.listing) = (desk.core.active, desk.listings);
        self.searched = query.clone();
        if query.is_empty() {
            (self.hits, self.missed) = (Vec::new(), Vec::new());
            return cx.notify();
        }
        // Open files search as they are now, saved or not.
        let open: Vec<(u64, PathBuf, String)> = desk
            .core
            .tabs
            .iter()
            .filter_map(|tab| {
                let name = PathBuf::from(tab.name.as_ref());
                let path = tab.session.path().map_or(name, PathBuf::from);
                Some((tab.id, path, desk.text_of(tab.id)?))
            })
            .collect();
        // Open files by real path, as the folder lists them.
        let reals: Vec<(u64, PathBuf)> = open
            .iter()
            .filter_map(|(id, path, _)| Some((*id, dunce::canonicalize(path).ok()?)))
            .collect();
        let sources: Vec<(Option<u64>, PathBuf)> = match (self.wide, &desk.folder) {
            (true, Some(folder)) => folder
                .files
                .iter()
                .map(|file| folder.root.join(file))
                .map(|path| {
                    let held = reals
                        .iter()
                        .find(|(_, real)| *real == path)
                        .map(|(id, _)| *id);
                    (held, path)
                })
                .collect(),
            _ => desk
                .active_tab()
                .map(|tab| (Some(tab.id), PathBuf::new()))
                .into_iter()
                .collect(),
        };
        log::info!("mark: searches {} files", sources.len());
        cx.spawn(async move |side, cx| {
            let hits = cx
                .background_executor()
                .spawn(async move {
                    let (mut hits, mut missed) = (Vec::new(), Vec::new());
                    for (tab, path) in sources {
                        let held = open.iter().find(|(id, ..)| Some(*id) == tab);
                        let (path, text) = match held {
                            Some((_, path, text)) => (path.clone(), text.clone()),
                            None => {
                                match std::fs::read(&path).and_then(crate::text::Source::decode) {
                                    Ok(source) => (path, source.text),
                                    Err(error) => {
                                        log::warn!(
                                            "mark: {} was not searched: {error}",
                                            path.display()
                                        );
                                        missed.push((path, error.to_string()));
                                        continue;
                                    }
                                }
                            }
                        };
                        let mut found = Vec::new();
                        lines_with(&path, &text, &query, &mut found);
                        hits.extend(found.into_iter().map(|hit| (tab, hit)));
                        if hits.len() >= MOST_HITS {
                            hits.truncate(MOST_HITS);
                            break;
                        }
                    }
                    (hits, missed)
                })
                .await;
            side.update(cx, |side, cx| {
                if side.runs == run {
                    (side.hits, side.missed) = hits;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn files(&self, desk: &Entity<Desk>, cx: &mut Context<Self>) -> AnyElement {
        let read = desk.read(cx);
        if let Some(folder) = &read.folder {
            let (root, opener) = (folder.root.clone(), desk.downgrade());
            return FileTree::new("mark-files", folder.files.clone())
                .on_open(move |path, _, cx| {
                    let path = root.join(path.as_ref());
                    opener.update(cx, |desk, cx| desk.open_path(path, cx)).ok();
                })
                .size_full()
                .into_any_element();
        }
        if read.recent.is_empty() {
            let open = Button::new("mark-open", text(cx, "command.file.open", &[]))
                .on_click(|_, window, cx| window.dispatch_action(Box::new(commands::Open), cx));
            return EmptyState::new(
                "mark-no-recent",
                IconName::Folder,
                text(cx, "mark.recent.none", &[]),
            )
            .action(open)
            .into_any_element();
        }
        let (opener, clearer, side) = (desk.downgrade(), desk.downgrade(), cx.weak_entity());
        let rows = read.recent.iter().map(|path| {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let key: SharedString = path.to_string_lossy().into_owned().into();
            let (side, file) = (side.clone(), path.clone());
            let item = ListItem::new(key.clone(), name)
                .description(
                    path.parent()
                        .map(|dir| dir.display().to_string())
                        .unwrap_or_default(),
                )
                .on_secondary(move |at, _, cx| {
                    side.update(cx, |side, cx| {
                        side.asked = Some(Offer::new(side.asked.as_ref(), 0, file.clone(), at));
                        cx.notify();
                    })
                    .ok();
                });
            (key, item)
        });
        let list = rows
            .fold(
                SelectableList::new("mark-recent").label(text(cx, "mark.recent", &[])),
                |list, (key, item)| list.row(key, item),
            )
            .on_activate(move |key, _, cx| {
                let path = PathBuf::from(key.as_ref());
                opener.update(cx, |desk, cx| desk.open_path(path, cx)).ok();
            });
        let clear = Button::new("mark-clear-recent", text(cx, "mark.recent.clear", &[]))
            .size(ControlSize::Sm)
            .on_click(move |_, _, cx| {
                clearer.update(cx, |desk, cx| desk.clear_recent(cx)).ok();
            });
        div()
            .flex()
            .flex_col()
            .gap_2()
            .size_full()
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .debug_selector(|| "mark-recent-list".into())
                    .child(self.recent_menu(desk, cx).child(list)),
            )
            .child(
                div()
                    .debug_selector(|| "mark-clear-recent".into())
                    .child(clear),
            )
            .into_any_element()
    }

    /// Takes one file out of the recent list.
    fn recent_menu(&self, desk: &Entity<Desk>, cx: &mut Context<Self>) -> ContextMenu {
        let Some(offer) = &self.asked else {
            return ContextMenu::new("mark-recent-menu", Menu::new()).manual(None).size_full();
        };
        let (desk, path) = (desk.downgrade(), offer.what.clone());
        let remove = MenuItem::new(text(cx, "mark.recent.remove", &[])).on_click(move |_, cx| {
            desk.update(cx, |desk, cx| desk.forget(&path, cx)).ok();
        });
        offer.host("mark-recent-menu", Menu::new().item(remove)).size_full()
    }

    fn outline(&self, desk: &Entity<Desk>, cx: &mut Context<Self>) -> AnyElement {
        let Some(view) = desk.read(cx).active_view() else {
            return EmptyState::new(
                "mark-no-outline",
                IconName::File,
                text(cx, "command.blocked.no_document", &[]),
            )
            .into_any_element();
        };
        let headings = view.read(cx).outline(cx);
        if headings.is_empty() {
            return EmptyState::new(
                "mark-no-headings",
                IconName::File,
                text(cx, "mark.outline.none", &[]),
            )
            .into_any_element();
        }
        let outline = headings.iter().enumerate().fold(
            Outline::new("mark-outline"),
            |outline, (at, heading)| {
                outline.item(at.to_string(), heading.title.clone(), heading.level)
            },
        );
        let outline = outline.on_select(move |key, window, cx| {
            let Some(heading) = key.parse::<usize>().ok().and_then(|at| headings.get(at)) else {
                return log::error!("mark: the outline chose no heading");
            };
            // The heading in view, the caret on it.
            let line = heading.line;
            view.update(cx, |view, cx| view.reveal((line, 0), window, cx));
        });
        ScrollArea::new("mark-outline-scroll")
            .vertical()
            .size_full()
            .child(
                div()
                    .debug_selector(|| "mark-outline".into())
                    .child(outline),
            )
            .into_any_element()
    }

    fn search_pane(&self, desk: &Entity<Desk>, cx: &mut Context<Self>) -> AnyElement {
        let this = cx.entity();
        let scope = SegmentedControl::new("mark-scope", if self.wide { "folder" } else { "file" })
            .size(ControlSize::Sm)
            .segment(
                "file",
                text(cx, "mark.search.file", &[]),
                Some(IconName::File),
            );
        let scope = match desk.read(cx).folder {
            Some(_) => scope.segment(
                "folder",
                text(cx, "mark.search.folder", &[]),
                Some(IconName::Folder),
            ),
            None => scope,
        };
        let scope = scope.on_change(move |key, _, cx| {
            this.update(cx, |side, cx| {
                side.wide = key == "folder";
                side.search(cx);
            })
        });
        let opener = desk.downgrade();
        let hits = self.hits.clone();
        // Files not read come first: the results lack them.
        let missed = self.missed.iter().enumerate().map(|(at, (path, why))| {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let title = text(cx, "mark.search.missed", &[("name", &name)]);
            (
                SharedString::from(format!("missed-{at}")),
                ListItem::new(format!("missed-{at}"), title).description(why.clone()),
            )
        });
        let rows = self.hits.iter().enumerate().map(|(at, (_, hit))| {
            let name = hit
                .file
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let item = ListItem::new(at.to_string(), hit.text.clone())
                .description(format!("{name}:{}", hit.line));
            (SharedString::from(at.to_string()), item)
        });
        let list = missed
            .collect::<Vec<_>>()
            .into_iter()
            .chain(rows)
            .fold(
                SelectableList::new("mark-hits").label(text(cx, "mark.search", &[])),
                |list, (key, item)| list.row(key, item),
            )
            .on_activate(move |key, window, cx| {
                let Some((tab, hit)) = key.parse::<usize>().ok().and_then(|at| hits.get(at)) else {
                    return;
                };
                // Hits count lines from one.
                let (tab, file, at) = (*tab, hit.file.clone(), (hit.line - 1, hit.column));
                opener
                    .update(cx, |desk, cx| desk.open_at(tab, file, at, window, cx))
                    .ok();
            });
        let more = (self.hits.len() == MOST_HITS)
            .then(|| text(cx, "mark.search.more", &[("count", &MOST_HITS.to_string())]));
        div()
            .flex()
            .flex_col()
            .gap_2()
            .size_full()
            .child(SearchInput::new("mark-search", &self.query))
            .child(scope)
            .child(list)
            .children(more)
            .into_any_element()
    }
}

impl Render for Side {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(desk) = self.desk.upgrade() else {
            return div().into_any_element();
        };
        let this = cx.entity();
        let key = match self.pane {
            Pane::Files => "files",
            Pane::Outline => "outline",
            Pane::Search => "search",
        };
        let panes = SegmentedControl::new("mark-pane", key)
            .size(ControlSize::Sm)
            .segment(
                "files",
                text(cx, "mark.pane.files", &[]),
                Some(IconName::Folder),
            )
            .segment(
                "outline",
                text(cx, "mark.pane.outline", &[]),
                Some(IconName::List),
            )
            .segment(
                "search",
                text(cx, "mark.pane.search", &[]),
                Some(IconName::Search),
            )
            .on_change(move |key, _, cx| {
                this.update(cx, |side, cx| {
                    side.pane = match key.as_ref() {
                        "outline" => Pane::Outline,
                        "search" => Pane::Search,
                        _ => Pane::Files,
                    };
                    cx.notify();
                })
            });
        let body = match self.pane {
            Pane::Files => self.files(&desk, cx),
            Pane::Outline => self.outline(&desk, cx),
            Pane::Search => self.search_pane(&desk, cx),
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .size_full()
            .child(panes)
            .child(div().flex_1().min_h_0().child(body))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests;
