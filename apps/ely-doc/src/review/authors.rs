//! Tracked changes by reviser: counts and the filter (#122).

use ely_gpui_component::{forms::{Choice, Select}, theme::ControlSize};
use ely_lok::review::Change;
use gpui::{AnyElement, Context, IntoElement as _, SharedString};

use super::Review;
use crate::docs::Docs;

/// The picker's value for every reviser.
const ALL: &str = "all";

/// Each reviser and how many changes they made, by name.
pub(super) fn authors(changes: &[Change]) -> Vec<(String, usize)> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for change in changes {
        match counts.iter_mut().find(|(name, _)| *name == change.author) {
            Some((_, n)) => *n += 1,
            None => counts.push((change.author.clone(), 1)),
        }
    }
    counts.sort_by(|a, b| a.0.cmp(&b.0));
    counts
}

/// The changes `who` made; every change without a filter.
pub(super) fn shown<'a>(changes: &'a [Change], who: Option<&'a str>) -> impl Iterator<Item = &'a Change> {
    changes.iter().filter(move |change| who.is_none_or(|who| change.author == who))
}

impl Review {
    /// Each reviser with a count; picking one filters the list.
    pub(super) fn revisers(&self, cx: &Context<Self>) -> AnyElement {
        let this = cx.entity();
        let total = self.changes.len().to_string();
        let mut choices = vec![Choice::new(SharedString::from(ALL), Docs::text(cx, "doc.review.reviser.all", &[("n", &total)]))];
        for (name, n) in authors(&self.changes) {
            let words = Docs::text(cx, "doc.review.reviser.each", &[("author", &name), ("n", &n.to_string())]);
            choices.push(Choice::new(SharedString::from(format!("a:{name}")), words));
        }
        let picked = self.who.as_ref().map_or(ALL.to_string(), |who| format!("a:{who}"));
        Select::new("doc-review-reviser", choices)
            .label(Docs::text(cx, "doc.review.reviser", &[]))
            .size(ControlSize::Sm)
            .selected(SharedString::from(picked))
            .on_change(move |value, _, cx| {
                let who = value.strip_prefix("a:").map(str::to_string);
                this.update(cx, |review, cx| {
                    log::info!("doc: review filter {}", if who.is_some() { "on" } else { "off" });
                    review.who = who;
                    cx.notify();
                })
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{authors, shown};
    use ely_lok::review::Change;

    fn change(index: u64, author: &str) -> Change {
        Change {
            index,
            author: author.into(),
            kind: "Insert".into(),
            description: String::new(),
            words: String::new(),
            at: String::new(),
            anchor: None,
            ranges: Vec::new(),
            node: 0,
            from: 0,
            to: 0,
            paragraph: None,
        }
    }

    #[test]
    fn revisers_are_counted_and_filtered() {
        let changes = [change(1, "Bo"), change(2, "Ann"), change(3, "Bo"), change(4, "")];
        let counts = authors(&changes);
        assert_eq!(counts, [(String::new(), 1), ("Ann".into(), 1), ("Bo".into(), 2)]);
        let bo: Vec<u64> = shown(&changes, Some("Bo")).map(|change| change.index).collect();
        assert_eq!(bo, [1, 3], "only Bo's, in list order");
        assert_eq!(shown(&changes, None).count(), 4, "no filter shows all");
        assert_eq!(shown(&changes, Some("Nobody")).count(), 0);
    }
}
