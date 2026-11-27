//! Previous and next change or comment; who changed what (#122).

use ely_lok::{Rect, review::Change};
use gpui::{App, Context, SharedString};

use super::Review;
use crate::docs::Docs;

/// Whether rect `a` starts before rect `b` in reading order.
fn before(a: Rect, b: Rect) -> bool {
    // Lines that overlap are one line: left to right.
    let same_line = a.1 < b.1 + b.3 && b.1 < a.1 + a.3;
    if same_line { a.0 < b.0 } else { a.1 < b.1 }
}

/// Whether `caret` sits in one of `ranges`, edges included.
pub(super) fn holds(ranges: &[Rect], caret: Rect) -> bool {
    ranges.iter().any(|r| {
        caret.0 >= r.0 && caret.0 <= r.0 + r.2 && caret.1 < r.1 + r.3 && r.1 < caret.1 + caret.3.max(1)
    })
}

/// Where to go from `current`, else `caret`; wraps around.
pub(super) fn step(spots: &[(Rect, bool)], caret: Option<Rect>, forward: bool) -> Option<usize> {
    let count = spots.len();
    if count == 0 {
        return None;
    }
    if let Some(at) = spots.iter().position(|(_, current)| *current) {
        return Some(if forward { (at + 1) % count } else { (at + count - 1) % count });
    }
    let Some(caret) = caret else {
        return Some(if forward { 0 } else { count - 1 });
    };
    let found = if forward {
        spots.iter().position(|(spot, _)| before(caret, *spot))
    } else {
        spots.iter().rposition(|(spot, _)| before(*spot, caret))
    };
    Some(found.unwrap_or(if forward { 0 } else { count - 1 }))
}

/// The hover words for a change: who, and what kind.
fn tip(change: &Change, cx: &App) -> SharedString {
    let key = match change.kind.as_str() {
        "Insert" => "doc.review.by.insert",
        "Delete" => "doc.review.by.delete",
        "Format" | "ParagraphFormat" => "doc.review.by.format",
        _ => "doc.review.by.other",
    };
    Docs::text(cx, key, &[("author", change.author.as_str())])
}

impl Review {
    /// Selects the next or previous change in the text.
    pub(super) fn step_change(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some(shown) = &self.shown else {
            return;
        };
        let caret = shown.view.read(cx).cursor();
        let spots: Vec<(Rect, bool)> = super::authors::shown(&self.changes, self.who.as_deref())
            .filter_map(|change| {
                let held = caret.is_some_and(|caret| holds(&change.ranges, caret));
                change.anchor.map(|anchor| (anchor, held))
            })
            .collect();
        let placed: Vec<&Change> = super::authors::shown(&self.changes, self.who.as_deref())
            .filter(|change| change.anchor.is_some())
            .collect();
        let Some(at) = step(&spots, caret, forward) else {
            return;
        };
        let id = placed[at].index.to_string();
        log::info!("doc: review goes to change {id}");
        self.run("track.goto", &[&id], cx);
    }

    /// Picks the next or previous comment thread in the text.
    pub(super) fn step_comment(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some(shown) = &self.shown else {
            return;
        };
        let caret = shown.view.read(cx).cursor();
        let mut roots: Vec<_> = self.notes.iter().filter(|note| note.parent == "0").collect();
        roots.sort_by(|a, b| {
            if before(a.anchor, b.anchor) {
                std::cmp::Ordering::Less
            } else if before(b.anchor, a.anchor) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        let active = self.active.as_deref();
        let spots: Vec<(Rect, bool)> = roots.iter().map(|note| (note.anchor, Some(note.id.as_str()) == active)).collect();
        if let Some(at) = step(&spots, caret, forward) {
            let key = SharedString::from(roots[at].id.clone());
            self.select(&key, cx);
        }
    }

    /// Each change's words say who made it, on hover.
    pub(super) fn tip_changes(&self, cx: &mut Context<Self>) {
        let Some(shown) = &self.shown else {
            return;
        };
        let tips = self
            .changes
            .iter()
            .flat_map(|change| {
                let words = tip(change, cx);
                change.ranges.iter().map(move |rect| (*rect, words.clone()))
            })
            .collect();
        shown.view.update(cx, |view, cx| view.tip_rects(tips, cx));
    }
}

impl Review {
    /// Previous and next buttons for the pane shown.
    pub(super) fn steps(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        use ely_gpui_component::{buttons::{ButtonVariant, IconButton}, primitives::IconName, theme::ControlSize};
        use gpui::{IntoElement as _, ParentElement as _, Styled as _, div};
        let changes = self.pane == super::Pane::Changes;
        let (prev, next, why, none) = match changes {
            true => ("doc.review.prev_change", "doc.review.next_change", "doc.why.no_changes", self.changes.is_empty()),
            false => ("doc.review.prev_comment", "doc.review.next_comment", "doc.why.no_comments", self.threads.is_empty()),
        };
        let this = cx.entity();
        let button = |id: &'static str, icon, key: &str, forward: bool| {
            let this = this.clone();
            let tip = Docs::text(cx, if none { why } else { key }, &[]);
            IconButton::new(id, icon)
                .size(ControlSize::Sm)
                .variant(ButtonVariant::Ghost)
                .disabled(none)
                .tooltip(tip)
                .on_click(move |_, _, cx| {
                    this.update(cx, |review, cx| match changes {
                        true => review.step_change(forward, cx),
                        false => review.step_comment(forward, cx),
                    })
                })
        };
        div()
            .flex()
            .gap_1()
            .child(button("doc-review-prev", IconName::ChevronLeft, prev, false))
            .child(button("doc-review-next", IconName::ChevronRight, next, true))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::step;

    #[test]
    fn steps_follow_reading_order_and_wrap() {
        let (a, b, c) = ((0, 0, 10, 10), (20, 0, 10, 10), (0, 20, 10, 10));
        let none = [(a, false), (b, false), (c, false)];
        assert_eq!(step(&[], None, true), None, "nothing to go to");
        assert_eq!((step(&none, None, true), step(&none, None, false)), (Some(0), Some(2)));
        // A caret between a and b on one line.
        let caret = Some((15, 2, 0, 8));
        assert_eq!((step(&none, caret, true), step(&none, caret, false)), (Some(1), Some(0)));
        // Past the last one, forward wraps to the first.
        assert_eq!(step(&none, Some((50, 30, 0, 8)), true), Some(0));
        assert_eq!(step(&none, Some((0, 0, 0, 0)), false), Some(2), "before the first wraps back");
        let at_last = [(a, false), (b, false), (c, true)];
        assert_eq!((step(&at_last, caret, true), step(&at_last, caret, false)), (Some(0), Some(1)));
    }
}
