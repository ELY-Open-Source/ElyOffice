//! Outline, pages and bookmarks beside the page (DOC-16).

use std::sync::{Arc, Mutex};

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    layout::ScrollArea,
    lists::{List, ListItem, Outline},
    primitives::{IconName, Severity},
    theme::ControlSize,
};
use ely_lok::{
    Doc, LokError, LokView, Rect, Reported, Worker,
    review::{Bookmark, Heading},
};
use gpui::{
    App, Context, Entity, EventEmitter, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Subscription, WeakEntity, Window, actions, div,
};

use crate::docs::Docs;
use ely_office::prelude::*;

actions!(doc, [GoBack]);

#[derive(Clone, Copy, PartialEq)]
enum Pane {
    Outline,
    Pages,
    Marks,
}

struct Shown {
    view: Entity<LokView>,
    doc: Doc,
    /// Edits and cursor at the last read.
    read: Option<(u64, Option<Rect>)>,
    /// A read is on its way.
    reading: bool,
    _observe: Subscription,
}

pub struct Navigator {
    docs: WeakEntity<Docs>,
    worker: Arc<Mutex<Worker>>,
    shown: Option<Shown>,
    outline: Vec<Heading>,
    marks: Vec<Bookmark>,
    pane: Pane,
}

impl EventEmitter<LokError> for Navigator {}

impl Navigator {
    pub fn new(docs: WeakEntity<Docs>, worker: Arc<Mutex<Worker>>) -> Self {
        Navigator {
            docs,
            worker,
            shown: None,
            outline: Vec::new(),
            marks: Vec::new(),
            pane: Pane::Outline,
        }
    }

    /// Follows the active tab; reads the lists after edits.
    pub fn show(&mut self, tab: Option<(&Entity<LokView>, &Doc)>, cx: &mut Context<Self>) {
        let id = tab.map(|(view, _)| view.entity_id());
        if self.shown.as_ref().map(|shown| shown.view.entity_id()) == id {
            return;
        }
        self.shown = None;
        self.outline.clear();
        self.marks.clear();
        cx.notify();
        let Some((view, doc)) = tab else {
            return;
        };
        let observe = cx.subscribe(view, |navigator, _, _: &Reported, cx| navigator.refresh(cx));
        self.shown = Some(Shown {
            view: view.clone(),
            doc: doc.clone(),
            read: None,
            reading: false,
            _observe: observe,
        });
        self.refresh(cx);
    }

    /// Rereads the lists after edits or moves.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(shown) = self.shown.as_mut() else {
            return;
        };
        let view = shown.view.read(cx);
        let seen = Some((view.edited(), view.cursor()));
        if shown.reading || shown.read == seen {
            return;
        }
        (shown.read, shown.reading) = (seen, true);
        let reader = shown.view.entity_id();
        ely_lok::read_off(
            &self.worker,
            &shown.doc,
            |worker, doc| {
                let json = worker.values(doc, ".uno:Outline")?;
                let outline = ely_lok::review::outline(&json)?;
                let json = worker.values(doc, ".uno:Bookmarks")?;
                Ok((outline, ely_lok::review::bookmarks(&json)?))
            },
            move |navigator: &mut Self, read, cx| {
                let Some(shown) = navigator.shown.as_mut() else {
                    return;
                };
                if shown.view.entity_id() != reader {
                    return;
                }
                shown.reading = false;
                navigator.take_lists(read, cx);
                navigator.refresh(cx);
            },
            cx,
        );
    }

    fn take_lists(
        &mut self,
        read: std::io::Result<(Vec<Heading>, Vec<Bookmark>)>,
        cx: &mut Context<Self>,
    ) {
        match read {
            Ok((outline, marks)) => {
                let (placed, hidden): (Vec<_>, Vec<_>) =
                    marks.into_iter().partition(|mark| mark.rect.is_some());
                log::info!(
                    "doc: {} headings, {} bookmarks, {} not laid out",
                    outline.len(),
                    placed.len(),
                    hidden.len()
                );
                self.outline = outline;
                self.marks = placed;
            }
            Err(error) => cx.emit(LokError(format!(
                "the navigator lists are unreadable: {error}"
            ))),
        }
        cx.notify();
    }

    fn jump(docs: &WeakEntity<Docs>, to: Rect, cx: &mut App) {
        docs.update(cx, |docs, cx| docs.jump(to, cx))
            .expect("the documents live");
    }

    /// The page that holds the cursor.
    fn current_page(&self, pages: &[Rect], cx: &App) -> Option<usize> {
        let cursor = self.shown.as_ref()?.view.read(cx).cursor()?;
        pages.iter().rposition(|page| page.1 <= cursor.1)
    }
}

impl Docs {
    /// Goes to `to`; Back returns to where the cursor was.
    pub(crate) fn jump(&mut self, to: Rect, cx: &mut Context<Self>) {
        self.mark_place(cx);
        self.go(to, cx);
    }

    /// Remembers the place of the active tab for Back.
    pub(crate) fn mark_place(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let view = tab.view.read(cx);
        let Some(cursor) = view.cursor() else {
            return;
        };
        // A jump the cursor has not caught up with.
        let place = match self.going {
            Some((id, seq, moves, to))
                if id == tab.id && (!view.reached(seq) || view.moves() == moves) =>
            {
                to
            }
            _ => cursor,
        };
        self.back.push((tab.id, place));
        cx.notify();
    }

    /// Puts the active tab's cursor at `to`.
    fn go(&mut self, to: Rect, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let (id, view) = (tab.id, tab.view.clone());
        let moves = view.read(cx).moves();
        let seq = view.update(cx, |view, cx| {
            view.reveal(to, cx);
            view.sent()
        });
        self.going = Some((id, seq, moves, to));
    }

    pub fn on_back(&mut self, _: &GoBack, _: &mut Window, cx: &mut Context<Self>) {
        let Some((id, place)) = self.back.pop() else {
            return;
        };
        if self.tab(id).is_none() {
            let text = Docs::text(cx, "doc.nav.gone", &[]);
            return self.toast(text, Severity::Warning, cx);
        }
        log::info!("doc: back to tab {id}");
        self.core.active = Some(id);
        self.go(place, cx);
        cx.notify();
    }
}

impl Render for Navigator {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let text = |key: &str| Docs::text(cx, key, &[]);
        let docs = self.docs.clone();
        let back = docs
            .upgrade()
            .is_some_and(|docs| !docs.read(cx).back.is_empty());
        let head = Button::new("doc-nav-back", text("doc.nav.back"))
            .size(ControlSize::Sm)
            .variant(ButtonVariant::Ghost)
            .icon(IconName::ArrowLeft)
            .disabled(!back)
            .on_click(move |_, window, cx| window.dispatch_action(Box::new(GoBack), cx));
        let pane = SegmentedControl::new(
            "doc-nav-pane",
            match self.pane {
                Pane::Outline => "outline",
                Pane::Pages => "pages",
                Pane::Marks => "marks",
            },
        )
        .size(ControlSize::Sm)
        .segment("outline", text("doc.nav.outline"), Some(IconName::List))
        .segment("pages", text("doc.nav.pages"), Some(IconName::FileText))
        .segment("marks", text("doc.nav.marks"), Some(IconName::Bookmark))
        .on_change({
            let this = this.clone();
            move |value, _, cx| {
                this.update(cx, |navigator, cx| {
                    navigator.pane = match value.as_ref() {
                        "pages" => Pane::Pages,
                        "marks" => Pane::Marks,
                        _ => Pane::Outline,
                    };
                    cx.notify();
                })
            }
        });
        let body = match self.pane {
            Pane::Outline => {
                let mut outline = Outline::new("doc-nav-outline");
                for (at, heading) in self.outline.iter().enumerate() {
                    let level = usize::try_from(heading.level).expect("a level") + 1;
                    outline = outline.item(at.to_string(), heading.text.clone(), level);
                }
                if let Some(at) = self.outline.iter().position(|heading| heading.current) {
                    outline = outline.current(at.to_string());
                }
                // The places shown: a reread may change the list.
                let places: Vec<_> = self.outline.iter().map(|heading| heading.rect).collect();
                let docs = self.docs.clone();
                outline
                    .on_select(move |key, _, cx| {
                        let at: usize = key.parse().expect("an outline index");
                        Navigator::jump(&docs, places[at], cx);
                    })
                    .into_any_element()
            }
            Pane::Pages => {
                let pages = self
                    .shown
                    .as_ref()
                    .map(|shown| shown.view.read(cx).pages().to_vec())
                    .unwrap_or_default();
                let current = self.current_page(&pages, cx);
                let rows = pages.iter().enumerate().map(|(at, &(x, y, _, _))| {
                    let docs = docs.clone();
                    let label: SharedString =
                        Docs::text(cx, "doc.nav.page", &[("n", &(at + 1).to_string())]);
                    ListItem::new(("doc-nav-page", at), label)
                        .selected(current == Some(at))
                        .on_click(move |_, _, cx| Navigator::jump(&docs, (x, y, 0, 0), cx))
                });
                List::new().children(rows).into_any_element()
            }
            Pane::Marks => {
                let rows = self.marks.iter().enumerate().map(|(at, mark)| {
                    let docs = docs.clone();
                    let to = mark.rect.expect("listed marks are placed");
                    ListItem::new(("doc-nav-mark", at), mark.name.clone())
                        .on_click(move |_, _, cx| Navigator::jump(&docs, to, cx))
                });
                List::new().children(rows).into_any_element()
            }
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .child(head)
            .child(pane)
            .child(
                ScrollArea::new("doc-nav-scroll").vertical()
                    .flex_1()
                    .min_h_0()
                    .child(body),
            )
    }
}

#[cfg(test)]
mod tests;
