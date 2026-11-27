//! "Changes only": text order, replacements paired (#126).

use ely_gpui_component::{
    buttons::{ButtonVariant, IconButton},
    feedback::EmptyState,
    lists::{List, ListItem},
    primitives::IconName,
    theme::ControlSize,
};
use ely_lok::review::Change;
use gpui::{AnyElement, Context, IntoElement as _, ParentElement as _, SharedString, Styled as _, div};

use super::{Review, authors, nav};
use crate::docs::Docs;

/// One row: a change, or a deletion with its insertion.
#[derive(Debug, PartialEq)]
pub(super) struct Row<'a> {
    pub first: &'a Change,
    pub then: Option<&'a Change>,
}

/// Changes in text order; touching delete and insert pair.
pub(super) fn rows<'a>(changes: impl IntoIterator<Item = &'a Change>) -> Vec<Row<'a>> {
    let mut sorted: Vec<&Change> = changes.into_iter().collect();
    sorted.sort_by_key(|change| (change.node, change.from));
    let mut rows = Vec::new();
    let mut at = 0;
    while let Some(&first) = sorted.get(at) {
        let then = sorted.get(at + 1).copied().filter(|next| pairs(first, next));
        at += 1 + usize::from(then.is_some());
        rows.push(Row { first, then });
    }
    rows
}

/// One deletion and one insertion, end to start.
fn pairs(first: &Change, next: &Change) -> bool {
    let kinds = [first.kind.as_str(), next.kind.as_str()];
    let replaced = kinds == ["Delete", "Insert"] || kinds == ["Insert", "Delete"];
    replaced && first.node == next.node && first.to >= 0 && first.to == next.from
}

impl Row<'_> {
    /// The deleted and inserted words of a pair.
    pub(super) fn replaced(&self) -> Option<(&str, &str)> {
        let then = self.then?;
        let (old, new) = if self.first.kind == "Delete" { (self.first, then) } else { (then, self.first) };
        Some((old.words.as_str(), new.words.as_str()))
    }
}

impl Review {
    /// Each change once, a replacement as old to new.
    pub(super) fn only_list(&self, cx: &mut Context<Self>) -> AnyElement {
        let rows = rows(authors::shown(&self.changes, self.who.as_deref()));
        if rows.is_empty() {
            let none = Docs::text(cx, "doc.review.none", &[]);
            return EmptyState::new("doc-review-only-none", IconName::Check, none).into_any_element();
        }
        let this = cx.entity();
        let (accept, reject) = (Docs::text(cx, "doc.review.accept", &[]), Docs::text(cx, "doc.review.reject", &[]));
        let items = rows.iter().map(|row| {
            let id = row.first.index as usize;
            // A pair is decided whole.
            let indexes: Vec<String> = [Some(row.first), row.then].into_iter().flatten().map(|c| c.index.to_string()).collect();
            let decide = |command: &'static str, icon, tip: &SharedString| {
                let (this, indexes) = (this.clone(), indexes.clone());
                IconButton::new((command, id), icon)
                    .size(ControlSize::Sm)
                    .variant(ButtonVariant::Ghost)
                    .tooltip(tip.clone())
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        this.update(cx, |review, cx| {
                            for index in indexes.iter().rev() {
                                review.run(command, &[index], cx);
                            }
                        })
                    })
            };
            let title = match row.replaced() {
                Some((old, new)) => Docs::text(cx, "doc.review.only.replaced", &[("old", old), ("new", new)]),
                None => row.first.words.clone().into(),
            };
            let said = row.first.paragraph.as_deref().map(context).filter(|text| !text.is_empty());
            let said = said.map(|text| Docs::text(cx, "doc.review.only.context", &[("text", &text)]));
            let lines = [Some(nav::tip(row.first, cx)), said].into_iter().flatten().collect::<Vec<_>>().join(" · ");
            let actions =
                div().flex().gap_1().child(decide("track.accept", IconName::Check, &accept)).child(decide("track.reject", IconName::X, &reject));
            let mut item = ListItem::new(("doc-review-only", id), title).description(lines).trailing(actions);
            if let Some(anchor) = row.first.anchor {
                let this = this.clone();
                item = item.on_click(move |_, _, cx| this.update(cx, |review, cx| review.reveal(anchor, cx)));
            }
            item
        });
        List::new().divided().children(items).into_any_element()
    }
}

/// The paragraph's start, kernel marks left out.
pub(super) fn context(paragraph: &str) -> String {
    paragraph.chars().filter(|c| !c.is_control()).collect::<String>().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(index: u64, kind: &str, (node, from, to): (i64, i64, i64), words: &str) -> Change {
        Change {
            index,
            author: "Ann".into(),
            kind: kind.into(),
            description: String::new(),
            words: words.into(),
            at: String::new(),
            anchor: None,
            ranges: Vec::new(),
            node,
            from,
            to,
            paragraph: None,
        }
    }

    #[test]
    fn touching_deletes_and_inserts_pair_in_text_order() {
        let changes = [
            change(3, "Insert", (9, 0, 4), "late"),
            change(1, "Delete", (5, 4, 5), "5"),
            change(2, "Insert", (5, 5, 7), "10"),
            change(4, "Delete", (5, 20, 25), "alone"),
            change(5, "Insert", (5, 30, 33), "far"),
        ];
        let rows = rows(&changes);
        let firsts: Vec<u64> = rows.iter().map(|row| row.first.index).collect();
        assert_eq!(firsts, [1, 4, 5, 3], "text order, the pair one row");
        assert_eq!(rows[0].replaced(), Some(("5", "10")));
        assert_eq!(rows[1].then, None, "apart, they stay apart");
        assert_eq!(rows[2].then, None);
    }

    #[test]
    fn an_insert_before_its_delete_still_reads_old_to_new() {
        let changes = [change(1, "Insert", (2, 0, 2), "10"), change(2, "Delete", (2, 2, 3), "5")];
        assert_eq!(rows(&changes)[0].replaced(), Some(("5", "10")));
    }

    #[test]
    fn a_change_across_paragraphs_pairs_with_nothing() {
        let changes = [change(1, "Delete", (2, 3, -1), "two lines"), change(2, "Insert", (3, 0, 1), "x")];
        assert!(rows(&changes).iter().all(|row| row.then.is_none()));
        assert_eq!(context("\u{1}Clause 27 reads"), "Clause 27 reads");
    }
}
