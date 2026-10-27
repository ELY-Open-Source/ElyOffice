//! Typora's Copy as Markdown and Paste as Plain Text.

use ely_gpui_component::secondary;
use ely_gpui_component::primitives::Severity;
use ely_office::prelude::*;
use ely_ui::{commands, i18n::text};
use gpui::{
    App, ClipboardItem, Context, Focusable as _, InteractiveElement as _, KeyBinding, Window,
    actions,
};

use crate::desk::Desk;

actions!(mark, [CopyMarkdown, PastePlain]);

pub fn bind(cx: &mut App) {
    let at = Some(commands::CONTEXT);
    cx.bind_keys([
        KeyBinding::new(secondary!("shift-c"), CopyMarkdown, at),
        KeyBinding::new(secondary!("shift-v"), PastePlain, at),
    ]);
}

impl Desk {
    /// Whether the document or nothing else holds focus.
    fn on_document(&self, window: &Window, cx: &App) -> bool {
        let Some(focused) = window.focused(cx) else {
            return true;
        };
        let held = (self.active_view()).is_some_and(|view| {
            let document = view.read(cx).document().read(cx);
            document.focus_handle(cx).contains_focused(window, cx)
        });
        held || focused == self.core.focus
    }

    /// Hands a field outside the document its own `name` action.
    fn to_field(name: &str, window: &mut Window, cx: &mut App) {
        match cx.build_action(name, None) {
            Ok(action) => window.dispatch_action(action, cx),
            Err(error) => log::error!("mark: no action {name}: {error}"),
        }
    }

    /// The selected Markdown, or the whole text, as written.
    fn copy_markdown(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.on_document(window, cx) {
            return Self::to_field("ely_input::Copy", window, cx);
        }
        let Some(view) = self.active_view() else {
            return self.no_document(cx);
        };
        let document = view.read(cx).document().read(cx);
        let selected = document.selected(cx);
        let markdown = match selected.is_empty() {
            true => document.text(cx),
            false => selected,
        };
        log::info!("mark: {} bytes copied as Markdown", markdown.len());
        cx.write_to_clipboard(ClipboardItem::new_string(markdown));
    }

    /// The clipboard's text at the caret, pictures left out.
    fn paste_plain(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.on_document(window, cx) {
            return Self::to_field("ely_input::Paste", window, cx);
        }
        let Some(view) = self.active_view() else {
            return self.no_document(cx);
        };
        let Some(pasted) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            let said = text(cx, "mark.paste.none", &[]);
            return self.toast(said, Severity::Info, cx);
        };
        let document = view.read(cx).document().clone();
        if !document.update(cx, |document, cx| document.insert(&pasted, cx)) {
            let said = text(cx, "mark.preview.read_only", &[]);
            self.toast(said, Severity::Info, cx);
        }
    }

    pub fn clip_actions(root: gpui::Div, cx: &mut Context<Self>) -> gpui::Div {
        root.on_action(
            cx.listener(|desk, _: &CopyMarkdown, window, cx| desk.copy_markdown(window, cx)),
        )
        .on_action(cx.listener(|desk, _: &PastePlain, window, cx| desk.paste_plain(window, cx)))
    }
}
