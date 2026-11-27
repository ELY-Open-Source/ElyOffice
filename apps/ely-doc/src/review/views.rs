//! Markup, Original or Final display of tracked changes (#126).

use std::sync::Arc;

use ely_gpui_component::{buttons::SegmentedControl, forms::Switch, theme::ControlSize};
use ely_lok::{Doc, LokError, LokView};
use gpui::{AnyElement, App, Context, Entity, IntoElement as _, ParentElement as _, Styled as _, div, prelude::FluentBuilder as _};

use super::{Review, original};
use crate::docs::Docs;

/// The Original view's read-only copy and its view.
pub(super) struct Original {
    pub(super) view: Entity<LokView>,
    pub(super) doc: Doc,
}

impl Review {
    /// What the page draws in place of the tab's view.
    pub(crate) fn original_view(&self) -> Option<Entity<LokView>> {
        self.original.as_ref().map(|original| original.view.clone())
    }

    /// The picked display, read from the kernel's state.
    pub(super) fn showing(&self, cx: &App) -> &'static str {
        if self.original.is_some() || self.preparing {
            return "original";
        }
        if self.only {
            return "changes";
        }
        let state = self.shown.as_ref().and_then(|shown| shown.view.read(cx).state(".uno:ShowTrackedChanges"));
        match state {
            Some("false") => "final",
            _ => "markup",
        }
    }

    /// The display picker over the changes.
    pub(super) fn views(&self, cx: &mut Context<Self>) -> AnyElement {
        let this = cx.entity();
        let picker = SegmentedControl::new("doc-review-view", self.showing(cx))
            .size(ControlSize::Sm)
            .segment("markup", Docs::text(cx, "doc.review.view.markup", &[]), None)
            .segment("original", Docs::text(cx, "doc.review.view.original", &[]), None)
            .segment("final", Docs::text(cx, "doc.review.view.final", &[]), None)
            .segment("changes", Docs::text(cx, "doc.review.view.changes", &[]), None)
            .on_change({
                let this = this.clone();
                move |value, _, cx| this.update(cx, |review, cx| review.show_as(value.as_ref(), cx))
            });
        let row = div()
            .flex()
            .items_center()
            .gap_2()
            .child(Docs::text(cx, "doc.review.view", &[]))
            .child(picker);
        // Deletions leave the text unless struck in place (#143).
        let hide = self.shown.as_ref().and_then(|shown| shown.view.read(cx).state(".uno:HideTrackedDeletions"));
        let inline = Switch::new("doc-review-inline", hide == Some("false"))
            .disabled(hide.is_none())
            .label(Docs::text(cx, "doc.review.view.inline", &[]))
            .on_change(move |on, _, cx| this.update(cx, |review, cx| review.strike_inline(on, cx)));
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(row)
            .when(self.showing(cx) == "markup", |it| it.child(inline))
            .into_any_element()
    }

    /// Switches the display; the document keeps every change.
    pub(super) fn show_as(&mut self, how: &str, cx: &mut Context<Self>) {
        if how == self.showing(cx) {
            return;
        }
        log::info!("doc: review shows {how}");
        self.only = how == "changes";
        if how == "original" {
            return self.prepare_original(cx);
        }
        self.drop_original(cx);
        let want = if how == "final" { "false" } else { "true" };
        let state = self.shown.as_ref().and_then(|shown| shown.view.read(cx).state(".uno:ShowTrackedChanges"));
        if state != Some(want) {
            self.run("track.shown", &[want], cx);
        }
        cx.notify();
    }

    /// Strikes deletions in the text, or takes them out.
    pub(super) fn strike_inline(&mut self, on: bool, cx: &mut Context<Self>) {
        log::info!("doc: deletions in the text {on}");
        ely_ui::settings::switches::set(ely_ui::settings::switches::STRUCK, on, cx);
        self.run("track.hide", &[crate::deletions::wanted(cx)], cx);
    }

    /// Opens the copy off the UI thread, then shows it.
    fn prepare_original(&mut self, cx: &mut Context<Self>) {
        let Some(shown) = &self.shown else {
            return;
        };
        self.preparing = true;
        cx.notify();
        let (work, reader) = (shown.work.clone(), shown.view.entity_id());
        ely_lok::read_off(
            &self.worker,
            &shown.doc,
            move |worker, doc| original::prepare(worker, doc, &work),
            move |review: &mut Self, prepared, cx| {
                let wanted = std::mem::take(&mut review.preparing);
                let current = review.shown.as_ref().map(|shown| shown.view.entity_id()) == Some(reader);
                match prepared {
                    Ok(ready) if wanted && current => {
                        let original::Prepared { doc, laid, feed, waiter } = ready;
                        let view = LokView::open(Arc::clone(&review.worker), doc.clone(), laid, (feed, waiter), cx);
                        review.original = Some(Original { view, doc });
                    }
                    // Switched away while it opened.
                    Ok(ready) => review.close_copy(ready.doc, cx),
                    Err(error) => {
                        let said = Docs::text(cx, "doc.review.view.failed", &[("error", &error.to_string())]);
                        cx.emit(LokError(said.to_string()));
                    }
                }
                cx.notify();
            },
            cx,
        );
    }

    /// Back to the tab's own view; the copy closes.
    pub(super) fn drop_original(&mut self, cx: &mut Context<Self>) {
        self.preparing = false;
        if let Some(original) = self.original.take() {
            self.close_copy(original.doc, cx);
            cx.notify();
        }
    }

    fn close_copy(&mut self, doc: Doc, cx: &mut Context<Self>) {
        log::info!("doc: the original copy closes");
        ely_lok::read_off(
            &self.worker,
            &doc,
            |worker, doc| worker.close_doc(doc.clone()),
            |_: &mut Self, closed, cx| {
                if let Err(error) = closed {
                    cx.emit(LokError(format!("the original copy did not close: {error}")));
                }
            },
            cx,
        );
    }
}
