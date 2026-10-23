//! Comments and tracked changes beside the page (DOC-11).

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, IconButton, SegmentedControl},
    collab::{Comment, CommentSidebar, Peer, Thread},
    feedback::{EmptyState, InlineMessage},
    forms::{Switch, TextInput},
    layout::ScrollArea,
    lists::{List, ListItem},
    primitives::{IconName, Severity},
    theme::ControlSize,
};
use ely_lok::{
    Doc, LokError, LokView, Reported, Worker,
    review::{Change, Note},
};
use gpui::{
    App, Context, Entity, EventEmitter, InteractiveElement as _, IntoElement, Menu, StatefulInteractiveElement as _, MenuItem, ParentElement as _, Render,
    SharedString, Styled as _, Subscription, Window, actions, div,
};

use crate::docs::{Docs, Side};

actions!(doc, [TrackChanges, AcceptAll, RejectAll, ShowReview]);

/// ElyDoc's Review menu.
pub fn menu(cx: &App) -> Menu {
    let label = |key: &str| Docs::text(cx, key, &[]);
    Menu::new(label("menu.review")).items([
        MenuItem::action(label("doc.insert.comment"), crate::refs::InsertComment),
        MenuItem::action(label("doc.review.panel"), ShowReview),
        MenuItem::separator(),
        MenuItem::action(label("doc.review.track"), TrackChanges),
        MenuItem::action(label("doc.review.accept_all"), AcceptAll),
        MenuItem::action(label("doc.review.reject_all"), RejectAll),
        MenuItem::separator(),
        MenuItem::action(label("doc.compare.open"), crate::compare::CompareWith),
    ])
}

impl Docs {
    pub fn on_track(&mut self, _: &TrackChanges, _: &mut Window, cx: &mut Context<Self>) {
        self.run_active("track.toggle", &[], cx);
    }

    pub fn on_accept_all(&mut self, _: &AcceptAll, _: &mut Window, cx: &mut Context<Self>) {
        self.run_active("track.accept_all", &[], cx);
    }

    pub fn on_reject_all(&mut self, _: &RejectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.run_active("track.reject_all", &[], cx);
    }

    pub fn on_show_review(&mut self, _: &ShowReview, _: &mut Window, cx: &mut Context<Self>) {
        let side = match self.side {
            Side::Review if ely_ui::settings::panels::shown(false, cx) => Side::Props,
            _ => Side::Review,
        };
        self.show_side(side, cx);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Pane {
    Comments,
    Changes,
}

struct Shown {
    view: Entity<LokView>,
    doc: Doc,
    /// The view's review count at the last read.
    read: Option<u64>,
    /// A read is on its way.
    reading: bool,
    _observe: Subscription,
}

pub struct Review {
    worker: Arc<Mutex<Worker>>,
    shown: Option<Shown>,
    notes: Vec<Note>,
    threads: Vec<Thread>,
    /// Notes whose time does not read: time and why.
    unread: Vec<Unread>,
    pub(crate) changes: Vec<Change>,
    active: Option<SharedString>,
    pane: Pane,
}

impl EventEmitter<LokError> for Review {}

impl Review {
    pub fn new(worker: Arc<Mutex<Worker>>) -> Self {
        Review {
            worker,
            shown: None,
            notes: Vec::new(),
            threads: Vec::new(),
            unread: Vec::new(),
            changes: Vec::new(),
            active: None,
            pane: Pane::Comments,
        }
    }

    /// Follows the active tab; reads the lists when they change.
    pub fn show(&mut self, tab: Option<(&Entity<LokView>, &Doc)>, cx: &mut Context<Self>) {
        let id = tab.map(|(view, _)| view.entity_id());
        if self.shown.as_ref().map(|shown| shown.view.entity_id()) == id {
            return;
        }
        // Another document's lists must not stay actionable.
        self.shown = None;
        self.active = None;
        self.notes.clear();
        self.threads.clear();
        self.unread.clear();
        self.changes.clear();
        cx.notify();
        let Some((view, doc)) = tab else {
            return;
        };
        let observe = cx.subscribe(view, |review, _, _: &Reported, cx| review.refresh(cx));
        self.shown = Some(Shown {
            view: view.clone(),
            doc: doc.clone(),
            read: None,
            reading: false,
            _observe: observe,
        });
        self.refresh(cx);
    }

    /// Rereads the lists after review events.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(shown) = self.shown.as_mut() else {
            return;
        };
        let count = shown.view.read(cx).review();
        if shown.reading || shown.read == Some(count) {
            return;
        }
        (shown.read, shown.reading) = (Some(count), true);
        let reader = shown.view.entity_id();
        ely_lok::read_off(
            &self.worker,
            &shown.doc,
            |worker, doc| {
                let json = worker.values(doc, ".uno:ViewAnnotations")?;
                let notes = ely_lok::review::notes(&json)?;
                let json = worker.values(doc, ".uno:AcceptTrackedChanges")?;
                Ok((notes, ely_lok::review::changes(&json)?))
            },
            move |review: &mut Self, read, cx| {
                let Some(shown) = review.shown.as_mut() else {
                    return;
                };
                if shown.view.entity_id() != reader {
                    return;
                }
                shown.reading = false;
                match read {
                    Ok((notes, changes)) => review.take_lists(notes, changes),
                    Err(error) => cx.emit(LokError(format!(
                        "the review lists are unreadable: {error}"
                    ))),
                }
                cx.notify();
                review.refresh(cx);
            },
            cx,
        );
    }

    fn run(&mut self, id: &str, values: &[&str], cx: &mut Context<Self>) {
        let Some(shown) = &self.shown else {
            return;
        };
        log::info!("doc: review runs {id}");
        if let Err(error) = shown.view.update(cx, |view, _| view.run(id, values)) {
            cx.emit(LokError(format!("{id}: {error}")));
        }
    }

    fn reveal(&mut self, anchor: ely_lok::Rect, cx: &mut Context<Self>) {
        if let Some(shown) = &self.shown {
            shown.view.update(cx, |view, cx| view.reveal(anchor, cx));
        }
    }

    /// Picks a thread and goes to its words.
    fn select(&mut self, key: &SharedString, cx: &mut Context<Self>) {
        self.active = Some(key.clone());
        if let Some(note) = self.notes.iter().find(|note| note.id == key.as_ref()) {
            self.reveal(note.anchor, cx);
        }
        cx.notify();
    }

    /// Answers the picked thread.
    fn reply(&mut self, text: &str, cx: &mut Context<Self>) {
        if let Some(key) = self.active.clone() {
            self.run("comment.reply", &[&key, text], cx);
        }
    }

    fn changes_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if self.changes.is_empty() {
            let none = Docs::text(cx, "doc.review.none", &[]);
            return EmptyState::new("doc-review-none", IconName::Check, none).into_any_element();
        }
        let this = cx.entity();
        let (accept, reject) = (
            Docs::text(cx, "doc.review.accept", &[]),
            Docs::text(cx, "doc.review.reject", &[]),
        );
        let rows = self.changes.iter().map(|change| {
            let index = change.index.to_string();
            let decide = |id: &'static str, icon, tip: &SharedString| {
                let (this, index) = (this.clone(), index.clone());
                IconButton::new((id, change.index as usize), icon)
                    .size(ControlSize::Sm)
                    .variant(ButtonVariant::Ghost)
                    .tooltip(tip.clone())
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        this.update(cx, |review, cx| review.run(id, &[&index], cx))
                    })
            };
            let actions = div()
                .flex()
                .gap_1()
                .child(decide("track.accept", IconName::Check, &accept))
                .child(decide("track.reject", IconName::X, &reject));
            let mut item = ListItem::new(
                ("doc-review-change", change.index as usize),
                change.description.clone(),
            )
            .description(change.author.clone())
            .trailing(actions);
            if let Some(anchor) = change.anchor {
                let this = this.clone();
                item = item.on_click(move |_, _, cx| {
                    this.update(cx, |review, cx| review.reveal(anchor, cx))
                });
            }
            item
        });
        List::new().divided().children(rows).into_any_element()
    }
}

impl Render for Review {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let tracking = self.shown.as_ref().and_then(|shown| {
            shown
                .view
                .read(cx)
                .state(".uno:TrackChanges")
                .map(|on| on == "true")
        });
        let all = |id: &'static str, key: &str| {
            let this = this.clone();
            Button::new(id, Docs::text(cx, key, &[]))
                .size(ControlSize::Sm)
                .variant(ButtonVariant::Ghost)
                .disabled(self.changes.is_empty())
                .on_click(move |_, _, cx| this.update(cx, |review, cx| review.run(id, &[], cx)))
        };
        let head = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(
                Switch::new("doc-review-track", tracking == Some(true))
                    .disabled(tracking.is_none())
                    .label(Docs::text(cx, "doc.review.track", &[]))
                    .on_change({
                        let this = this.clone();
                        move |_, _, cx| {
                            this.update(cx, |review, cx| review.run("track.toggle", &[], cx))
                        }
                    }),
            )
            .child(all("track.accept_all", "doc.review.accept_all"))
            .child(all("track.reject_all", "doc.review.reject_all"));
        let roots = self.threads.len().to_string();
        let pane = SegmentedControl::new(
            "doc-review-pane",
            match self.pane {
                Pane::Comments => "comments",
                Pane::Changes => "changes",
            },
        )
        .size(ControlSize::Sm)
        .segment(
            "comments",
            Docs::text(cx, "doc.review.comments", &[("n", &roots)]),
            None,
        )
        .segment(
            "changes",
            Docs::text(
                cx,
                "doc.review.changes",
                &[("n", &self.changes.len().to_string())],
            ),
            None,
        )
        .on_change({
            let this = this.clone();
            move |value, _, cx| {
                this.update(cx, |review, cx| {
                    review.pane = match value.as_ref() {
                        "changes" => Pane::Changes,
                        _ => Pane::Comments,
                    };
                    cx.notify();
                })
            }
        });
        let body = match self.pane {
            Pane::Changes => ScrollArea::new("doc-review-scroll").vertical()
                .size_full()
                .child(self.changes_list(cx))
                .into_any_element(),
            Pane::Comments => {
                let field = window.use_keyed_state("doc-review-reply", cx, |window, cx| {
                    TextInput::new(window, cx)
                });
                let (pick, send) = (this.clone(), this.clone());
                CommentSidebar::new("doc-review-comments", self.threads.clone())
                    .active(self.active.clone(), move |key, _, cx| {
                        pick.update(cx, |review, cx| review.select(key, cx))
                    })
                    .reply(&field.clone(), move |_, cx| {
                        let text = field.read(cx).text().trim().to_string();
                        send.update(cx, |review, cx| review.reply(&text, cx));
                        field.update(cx, |field, cx| field.set_text("", cx));
                    })
                    .into_any_element()
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
            // Many bad notes scroll; the lists stay in reach.
            .children(self.unread_alert(cx).map(|alert| {
                ScrollArea::new("doc-review-unread-scroll").vertical().w_full().max_h(gpui::rems(6.)).flex_none().child(alert)
            }))
            .child(div().flex_1().min_h_0().child(body))
    }
}

/// A note whose time does not read, and why.
#[derive(Clone, Debug, PartialEq)]
pub struct Unread {
    pub author: String,
    pub at: String,
    pub error: String,
}

impl Review {
    /// The kernel's lists; one bad time spares the rest.
    fn take_lists(&mut self, notes: Vec<Note>, changes: Vec<Change>) {
        let (threads, unread) = threads(&notes);
        for bad in &unread {
            log::error!("doc: a bad comment time {:?}: {}", bad.at, bad.error);
        }
        log::info!("doc: review lists {} notes, {} changes", notes.len(), changes.len());
        (self.notes, self.threads, self.unread, self.changes) = (notes, threads, unread, changes);
    }

    /// Each unreadable note, by its author and the reason.
    fn unread_lines(&self, cx: &App) -> Vec<SharedString> {
        let line = |bad: &Unread| {
            Docs::text(cx, "doc.review.unread", &[("author", &bad.author), ("at", &bad.at), ("error", &bad.error)])
        };
        self.unread.iter().map(line).collect()
    }

    /// The lines as one alert assistive tech reads whole.
    fn unread_alert(&self, cx: &App) -> Option<gpui::AnyElement> {
        let lines = self.unread_lines(cx);
        let said: SharedString = lines.join("\n").into();
        (!lines.is_empty()).then(|| {
            div()
                .id("doc-review-unread")
                .role(gpui::Role::Alert)
                .aria_label(said)
                .flex()
                .flex_col()
                .gap_1()
                .children(lines.into_iter().map(|line| InlineMessage::new(Severity::Danger, line)))
                .into_any_element()
        })
    }
}

/// Notes threaded under their first comment; bad times apart.
fn threads(notes: &[Note]) -> (Vec<Thread>, Vec<Unread>) {
    let parents: HashMap<&str, &str> = notes
        .iter()
        .map(|note| (note.id.as_str(), note.parent.as_str()))
        .collect();
    let root = |id: &'_ str| -> String {
        let mut id = id;
        while let Some(parent) = parents.get(id).filter(|parent| **parent != "0") {
            id = parent;
        }
        id.to_string()
    };
    let authors: Vec<&str> = notes.iter().map(|note| note.author.as_str()).collect();
    let (mut threads, mut unread): (Vec<Thread>, Vec<Unread>) = (Vec::new(), Vec::new());
    for note in notes {
        let read = note.at.parse::<jiff::civil::DateTime>().and_then(|at| at.to_zoned(jiff::tz::TimeZone::system()));
        let at = match read {
            Ok(at) => at,
            Err(error) => {
                unread.push(Unread { author: note.author.clone(), at: note.at.clone(), error: error.to_string() });
                continue;
            }
        };
        let hue = authors
            .iter()
            .position(|name| *name == note.author)
            .expect("the author is listed");
        let comment = Comment {
            author: Peer::new(note.author.clone(), note.author.clone(), hue),
            at: at.timestamp(),
            body: note.text.clone().into(),
            reactions: Vec::new(),
        };
        let key: SharedString = root(&note.id).into();
        match threads.iter_mut().find(|thread| thread.key == key) {
            Some(thread) => thread.comments.push(comment),
            None => threads.push(Thread {
                key,
                quote: None,
                comments: vec![comment],
                resolved: note.resolved,
            }),
        }
    }
    (threads, unread)
}

#[cfg(test)]
mod tests;
