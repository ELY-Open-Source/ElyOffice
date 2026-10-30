//! Tabs, page, status line and side panel.

use ely_gpui_component::{
    feedback::Banner,
    primitives::{IconName, Severity},
    shell::{StatusBar, StatusBarItem},
};
use ely_lok::LokView;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, AnyView, Context, Entity, IntoElement, ParentElement as _, Styled as _, div,
};

use ely_office::prelude::*;

use crate::docs::{Docs, Side};

impl Docs {
    /// The save state, and the kernel's word count.
    pub(crate) fn status(&self, cx: &mut Context<Self>) -> StatusBar {
        let state = self.save_state(cx);
        let status = StatusBar::new().left(StatusBarItem::new("doc-state").label(state));
        let zoom = self.zoom_control(cx);
        let Some(counts) = self.active_tab().and_then(|tab| tab.extra.counts) else {
            return status.when_some(zoom, |status, zoom| status.right(zoom));
        };
        // The kernel counts selected words only; none reads as none.
        let selected = (counts.selected_words > 0).then_some(counts.selected_words);
        let label = ely_ui::i18n::word_count(cx, counts.words, counts.characters, selected);
        status.right(StatusBarItem::new("doc-count").label(label)).when_some(zoom, |status, zoom| status.right(zoom))
    }

    pub(crate) fn body(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Some(tab) = self.active_tab() {
            let view = div()
                .relative()
                .size_full()
                .child(tab.view.clone())
                .children(self.find_box(cx))
                .children(self.context_menu(cx))
                .into_any_element();
            let risk = self.stopped_banner(cx).or_else(|| self.risk_banner(cx));
            let fonts = tab.view.read(cx).missing_fonts().join(", ");
            let fonts = (!fonts.is_empty()).then(|| {
                let warning = Self::text(cx, "doc.fonts.missing", &[("fonts", &fonts)]);
                Banner::new("doc-fonts", Severity::Warning, warning)
            });
            if risk.is_none() && fonts.is_none() {
                return view;
            }
            return div()
                .size_full()
                .flex()
                .flex_col()
                .children(risk)
                .children(fonts)
                .child(div().flex_1().min_h_0().child(view))
                .into_any_element();
        }
        self.empty_body("doc-empty", IconName::FileText, cx)
    }

    /// The navigator column, while shown.
    pub(crate) fn left_view(&mut self, cx: &mut Context<Self>) -> Option<AnyView> {
        let shown = self
            .active_tab()
            .map(|tab| (tab.view.clone(), tab.doc.clone()));
        // Hidden, it follows nothing: no reads on every move.
        let open = ely_ui::settings::panels::shown(true, cx);
        let tab = shown.as_ref().filter(|_| open).map(|(view, doc)| (view, doc));
        self.navigator
            .update(cx, |navigator, cx| navigator.show(tab, cx));
        shown?;
        Some(self.navigator.clone().into())
    }

    /// The right column for the active tab.
    pub(crate) fn side_view(&mut self, cx: &mut Context<Self>) -> Option<AnyView> {
        let shown = self
            .active_tab()
            .map(|tab| (tab.view.clone(), tab.doc.clone()));
        let tab = shown.as_ref().map(|(view, doc)| (view, doc));
        self.props.update(cx, |props, cx| props.show(tab, cx));
        self.review.update(cx, |review, cx| review.show(tab, cx));
        shown.as_ref()?;
        Some(match self.side {
            Side::Props => self.props.clone().into(),
            Side::Review => self.review.clone().into(),
            Side::Merge => self.merge.clone().into(),
        })
    }

    /// Reads the counts, again when the kernel's count moves.
    pub(crate) fn count(&mut self, id: u64, view: &Entity<LokView>, cx: &mut Context<Self>) {
        let state = view
            .read(cx)
            .state(".uno:StateWordCount")
            .map(str::to_string);
        let Some(tab) = self.tab(id) else {
            return;
        };
        // The kernel reports its count only once the status moves.
        let asked = tab.extra.counted.as_ref() == Some(&state);
        if tab.extra.counting || asked {
            return;
        }
        let doc = tab.doc.clone();
        // Asked now: a failure waits for the next change.
        let extra = &mut self.tab_mut(id).expect("the tab is open").extra;
        (extra.counting, extra.counted) = (true, Some(state));
        ely_lok::read_off(
            &self.core.worker,
            &doc,
            |worker, doc| {
                let json = worker.values(doc, ".uno:WordCount")?;
                ely_lok::review::counts(&json)
            },
            move |docs: &mut Self, read, cx| {
                let Some(tab) = docs.tab_mut(id) else {
                    return;
                };
                tab.extra.counting = false;
                match read {
                    Ok(counts) => tab.extra.counts = Some(counts),
                    Err(error) => docs.engine_failed(&error.to_string(), cx),
                }
                cx.notify();
                // The status may have moved meanwhile.
                let view = docs.tab(id).map(|tab| tab.view.clone());
                if let Some(view) = view {
                    docs.count(id, &view, cx);
                }
            },
            cx,
        );
    }
}
