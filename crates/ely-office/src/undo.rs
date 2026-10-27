//! Undo and Redo: keys and the Edit menu (#15).

use ely_gpui_component::secondary;
use ely_ui::commands;
use ely_lok::LokView;
use gpui::{App, Context, Div, InteractiveElement as _, KeyBinding, MenuItem, actions};

use crate::{
    Host,
    lok::{Commands as _, Lok},
    text,
};

actions!(office, [Undo, Redo]);

/// Command-Z, Shift-Command-Z; Ctrl+Y off the Mac.
pub fn bind_undo_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new(secondary!("z"), Undo, Some(commands::CONTEXT)),
        KeyBinding::new(secondary!("shift-z"), Redo, Some(commands::CONTEXT)),
    ]);
    #[cfg(not(target_os = "macos"))]
    cx.bind_keys([KeyBinding::new("ctrl-y", Redo, Some(commands::CONTEXT))]);
}

/// Undo and Redo, greyed where the kernel can't.
pub fn undo_items((undo, redo): (bool, bool), cx: &App) -> [MenuItem; 2] {
    [
        MenuItem::action(text(cx, "office.undo", &[]), Undo).disabled(!undo),
        MenuItem::action(text(cx, "office.redo", &[]), Redo).disabled(!redo),
    ]
}

/// Whether the kernel last said Undo and Redo may run.
pub fn undo_able(view: &LokView) -> (bool, bool) {
    let on = |uno| view.state(uno) == Some("enabled");
    (on(".uno:Undo"), on(".uno:Redo"))
}

/// The kernel's undo, after input already sent.
pub trait Undoing: Host<Engine = Lok> {
    /// The window root, answering Undo and Redo.
    fn bind_undo(root: Div, cx: &mut Context<Self>) -> Div {
        root.on_action(cx.listener(|host: &mut Self, _: &Undo, _, cx| {
            host.run_active("edit.undo", &[], cx)
        }))
        .on_action(cx.listener(|host: &mut Self, _: &Redo, _, cx| {
            host.run_active("edit.redo", &[], cx)
        }))
    }
}

impl<H: Host<Engine = Lok>> Undoing for H {}
