//! Hyperlinks on a slide: Insert, Hyperlink (SHOW-03).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::{Input, TextInput},
    overlays::Dialog as Card,
};
use ely_office::prelude::*;
use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, IntoElement, ParentElement as _,
    Styled as _, Window, div,
};

use crate::deck::Deck;
use crate::dialog::Dialog;

impl Deck {
    /// Opens the hyperlink dialog: the words shown, and the address.
    pub fn ask_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active() else {
            return;
        };
        let (words, address) = (Deck::text(cx, "doc.link.text", &[]), Deck::text(cx, "doc.link.url", &[]));
        let text = cx.new(|cx| TextInput::new(window, cx).label(words));
        let url = cx.new(|cx| TextInput::new(window, cx).label(address));
        // The Insert button waits for an address.
        cx.observe(&url, |_, _, cx| cx.notify()).detach();
        // Refused while another dialog is open; it says so.
        let _ = self.open_dialog(Dialog::Link { tab, text, url }, cx);
    }

    /// Sends the dialog: a link worded as asked.
    pub(crate) fn link_asked(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Link { tab, text, url }) = self.dialog.take() else {
            return;
        };
        let (words, address) = (
            text.read(cx).text().trim().to_string(),
            url.read(cx).text().trim().to_string(),
        );
        if address.is_empty() {
            return log::warn!("show: a link asked with no address");
        }
        // No words show the address.
        let shown = if words.is_empty() { &address } else { &words };
        self.run_in(Some(tab), "show.link.insert", &[shown, &address], cx);
        self.force_reread(tab, cx);
        cx.notify();
    }

    pub fn link_dialog(
        &self,
        (text, url): (&Entity<TextInput>, &Entity<TextInput>),
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let this = cx.entity();
        let label = |key: &str| Deck::text(cx, key, &[]);
        let fields = DescriptionList::new()
            .item(label("doc.link.text"), div().w_full().max_w_64().child(Input::new(text)))
            .item(label("doc.link.url"), div().w_full().max_w_64().child(Input::new(url)));
        let closing = this.clone();
        let dismiss = move |_: &mut Window, cx: &mut App| {
            closing.update(cx, |deck, cx| {
                deck.dialog = None;
                cx.notify();
            })
        };
        let (go, empty) = (label("doc.insert.insert"), url.read(cx).text().trim().is_empty());
        Card::new("show-link", label("doc.link.title"), dismiss)
            .child(fields)
            .action(ely_office::cancel(cx))
            .action(move |_| {
                Button::new("show-link-go", go)
                    .variant(ButtonVariant::Primary)
                    .disabled(empty)
                    .on_click(move |_, _, cx| this.update(cx, |deck, cx| deck.link_asked(cx)))
            })
            .into_any_element()
    }
}
