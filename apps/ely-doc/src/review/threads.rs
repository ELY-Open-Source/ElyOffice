//! Sidebar threads: edit, delete, resolve (#123, #145, #148).

use std::collections::HashMap;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    collab::{Comment, Peer, Thread},
    overlays::ConfirmDialog,
    theme::ControlSize,
};
use ely_lok::{LokError, review::Note};
use gpui::{App, Context, Entity, IntoElement, SharedString, Window};

use super::{DeleteComments, Review, Unread};
use crate::{actions::Dialog, docs::Docs};

/// Each thread's note ids, comment by comment.
pub(super) type Ids = HashMap<SharedString, Vec<String>>;

/// The author, hued by their first note.
pub(super) fn author(notes: &[Note], note: &Note) -> Peer {
    let hue = notes
        .iter()
        .position(|other| other.author == note.author)
        .expect("the note is among the notes");
    Peer::new(note.author.clone(), note.author.clone(), hue)
}

/// Notes threaded under their first comment; bad times apart.
pub(super) fn threads(notes: &[Note]) -> (Vec<Thread>, Ids, Vec<Unread>) {
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
    let (mut threads, mut ids, mut unread): (Vec<Thread>, Ids, Vec<Unread>) = (Vec::new(), Ids::new(), Vec::new());
    for note in in_order(notes) {
        let read = note.at.parse::<jiff::civil::DateTime>().and_then(|at| at.to_zoned(jiff::tz::TimeZone::system()));
        let at = match read {
            Ok(at) => at,
            Err(error) => {
                unread.push(Unread { author: note.author.clone(), at: note.at.clone(), error: error.to_string() });
                continue;
            }
        };
        let comment = Comment {
            author: author(notes, note),
            at: at.timestamp(),
            body: note.text.clone().into(),
            reactions: Vec::new(),
        };
        let key: SharedString = root(&note.id).into();
        ids.entry(key.clone()).or_default().push(note.id.clone());
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
    (threads, ids, unread)
}

/// First comments where the kernel puts them; replies by id.
fn in_order(notes: &[Note]) -> Vec<&Note> {
    // The kernel lists replies newest first; ids count up.
    let (mut roots, mut replies): (Vec<&Note>, Vec<&Note>) = notes.iter().partition(|note| note.parent == "0");
    replies.sort_by(|a, b| (a.id.len(), &a.id).cmp(&(b.id.len(), &b.id)));
    roots.append(&mut replies);
    roots
}

impl Review {
    /// Rewrites comment `ix` of thread `key`; one undo step.
    pub(super) fn edit(&mut self, key: &SharedString, ix: usize, text: &str, cx: &mut Context<Self>) {
        if let Some(id) = self.note_id(key, ix, cx) {
            self.run("comment.edit", &[&id, text], cx);
        }
    }

    /// Deletes comment `ix`; the first takes its replies.
    pub(super) fn delete(&mut self, key: &SharedString, ix: usize, cx: &mut Context<Self>) {
        let Some(id) = self.note_id(key, ix, cx) else {
            return;
        };
        let command = if ix == 0 { "comment.delete_thread" } else { "comment.delete" };
        self.run(command, &[&id], cx);
    }

    /// Resolves thread `key`, or reopens it; one undo step.
    pub(super) fn resolve(&mut self, key: &SharedString, on: bool, cx: &mut Context<Self>) {
        let Some(thread) = self.threads.iter().find(|thread| &thread.key == key) else {
            log::error!("doc: thread {key} is not listed");
            cx.emit(LokError(format!("thread {key} is not listed")));
            return;
        };
        // The kernel toggles; a stale ask would flip it back.
        if thread.resolved == on {
            log::warn!("doc: thread {key} is already resolved {on}");
            return;
        }
        if let Some(id) = self.note_id(key, 0, cx) {
            self.run("comment.resolve", &[&id], cx);
        }
    }

    fn note_id(&self, key: &SharedString, ix: usize, cx: &mut Context<Self>) -> Option<String> {
        let id = self.ids.get(key).and_then(|ids| ids.get(ix)).cloned();
        if id.is_none() {
            log::error!("doc: comment {ix} of thread {key} is not listed");
            cx.emit(LokError(format!("comment {ix} of thread {key} is not listed")));
        }
        id
    }
}

impl Review {
    /// The sidebar's Delete All Comments button.
    pub(super) fn delete_all(&self, cx: &App) -> impl IntoElement {
        Button::new("doc-review-delete-comments", Docs::text(cx, "doc.review.delete_comments", &[]))
            .size(ControlSize::Sm)
            .variant(ButtonVariant::Ghost)
            .disabled(self.notes.is_empty())
            .on_click(|_, window, cx| window.dispatch_action(Box::new(DeleteComments), cx))
    }
}

impl Docs {
    /// Asks, then deletes every comment and reply (#148).
    pub fn on_delete_comments(&mut self, _: &DeleteComments, _: &mut Window, cx: &mut Context<Self>) {
        let count = self.review.read(cx).notes.len();
        if count == 0 {
            log::info!("doc: no comments to delete");
            return;
        }
        self.dialog = Some(Dialog::DeleteComments { count });
        cx.notify();
    }

    pub(crate) fn delete_comments(&mut self, cx: &mut Context<Self>) {
        self.dialog = None;
        log::info!("doc: deletes every comment");
        self.run_active("comment.delete_all", &[], cx);
        cx.notify();
    }
}

/// The delete-all warning; one undo brings them back.
pub(crate) fn delete_comments_dialog(
    this: Entity<Docs>,
    count: usize,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> ConfirmDialog {
    ConfirmDialog::new(
        "doc-delete-comments",
        Docs::text(cx, "doc.review.delete_comments.title", &[]),
        Docs::text(cx, "doc.review.delete_comments.body", &[("count", &count.to_string())]),
        dismiss,
    )
    .destructive()
    .confirm(Docs::text(cx, "doc.review.delete_comments.confirm", &[]))
    .on_confirm(move |_, cx| this.update(cx, |docs, cx| docs.delete_comments(cx)))
}
