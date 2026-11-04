//! Comments on slides, beside the canvas (SHOW-08).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    collab::{Comment, CommentSidebar, Peer, Thread},
    feedback::InlineMessage,
    forms::TextInput,
    overlays::PromptDialog,
    primitives::Severity,
    theme::ControlSize,
};
use ely_lok::show::Remark;
use ely_office::prelude::*;
use gpui::{
    AnyElement, App, AppContext as _, Context, IntoElement, ParentElement as _, SharedString,
    Styled as _, Window, div, prelude::FluentBuilder as _,
};

use crate::deck::Deck;
use crate::dialog::Dialog;

/// Threads quoted with their slides; unreadable comments apart.
fn threads(remarks: &[Remark], cx: &App) -> (Vec<Thread>, Vec<(String, String)>) {
    let authors: Vec<&str> = remarks
        .iter()
        .map(|remark| remark.author.as_str())
        .collect();
    let (good, bad): (Vec<_>, Vec<_>) = remarks
        .iter()
        .map(|remark| {
            let at = remark
                .at
                .parse::<jiff::civil::DateTime>()
                .and_then(|at| at.to_zoned(jiff::tz::TimeZone::system()))
                .map_err(|error| (remark.at.clone(), error.to_string()))?
                .timestamp();
            let hue = authors
                .iter()
                .position(|name| *name == remark.author)
                .expect("the author is listed");
            let slide = (remark.part + 1).to_string();
            Ok(Thread {
                key: remark.id.clone().into(),
                quote: Some(Deck::text(cx, "show.remark.slide", &[("n", &slide)])),
                comments: vec![Comment {
                    author: Peer::new(remark.author.clone(), remark.author.clone(), hue),
                    at,
                    body: remark.text.clone().into(),
                    reactions: Vec::new(),
                }],
                resolved: false,
            })
        })
        .partition(Result::is_ok);
    let bad = bad.into_iter().filter_map(Result::err).collect();
    (good.into_iter().filter_map(Result::ok).collect(), bad)
}

impl Deck {
    /// Opens the comment dialog: new, or an edit of `edit`.
    pub fn ask_remark(
        &mut self,
        edit: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let text = match &edit {
            None => String::new(),
            Some(id) => match tab.extra.remarks.iter().find(|remark| &remark.id == id) {
                Some(remark) => remark.text.clone(),
                None => return log::warn!("show: comment {id} went before its edit"),
            },
        };
        let (tab, label) = (tab.id, Deck::text(cx, "show.remark.text", &[]));
        let field = cx.new(|cx| {
            let mut input = TextInput::new(window, cx).multi_line(3, 8).label(label);
            input.set_text(text, cx);
            input
        });
        // Refused while another dialog is open; it says so.
        let _ = self.open_dialog(Dialog::Remark { tab, edit, field }, cx);
    }

    /// Sends the dialog's text: a new comment or an edit.
    pub(crate) fn send_remark(
        &mut self,
        (tab, edit): (u64, Option<String>),
        text: &str,
        cx: &mut Context<Self>,
    ) {
        let text = text.trim();
        match edit {
            None => self.run_in(Some(tab), "comment.insert", &[text], cx),
            Some(id) => self.run_in(Some(tab), "remark.edit", &[&id, text], cx),
        }
        self.force_reread(tab, cx);
        cx.notify();
    }

    pub fn remark_dialog(
        &self,
        (tab, edit): (u64, &Option<String>),
        field: &gpui::Entity<TextInput>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let this = cx.entity();
        let label = |key: &str| Deck::text(cx, key, &[]);
        let title = match edit {
            None => "show.remark.new_title",
            Some(_) => "show.remark.edit_title",
        };
        let closing = this.clone();
        let dismiss = move |_: &mut Window, cx: &mut App| {
            closing.update(cx, |deck, cx| {
                deck.dialog = None;
                cx.notify();
            })
        };
        let (edit, blank) = (edit.clone(), label("show.remark.blank"));
        PromptDialog::new("show-remark", label(title), field, dismiss)
            .submit(label("show.remark.save"))
            .check(move |text| match text.trim().is_empty() {
                true => Err(blank.clone()),
                false => Ok(()),
            })
            .on_submit(move |text, _, cx| {
                this.update(cx, |deck, cx| {
                    deck.send_remark((tab, edit.clone()), text, cx)
                })
            })
            .into_any_element()
    }

    /// The picked comment, while its slide is shown.
    pub(crate) fn shown_remark(&self) -> Option<SharedString> {
        let tab = self.active_tab()?;
        let part = tab.extra.slides.current;
        let key = self.active_remark.clone()?;
        let here = |remark: &Remark| remark.id == key.as_ref() && remark.part == part;
        tab.extra.remarks.iter().any(here).then_some(key)
    }

    /// Shows comment `key`'s slide and marks it.
    pub(crate) fn pick_remark(&mut self, key: &SharedString, cx: &mut Context<Self>) {
        self.active_remark = Some(key.clone());
        let Some(tab) = self.active_tab() else {
            return;
        };
        if let Some(remark) = tab
            .extra
            .remarks
            .iter()
            .find(|remark| remark.id == key.as_ref())
        {
            let (view, part) = (tab.view.clone(), remark.part);
            log::info!("show: comment {key} on slide {part}");
            view.update(cx, |view, _| view.show_part(part));
        }
        cx.notify();
    }

    /// The comments column.
    pub fn remarks_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let this = cx.entity();
        let remarks = self
            .active_tab()
            .map(|tab| tab.extra.remarks.clone())
            .unwrap_or_default();
        let (threads, bad) = threads(&remarks, cx);
        for (at, error) in &bad {
            log::error!("show: a bad comment time {at}: {error}");
        }
        // One line however many: the comments below stay in view.
        let bad = bad.first().map(|(at, _)| {
            let n = bad.len().to_string();
            InlineMessage::new(Severity::Danger, Deck::text(cx, "show.remark.bad", &[("n", &n), ("at", at)]))
        });
        let active = self.shown_remark();
        let act = |id: &'static str,
                   key: &str,
                   on: bool,
                   run: fn(&mut Deck, String, &mut Window, &mut Context<Deck>)| {
            let (this, active) = (this.clone(), active.clone());
            Button::new(id, Deck::text(cx, key, &[]))
                .size(ControlSize::Sm)
                .variant(ButtonVariant::Ghost)
                .disabled(!on)
                .on_click(move |_, window, cx| {
                    let id = active
                        .clone()
                        .map(|key| key.to_string())
                        .unwrap_or_default();
                    this.update(cx, |deck, cx| run(deck, id, window, cx))
                })
        };
        let some = active.is_some();
        let head = div()
            .flex()
            .flex_wrap()
            .gap_1()
            .child(act(
                "show-remark-add",
                "show.remark.add",
                true,
                |deck, _, window, cx| deck.ask_remark(None, window, cx),
            ))
            .child(act(
                "show-remark-edit",
                "show.remark.edit",
                some,
                |deck, id, window, cx| deck.ask_remark(Some(id), window, cx),
            ))
            .child(act(
                "show-remark-delete",
                "show.remark.delete",
                some,
                |deck, id, _, cx| deck.delete_remark(&id, cx),
            ));
        let pick = this.clone();
        let sidebar = CommentSidebar::new("show-remarks", threads)
            .active(active, move |key, _, cx| {
                pick.update(cx, |deck, cx| deck.pick_remark(key, cx))
            });
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .child(head)
            .children(bad)
            .child(div().flex_1().min_h_0().child(sidebar))
            .into_any_element()
    }

    fn delete_remark(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(tab) = self.core.active() {
            self.active_remark = None;
            self.run_in(Some(tab), "remark.delete", &[id], cx);
            self.force_reread(tab, cx);
        }
    }
}

/// The comments column as a view of the deck.
pub struct Pane {
    pub deck: gpui::WeakEntity<Deck>,
}

impl gpui::Render for Pane {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(deck) = self.deck.upgrade() else {
            return div().into_any_element();
        };
        deck.update(cx, |deck, cx| deck.remarks_view(cx))
    }
}

impl Deck {
    /// A pin on the shown slide for each comment there.
    pub fn remark_layer(
        &self,
        view: &gpui::Entity<ely_lok::LokView>,
        cx: &App,
    ) -> Option<AnyElement> {
        let tab = self.active_tab()?;
        let part = tab.extra.slides.current;
        let lok = view.read(cx);
        let theme = ely_gpui_component::theme::ActiveTheme::theme(cx);
        let pins: Vec<AnyElement> = (tab.extra.remarks.iter())
            .filter(|remark| remark.part == part)
            .map(|remark| {
                let (x, y, _, _) = remark.anchor;
                let at = lok.place((x, y));
                let lit = self
                    .active_remark
                    .as_ref()
                    .is_some_and(|key| key == &remark.id);
                div()
                    .absolute()
                    .left(at.x)
                    .top(at.y)
                    .size(gpui::px(14.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(theme.colors.accent)
                    .when(lit, |pin| pin.bg(theme.colors.accent))
                    .into_any_element()
            })
            .collect();
        (!pins.is_empty()).then(|| div().absolute().inset_0().children(pins).into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use ely_lok::show::Remark;
    use gpui::TestAppContext;

    use super::threads;

    #[test]
    fn one_unreadable_time_spares_the_other_comments() {
        let cx = TestAppContext::single();
        cx.update(|cx| ely_ui::i18n::install_as("en-US", cx));
        let remark = |id: &str, at: &str| Remark {
            id: id.into(),
            author: "A".into(),
            text: "t".into(),
            at: at.into(),
            part: 0,
            anchor: (0, 0, 0, 0),
        };
        // The kernel writes an empty date as 0-00-00.
        let remarks = [remark("1", "2025-03-01"), remark("2", "0-00-00"), remark("3", "2025-03-01T10:15:00")];
        let (good, bad) = cx.update(|cx| threads(&remarks, cx));
        let keys: Vec<_> = good.iter().map(|thread| thread.key.to_string()).collect();
        assert_eq!(keys, ["1", "3"]);
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].0, "0-00-00");
    }
}
