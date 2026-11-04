//! Find and replace in the shown file (MARK-06).

use std::ops::Range;

use ely_gpui_component::{
    editor::{FindOptions, FindWidget, find_all},
    forms::{InputEvent, TextInput},
    primitives::{Severity, raise},
    theme::ActiveTheme as _,
};
use ely_office::prelude::*;
use ely_ui::i18n::text;
use gpui::{
    AnyElement, AppContext as _, Context, Entity, Focusable as _, InteractiveElement as _,
    IntoElement, ParentElement as _, SharedString, Styled as _, Subscription, Window, div,
};

use crate::desk::Desk;
use crate::view::MarkView;

/// The find box over one tab, and what it found.
pub struct Find {
    tab: u64,
    query: Entity<TextInput>,
    replace: Option<Entity<TextInput>>,
    options: FindOptions,
    hits: Vec<Range<usize>>,
    current: Option<usize>,
    /// The pattern did not compile; why.
    error: Option<SharedString>,
    _query: Subscription,
}

impl Desk {
    pub fn on_find(
        &mut self,
        _: &ely_office::FindReplace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.core.active() else {
            let said = text(cx, "command.blocked.no_document", &[]);
            return self.toast(said, Severity::Warning, cx);
        };
        if let Some(find) = self.finding.as_ref().filter(|find| find.tab == tab) {
            return window.focus(&find.query.focus_handle(cx), cx);
        }
        self.close_find(window, cx);
        let query = cx.new(|cx| TextInput::new(window, cx));
        let subscription =
            cx.subscribe_in(&query, window, |desk, _, event, window, cx| match event {
                InputEvent::Changed => desk.count(cx),
                InputEvent::Submit => desk.step(true, window, cx),
                _ => {}
            });
        window.focus(&query.focus_handle(cx), cx);
        log::info!("mark: find opens on tab {tab}");
        self.finding = Some(Find {
            tab,
            query,
            replace: None,
            options: FindOptions::default(),
            hits: Vec::new(),
            current: None,
            error: None,
            _query: subscription,
        });
        cx.notify();
    }

    fn found_view(&self) -> Option<(Entity<MarkView>, &Find)> {
        let find = self.finding.as_ref()?;
        Some((self.tab(find.tab)?.view.clone(), find))
    }

    /// Matches again in the tab's text, marked in source.
    pub fn count(&mut self, cx: &mut Context<Self>) {
        let Some((view, find)) = self.found_view() else {
            return;
        };
        let query = find.query.read(cx).text().to_string();
        let options = find.options;
        let document = view.read(cx).document().clone();
        let found = find_all(&document.read(cx).text(cx), &query, options);
        let Some(find) = self.finding.as_mut() else {
            return;
        };
        (find.hits, find.error) = match found {
            Ok(hits) => (hits, None),
            Err(why) => (Vec::new(), Some(why.into())),
        };
        find.current = find.current.filter(|current| *current < find.hits.len());
        let (hits, current) = (find.hits.clone(), find.current);
        mark(&document, &hits, current, cx);
        cx.notify();
    }

    fn step(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some((view, find)) = self.found_view() else {
            return;
        };
        let count = find.hits.len();
        if count == 0 {
            return;
        }
        let current = match (find.current, forward) {
            (None, true) => 0,
            (None, false) => count - 1,
            (Some(at), true) => (at + 1) % count,
            (Some(at), false) => (at + count - 1) % count,
        };
        let range = find.hits[current].clone();
        let hits = find.hits.clone();
        if let Some(find) = self.finding.as_mut() {
            find.current = Some(current);
        }
        let document = view.read(cx).document().clone();
        mark(&document, &hits, Some(current), cx);
        document.update(cx, |document, cx| document.show_match(range, window, cx));
        cx.notify();
    }

    fn replace(&mut self, every: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some((view, find)) = self.found_view() else {
            return;
        };
        let with = find
            .replace
            .as_ref()
            .map_or(String::new(), |field| field.read(cx).text().to_string());
        let edits: Vec<(Range<usize>, String)> = match (every, find.current) {
            (true, _) => find
                .hits
                .iter()
                .map(|hit| (hit.clone(), with.clone()))
                .collect(),
            (false, Some(at)) => vec![(find.hits[at].clone(), with)],
            (false, None) => return self.step(true, window, cx),
        };
        if edits.is_empty() {
            return;
        }
        // The next match starts past the replaced text.
        let after = edits[0].0.start + edits[0].1.len();
        let document = view.read(cx).document().clone();
        if !document.update(cx, |document, cx| document.replace(edits, cx)) {
            let said = text(cx, "mark.preview.read_only", &[]);
            return self.toast(said, Severity::Info, cx);
        }
        self.count(cx);
        if every {
            return;
        }
        let Some(find) = self.finding.as_mut() else {
            return;
        };
        let next = find.hits.iter().position(|hit| hit.start >= after);
        find.current = next.or((!find.hits.is_empty()).then_some(0));
        let Some(range) = find.current.map(|at| find.hits[at].clone()) else {
            return;
        };
        let hits = find.hits.clone();
        mark(&document, &hits, find.current, cx);
        document.update(cx, |document, cx| document.show_match(range, window, cx));
    }

    fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(find) = self.finding.take() else {
            return;
        };
        if let Some(tab) = self.tab(find.tab) {
            let document = tab.view.read(cx).document().clone();
            mark(&document, &[], None, cx);
            window.focus(&tab.view.read(cx).focus.clone(), cx);
        }
        cx.notify();
    }

    /// The find box over the file, while open.
    pub fn find_box(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let find = self
            .finding
            .as_ref()
            .filter(|find| Some(find.tab) == self.core.active())?;
        let this = cx.entity();
        let mut widget = FindWidget::new("mark-find", &find.query, find.current, find.hits.len())
            .options(find.options)
            .on_options({
                let this = this.clone();
                move |options, _, cx| {
                    this.update(cx, |desk, cx| {
                        if let Some(find) = desk.finding.as_mut() {
                            find.options = options;
                        }
                        desk.count(cx);
                    })
                }
            })
            .on_step({
                let this = this.clone();
                move |forward, window, cx| {
                    this.update(cx, |desk, cx| desk.step(forward, window, cx))
                }
            })
            .on_replace({
                let this = this.clone();
                move |every, window, cx| this.update(cx, |desk, cx| desk.replace(every, window, cx))
            })
            .on_toggle_replace({
                let this = this.clone();
                move |show, window, cx| {
                    this.update(cx, |desk, cx| {
                        if let Some(find) = desk.finding.as_mut() {
                            find.replace = show.then(|| cx.new(|cx| TextInput::new(window, cx)));
                        }
                        cx.notify();
                    })
                }
            })
            .on_close(move |window, cx| this.update(cx, |desk, cx| desk.close_find(window, cx)));
        if let Some(field) = &find.replace {
            widget = widget.replace(field);
        }
        if let Some(error) = &find.error {
            widget = widget.error(error.clone());
        }
        let place = div()
            .absolute()
            .top_2()
            .right_4()
            .block_mouse_except_scroll()
            .child(widget);
        Some(raise("mark-find-layer", place).into_any_element())
    }
}

/// The matches marked in the source; the current one stronger.
fn mark(
    document: &Entity<ely_gpui_component::documents::MarkdownDocument>,
    hits: &[Range<usize>],
    current: Option<usize>,
    cx: &mut Context<Desk>,
) {
    let tone = cx.theme().colors.warning;
    let marks = hits
        .iter()
        .enumerate()
        .map(|(at, hit)| {
            let strength = if Some(at) == current { 0.45 } else { 0.2 };
            (hit.clone(), tone.opacity(strength))
        })
        .collect();
    let editor = document.read(cx).editor().clone();
    editor.update(cx, |editor, cx| editor.set_backgrounds(marks, cx));
}

#[cfg(test)]
mod tests;
