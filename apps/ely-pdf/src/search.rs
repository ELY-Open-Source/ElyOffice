//! Find in the whole document, pages in batches (PDF-01).

use ely_gpui_component::{
    editor::FindWidget,
    forms::{InputEvent, TextInput},
    primitives::Severity,
};
use ely_office::Tabs as _;
use gpui::{
    AnyElement, AppContext as _, Context, Entity, Focusable as _, IntoElement, ParentElement as _,
    Styled as _, Subscription, Window, div,
};

use crate::view::PdfView;
use crate::viewer::Viewer;

/// Pages searched in one worker call.
const BATCH: usize = 20;

/// The find box; its hits live in the tab's view.
pub struct Find {
    /// Each find box is new; late answers check it.
    pub id: u64,
    pub tab: u64,
    pub query: Entity<TextInput>,
    /// Pages searched so far.
    pub done: usize,
    /// Counts searches; an older batch is dropped.
    pub round: u64,
    /// The view's version searched; an edit starts over.
    pub version: u64,
    _query: Subscription,
}

/// A quad's bounding box: x0, y0, x1, y1.
pub fn bounds(quad: &[f32; 8]) -> [f32; 4] {
    let xs = [quad[0], quad[2], quad[4], quad[6]];
    let ys = [quad[1], quad[3], quad[5], quad[7]];
    let min = |v: [f32; 4]| v.into_iter().fold(f32::MAX, f32::min);
    let max = |v: [f32; 4]| v.into_iter().fold(f32::MIN, f32::max);
    [min(xs), min(ys), max(xs), max(ys)]
}

impl Viewer {
    /// The view of tab `id`.
    fn view_of(&self, id: u64) -> Option<Entity<PdfView>> {
        self.tab(id).map(|tab| tab.view.clone())
    }

    pub fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active() else {
            let text = Self::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        if let Some(find) = self.find.as_ref().filter(|find| find.tab == tab) {
            return window.focus(&find.query.focus_handle(cx), cx);
        }
        self.drop_find(cx);
        let query = cx.new(|cx| TextInput::new(window, cx));
        let subscription = cx.subscribe(&query, |viewer, _, event, cx| match event {
            InputEvent::Changed => viewer.search(cx),
            InputEvent::Submit => viewer.step(true, cx),
            _ => {}
        });
        window.focus(&query.focus_handle(cx), cx);
        self.finds += 1;
        let id = self.finds;
        self.find = Some(Find { id, tab, query, done: 0, round: 0, version: 0, _query: subscription });
        cx.notify();
    }

    /// Closes the find box; its marks go.
    pub fn drop_find(&mut self, cx: &mut Context<Self>) {
        if let Some(view) = self.find.take().and_then(|find| self.view_of(find.tab)) {
            view.update(cx, |view, cx| {
                (view.found, view.chosen) = (Vec::new(), None);
                cx.notify();
            });
        }
    }

    /// Starts over for the query now typed.
    pub fn search(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.find.as_ref().and_then(|find| self.view_of(find.tab)) else {
            return;
        };
        let version = view.read(cx).version;
        view.update(cx, |view, _| (view.found, view.chosen) = (Vec::new(), None));
        if let Some(find) = self.find.as_mut() {
            (find.round, find.done, find.version) = (find.round + 1, 0, version);
        }
        self.search_more(cx);
        cx.notify();
    }

    /// Searches the next batch of pages.
    fn search_more(&mut self, cx: &mut Context<Self>) {
        let Some(find) = self.find.as_ref() else {
            return;
        };
        let needle = find.query.read(cx).text().to_string();
        let (finder, id, round, from) = (find.id, find.tab, find.round, find.done);
        let Some(view) = self.view_of(id) else {
            return;
        };
        let (doc, pages) = (view.read(cx).doc, view.read(cx).sizes.len());
        if needle.is_empty() || from >= pages {
            return;
        }
        let to = (from + BATCH).min(pages);
        let this = cx.weak_entity();
        view.update(cx, |view, cx| {
            view.read(
                move |worker| {
                    (from..to)
                        .map(|page| Ok(worker.search(doc, page, &needle)?.into_iter().map(move |quad| (page, quad))))
                        .collect::<std::io::Result<Vec<_>>>()
                        .map(|pages| pages.into_iter().flatten().collect::<Vec<_>>())
                },
                move |view, found, cx| {
                    let live = move |viewer: &Viewer| viewer.find.as_ref().is_some_and(|find| (find.id, find.round) == (finder, round));
                    if !this.upgrade().is_some_and(|viewer| live(viewer.read(cx))) {
                        return;
                    }
                    let hits = match found {
                        Ok(hits) => hits,
                        Err(error) => return view.fail("pdf.doing.search", &error, cx),
                    };
                    view.found.extend(hits);
                    let first = view.chosen.is_none() && !view.found.is_empty();
                    cx.notify();
                    // The viewer reads this view; wait until it is free.
                    cx.defer(move |cx| {
                        let Some(viewer) = this.upgrade() else {
                            return;
                        };
                        viewer.update(cx, |viewer, cx| {
                            if !live(viewer) {
                                return;
                            }
                            if let Some(find) = viewer.find.as_mut() {
                                find.done = to;
                            }
                            if first {
                                viewer.step(true, cx);
                            }
                            viewer.search_more(cx);
                            cx.notify();
                        })
                    });
                },
                cx,
            )
        });
    }

    /// Goes to the next or previous hit.
    pub fn step(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some(find) = self.find.as_ref() else {
            return;
        };
        let Some(view) = self.view_of(find.tab) else {
            return;
        };
        view.update(cx, |view, cx| {
            let count = view.found.len();
            if count == 0 {
                return;
            }
            let at = match (view.chosen, forward) {
                (None, _) => 0,
                (Some(at), true) => (at + 1) % count,
                (Some(at), false) => (at + count - 1) % count,
            };
            view.chosen = Some(at);
            let (page, y) = (view.found[at].0, bounds(&view.found[at].1[0])[1]);
            view.show(page, y, cx);
        });
        cx.notify();
    }

    /// The find box over the pages, while open.
    pub fn find_bar(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let find = self.find.as_ref().filter(|find| Some(find.tab) == self.core.active())?;
        let view = self.view_of(find.tab)?;
        let (chosen, count) = (view.read(cx).chosen, view.read(cx).found.len());
        let (step, close) = (cx.entity(), cx.entity());
        let widget = FindWidget::new("pdf-find", &find.query, chosen, count)
            .on_step(move |forward, _, cx| step.update(cx, |viewer, cx| viewer.step(forward, cx)))
            .on_close(move |window, cx| {
                close.update(cx, |viewer, cx| {
                    viewer.drop_find(cx);
                    window.focus(&view.read(cx).focus.clone(), cx);
                    cx.notify();
                })
            });
        Some(div().flex().justify_end().px_3().py_1().child(widget).into_any_element())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_quad_bounds_its_four_corners() {
        let quad = [10.0, 20.0, 30.0, 20.0, 10.0, 32.0, 30.0, 32.0];
        assert_eq!(super::bounds(&quad), [10.0, 20.0, 30.0, 32.0]);
    }
}
