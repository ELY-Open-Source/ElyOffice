//! Bookmarks: jump, add, rename, delete (PDF-01, PDF-07).

use ely_gpui_component::{
    feedback::EmptyState,
    forms::TextInput,
    layout::ScrollArea,
    lists::Outline,
    menus::{ContextMenu, Menu as Popup, MenuItem as PopupItem},
    overlays::PromptDialog,
    primitives::IconName,
};
use ely_office::{Closing as _, Tabs as _};
use ely_pdf_engine::{Bookmark, BookmarkChange};
use gpui::{Action, AnyElement, App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Styled as _, Window, div};

use crate::document::{DeleteBookmark, RenameBookmark};
use crate::view::PdfView;
use crate::viewer::Viewer;

/// One bookmark: title, level, target, and where it sits.
#[derive(Clone, Debug, PartialEq)]
pub struct Flat {
    pub title: String,
    pub level: usize,
    pub to: Option<(usize, f32)>,
    /// Child indices from the top.
    pub path: Vec<usize>,
}

/// Bookmarks depth first.
pub fn flat(marks: &[Bookmark], above: &[usize], out: &mut Vec<Flat>) {
    for (at, mark) in marks.iter().enumerate() {
        let path: Vec<usize> = above.iter().copied().chain([at]).collect();
        out.push(Flat { title: mark.title.clone(), level: path.len(), to: mark.page.map(|page| (page, mark.y)), path: path.clone() });
        flat(&mark.down, &path, out);
    }
}

/// A new bookmark's place: after the pick, else last.
pub fn place_for(marks: &[Bookmark], picked: Option<&Flat>) -> Vec<usize> {
    match picked {
        Some(flat) => {
            let mut path = flat.path.clone();
            *path.last_mut().expect("a path has a step") += 1;
            path
        }
        None => vec![marks.len()],
    }
}

/// A bookmark title being typed.
pub struct Naming {
    pub tab: u64,
    pub version: u64,
    pub field: Entity<TextInput>,
    /// None adds at `path`; Some renames the bookmark titled so.
    pub expect: Option<String>,
    pub path: Vec<usize>,
    pub page: usize,
}

impl PdfView {
    /// The picked bookmark, if it is still there.
    pub fn picked_mark(&self) -> Option<Flat> {
        let mut all = Vec::new();
        flat(&self.marks, &[], &mut all);
        self.mark_pick.and_then(|at| all.get(at).cloned())
    }
}

/// The bookmarks of `view`; a press picks and jumps.
pub fn outline(view: &Entity<PdfView>, cx: &App) -> AnyElement {
    let mut items = Vec::new();
    flat(&view.read(cx).marks, &[], &mut items);
    if items.is_empty() {
        return EmptyState::new("pdf-marks-none", IconName::File, Viewer::text(cx, "pdf.marks.none", &[])).into_any_element();
    }
    let targets: Vec<Option<(usize, f32)>> = items.iter().map(|flat| flat.to).collect();
    let picked = view.read(cx).mark_pick;
    let view = view.clone();
    let mut outline = (items.into_iter().enumerate())
        .fold(Outline::new("pdf-marks"), |outline, (at, flat)| outline.item(at.to_string(), flat.title, flat.level))
        .on_select(move |key, _, cx| {
            let Ok(at) = key.parse::<usize>() else {
                return;
            };
            view.update(cx, |view, cx| {
                view.mark_pick = Some(at);
                match targets.get(at).copied().flatten() {
                    Some((page, y)) => view.show(page, y, cx),
                    None => cx.notify(),
                }
            });
        });
    if let Some(at) = picked {
        outline = outline.current(at.to_string());
    }
    let item = |key: &str, action: Box<dyn Action>| {
        PopupItem::new(Viewer::text(cx, key, &[])).on_click(move |window, cx| window.dispatch_action(action.boxed_clone(), cx))
    };
    let popup = Popup::new().item(item("pdf.mark.rename", Box::new(RenameBookmark))).item(item("pdf.mark.delete", Box::new(DeleteBookmark)));
    let list = div().debug_selector(|| "pdf-marks".into()).p_2().child(outline);
    let list = ScrollArea::new("pdf-marks-scroll").vertical().size_full().child(list);
    ContextMenu::new("pdf-marks-menu", popup).size_full().child(list).into_any_element()
}

impl Viewer {
    /// Asks a title for a new or picked bookmark.
    pub fn ask_bookmark(&mut self, add: bool, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(tab), Some(view)) = (self.core.active(), self.active_view()) else {
            return self.no_document(cx);
        };
        let read = view.read(cx);
        let picked = read.picked_mark();
        let (title, expect, path, page) = match (add, picked) {
            (true, picked) => {
                let n = (read.current + 1).to_string();
                (Self::text(cx, "pdf.page.label", &[("n", &n)]).to_string(), None, place_for(&read.marks, picked.as_ref()), read.current)
            }
            (false, Some(flat)) => (flat.title.clone(), Some(flat.title), flat.path, read.current),
            (false, None) => return,
        };
        let version = read.version;
        let field = cx.new(|cx| {
            let mut field = TextInput::new(window, cx);
            field.set_text(title, cx);
            field
        });
        self.naming = Some(Naming { tab, version, field, expect, path, page });
        cx.notify();
    }

    pub(crate) fn name_bookmark(&mut self, title: String, cx: &mut Context<Self>) {
        let Some(Naming { tab, version, expect, path, page, .. }) = self.naming.take() else {
            return;
        };
        cx.notify();
        let Some(view) = self.tab(tab).map(|tab| tab.view.clone()) else {
            return;
        };
        let change = match expect {
            None => BookmarkChange::Add { path, title, page, y: 0.0 },
            Some(expect) => BookmarkChange::Edit { path, expect, title: Some(title), to: None },
        };
        self.on_same(&view, version, cx, |view, cx| {
            view.edit("pdf.doing.change_marks", move |worker, doc| worker.change_bookmark(doc, &change), cx)
        });
    }

    pub fn delete_bookmark(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.active_view() else {
            return self.no_document(cx);
        };
        let (picked, version) = (view.read(cx).picked_mark(), view.read(cx).version);
        let Some(flat) = picked else {
            return;
        };
        let change = BookmarkChange::Delete { path: flat.path, expect: flat.title };
        self.on_same(&view, version, cx, |view, cx| {
            view.mark_pick = None;
            view.edit("pdf.doing.delete_mark", move |worker, doc| worker.change_bookmark(doc, &change), cx)
        });
    }

    /// The bookmark title prompt, while open.
    pub fn naming_element(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let naming = self.naming.as_ref()?;
        let (closed, sent) = (cx.entity(), cx.entity());
        let key = if naming.expect.is_some() { "pdf.mark.rename" } else { "pdf.mark.add" };
        let prompt = PromptDialog::new("pdf-mark-name", Self::text(cx, key, &[]), &naming.field, move |_, cx| {
            closed.update(cx, |viewer, cx| {
                viewer.naming = None;
                cx.notify();
            })
        })
        .label(Self::text(cx, "pdf.mark.title", &[]))
        .submit(Self::text(cx, "pdf.text.apply", &[]))
        .check({
            let empty = Self::text(cx, "pdf.mark.empty", &[]);
            move |title| if title.trim().is_empty() { Err(empty.clone()) } else { Ok(()) }
        })
        .on_submit(move |title, _, cx| sent.update(cx, |viewer, cx| viewer.name_bookmark(title.trim().to_string(), cx)));
        Some(prompt.into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark(title: &str, page: Option<usize>, down: Vec<Bookmark>) -> Bookmark {
        Bookmark { title: title.into(), page, y: 0.0, down }
    }

    #[test]
    fn bookmarks_flatten_with_levels_and_paths() {
        let marks = [mark("A", Some(0), vec![mark("A1", Some(1), vec![])]), mark("Web", None, vec![])];
        let mut out = Vec::new();
        flat(&marks, &[], &mut out);
        let short: Vec<(&str, usize, Vec<usize>)> = out.iter().map(|f| (f.title.as_str(), f.level, f.path.clone())).collect();
        assert_eq!(short, [("A", 1, vec![0]), ("A1", 2, vec![0, 0]), ("Web", 1, vec![1])]);
        assert_eq!(place_for(&marks, Some(&out[1])), [0, 1], "after the picked one");
        assert_eq!(place_for(&marks, None), [2], "else last");
    }
}
