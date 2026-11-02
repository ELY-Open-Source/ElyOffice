//! Kernel dialogs Ely closed, and links the user clicked.

use gpui::{Context, EventEmitter};

use super::LokView;

/// A closed kernel dialog: its title and text.
pub struct Closed {
    pub title: String,
    pub body: String,
}

/// A link the user clicked, as the kernel names it.
pub struct Linked(pub String);

impl EventEmitter<Closed> for LokView {}

impl EventEmitter<Linked> for LokView {}

impl LokView {
    pub(super) fn relay(&mut self, blocked: Vec<(String, String)>, links: Vec<String>, cx: &mut Context<Self>) {
        for (title, body) in blocked {
            cx.emit(Closed { title, body });
        }
        for link in links {
            cx.emit(Linked(link));
        }
    }
}
