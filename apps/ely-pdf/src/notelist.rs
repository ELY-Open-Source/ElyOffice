//! The annotation list in the side panel (PDF-05).

use ely_gpui_component::{
    feedback::EmptyState,
    lists::{ListItem, SelectableList},
    primitives::IconName,
};
use gpui::{
    AnyElement, App, Entity, InteractiveElement as _, IntoElement, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, div,
};

use crate::annotate::Pick;
use crate::content::Aim;
use crate::notes::kind_label;
use crate::view::PdfView;
use crate::viewer::Viewer;

fn key(page: usize, id: u64) -> String {
    format!("{page}:{id}")
}

fn parse(key: &str) -> Option<(usize, u64)> {
    let (page, id) = key.split_once(':')?;
    Some((page.parse().ok()?, id.parse().ok()?))
}

/// Every annotation, page by page; unread pages are asked.
pub fn list(view: &Entity<PdfView>, cx: &mut App) -> AnyElement {
    let count = view.read(cx).sizes.len();
    view.update(cx, |view, cx| view.ask_notes(0..count, cx));
    let read = view.read(cx);
    let selected = match read.aim {
        Some(Aim::Note(Pick::Annot { page, id })) => Some(key(page, id)),
        _ => None,
    };
    let mut rows = Vec::new();
    for (page, marks) in (0..count).filter_map(|page| Some((page, read.notes.get(&page)?))) {
        for annot in &marks.annots {
            let place = Viewer::text(cx, "pdf.page.label", &[("n", &(page + 1).to_string())]);
            let mut item = ListItem::new(("pdf-note", annot.id as usize), kind_label(cx, &annot.kind)).detail(place);
            if let Some(first) = annot.contents.lines().find(|line| !line.trim().is_empty()) {
                item = item.description(first.to_string());
            }
            rows.push((key(page, annot.id), item));
        }
    }
    let all_read = (0..count).all(|page| read.notes.contains_key(&page));
    if rows.is_empty() && all_read {
        return EmptyState::new("pdf-notes-none", IconName::File, Viewer::text(cx, "pdf.notes.none", &[])).into_any_element();
    }
    let total = rows.len();
    let picked = view.clone();
    let list = (rows.into_iter().enumerate())
        .fold(SelectableList::new("pdf-notes"), |list, (at, (key, item))| list.row(key, item.place(at + 1, total)))
        .selected(selected)
        .label(Viewer::text(cx, "pdf.side.notes", &[]))
        .on_change(move |keys, _, cx| {
            let Some((page, id)) = keys.first().and_then(|key| parse(key)) else {
                return;
            };
            picked.update(cx, |view, cx| view.pick_note(page, id, cx))
        });
    div().id("pdf-notes-scroll").size_full().overflow_y_scroll().child(list).into_any_element()
}

impl PdfView {
    /// Picks annotation `id` and shows where it is.
    pub fn pick_note(&mut self, page: usize, id: u64, cx: &mut gpui::Context<Self>) {
        // Gone with an edit since the list was drawn.
        let Some(top) = self.notes.get(&page).and_then(|m| m.annots.iter().find(|a| a.id == id)).map(|a| a.rect[1]) else {
            return;
        };
        self.aim = Some(Aim::Note(Pick::Annot { page, id }));
        self.open_note_panel(cx);
        self.show(page, top, cx);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn row_keys_name_page_and_object() {
        assert_eq!(super::parse(&super::key(3, 41)), Some((3, 41)));
        assert_eq!(super::parse("3"), None);
    }
}
