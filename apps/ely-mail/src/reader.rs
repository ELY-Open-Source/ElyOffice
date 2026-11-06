//! The reading pane: one conversation (C12.10, C12.11).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    documents::{MarkdownBlock, MarkdownRenderer},
    feedback::{Alert, EmptyState, ErrorView},
    layout::ScrollArea,
    mail::{Contact, MailThreadView, Message},
    primitives::{IconName, Severity},
};
use ely_mail_engine::{Mailbox, MessageId};
use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    Styled as _, div,
};

use crate::{
    letter::Body,
    mail::{Mail, text},
    reading::{Each, Open},
    writing::How,
};

/// Opens a message answering the pressed one.
fn answering(
    how: How,
    cx: &mut Context<Mail>,
) -> impl Fn(&SharedString, &mut gpui::Window, &mut gpui::App) + 'static {
    let this = cx.weak_entity();
    move |key, _, cx| {
        let Ok(id) = key.parse() else {
            return log::error!("mail: a message key is not a number");
        };
        this.update(cx, |mail, cx| mail.answer(MessageId(id), how, cx))
            .ok();
    }
}

fn contact(mailbox: &Mailbox) -> Contact {
    let name = match &mailbox.name {
        Some(name) if !name.trim().is_empty() => name.clone(),
        _ => mailbox.email.clone(),
    };
    Contact::new(name, mailbox.email.clone())
}

impl Mail {
    pub fn reader(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(shown) = &self.reading.shown else {
            let none = text(cx, "mail.reader.none", &[]);
            return EmptyState::new("mail-reader-empty", IconName::Mail, none).into_any_element();
        };
        let thread = match &shown.thread {
            None => {
                let words = text(cx, "mail.reader.reading", &[]);
                return EmptyState::new("mail-reader-reading", IconName::Mail, words)
                    .into_any_element();
            }
            Some(Err(error)) => {
                let words = text(cx, "mail.reader.failed", &[]);
                return ErrorView::new("mail-reader-error", IconName::TriangleAlert, words)
                    .detail(error.clone())
                    .into_any_element();
            }
            Some(Ok(thread)) => thread,
        };
        let picked = thread
            .letters
            .iter()
            .find(|each| each.stored.id == shown.id);
        let subject = match picked.and_then(|each| each.stored.subject.clone()) {
            Some(subject) => subject.into(),
            None => text(cx, "mail.no_subject", &[]),
        };
        let mut view = MailThreadView::new("mail-thread", subject)
            .picked(shown.id.0.to_string())
            .on_reply(answering(How::Reply, cx))
            .on_reply_all(answering(How::ReplyAll, cx))
            .on_forward(answering(How::Forward, cx))
            .on_open_file({
                let this = cx.weak_entity();
                move |key, index, _, cx| {
                    this.update(cx, |mail, cx| mail.open_file(key, index, cx)).ok();
                }
            })
            .on_download({
                let this = cx.weak_entity();
                move |key, index, _, cx| {
                    this.update(cx, |mail, cx| mail.file_button(key, index, cx)).ok();
                }
            });
        for each in &thread.letters {
            view = view.message(message(each, cx), self.letter(each, cx));
        }
        let older = (thread.older > 0).then(|| {
            let count = thread.older.to_string();
            let words = text(cx, "mail.thread.older", &[("n", &count)]);
            let alert = Alert::new("mail-thread-older", Severity::Info, words);
            div().debug_selector(|| "thread-older".into()).child(alert)
        });
        let column = div().p_6().flex().flex_col().gap_4();
        let scroll = ScrollArea::new("mail-reader-scroll")
            .vertical()
            .size_full()
            .child(column.children(older).child(view));
        self.body_menu_host(cx).size_full().child(scroll).into_any_element()
    }

    /// One letter's words, notices and headers.
    fn letter(&self, each: &Each, cx: &mut Context<Self>) -> AnyElement {
        let id = each.stored.id;
        let open = match &each.open {
            Ok(open) => open,
            Err(error) => {
                let words = text(cx, "mail.reader.failed", &[]);
                let key = format!("mail-letter-error-{}", id.0);
                let view = ErrorView::new(SharedString::from(key), IconName::TriangleAlert, words)
                    .detail(error.clone());
                let named = format!("letter-error {}", id.0);
                return div()
                    .debug_selector(move || named)
                    .child(view)
                    .into_any_element();
            }
        };
        let letter = &open.letter;
        let part = div().flex().flex_col().gap_3();
        let Some(body) = &letter.body else {
            let only = text(cx, "mail.reader.headers_only", &[]);
            let key = format!("mail-headers-only-{}", id.0);
            let name = format!("mail-download-{}", id.0);
            let this = cx.weak_entity();
            let download = Button::new(SharedString::from(name.clone()), text(cx, "mail.reader.download", &[]))
                .disabled(self.downloading.contains(&id))
                .on_click(move |_, _, cx| {
                    this.update(cx, |mail, cx| mail.download(id, cx)).ok();
                });
            return part
                .child(Alert::new(SharedString::from(key), Severity::Info, only))
                .child(div().debug_selector(move || name.clone()).child(download))
                .child(self.headers(each, &letter.headers, cx))
                .into_any_element();
        };
        let drawn = MarkdownRenderer::blocks(format!("mail-body-{}", id.0), open.blocks.clone())
            .pictures(open.pictures.clone())
            .on_link(|url, _, cx| {
                // The engine kept only web and mail links.
                log::info!("mail: a link opens");
                cx.open_url(url);
            })
            .selectable({
                let this = cx.weak_entity();
                move |press, _, cx| {
                    this.update(cx, |mail, cx| mail.body_pressed(id, press, cx)).ok();
                }
            });
        part.children(self.pictures_notice(each, open, body, cx))
            .children(body.short.then(|| {
                let cut = text(cx, "mail.reader.short", &[]);
                Alert::new(
                    SharedString::from(format!("mail-short-{}", id.0)),
                    Severity::Warning,
                    cut,
                )
            }))
            .child({
                let named = format!("mail-body {}", id.0);
                div().debug_selector(move || named).child(drawn)
            })
            .child(self.headers(each, &letter.headers, cx))
            .into_any_element()
    }

    /// What web pictures wait, and how to let them in.
    fn pictures_notice(
        &self,
        each: &Each,
        open: &Open,
        body: &Body,
        cx: &mut Context<Self>,
    ) -> Option<Alert> {
        let id = each.stored.id;
        let named = |name: &str| SharedString::from(format!("mail-{name}-{}", id.0));
        let sender = each.sender().unwrap_or_default().to_owned();
        let trackers = body.trackers.to_string();
        let kept =
            (body.trackers > 0).then(|| text(cx, "mail.reader.trackers", &[("n", &trackers)]));
        if open.trusted {
            let title = text(cx, "mail.reader.trusted", &[("sender", &sender)]);
            let this = cx.weak_entity();
            let stop = Button::new(named("distrust"), text(cx, "mail.reader.distrust", &[]))
                .variant(ButtonVariant::Ghost)
                .on_click(move |_, _, cx| {
                    this.update(cx, |mail, cx| mail.trust_sender(id, false, cx))
                        .ok();
                });
            let alert = Alert::new(named("trusted"), Severity::Info, title).action(stop);
            return Some(kept.into_iter().fold(alert, Alert::body));
        }
        if body.remote.is_empty() || open.fetching {
            return kept.map(|kept| Alert::new(named("trackers"), Severity::Info, kept));
        }
        let count = body.remote.len().to_string();
        let title = text(cx, "mail.reader.blocked", &[("n", &count)]);
        let this = cx.weak_entity();
        let load = Button::new(named("load-pictures"), text(cx, "mail.reader.load", &[]))
            .variant(ButtonVariant::Primary)
            .on_click(move |_, _, cx| {
                this.update(cx, |mail, cx| mail.load_pictures(id, cx)).ok();
            });
        let mut alert = Alert::new(named("blocked"), Severity::Info, title).action(load);
        if !sender.is_empty() {
            let this = cx.weak_entity();
            let words = text(cx, "mail.reader.trust", &[("sender", &sender)]);
            alert = alert.action(
                Button::new(named("trust"), words).on_click(move |_, _, cx| {
                    this.update(cx, |mail, cx| mail.trust_sender(id, true, cx))
                        .ok();
                }),
            );
        }
        Some(kept.into_iter().fold(alert, Alert::body))
    }

    /// The header block as it came, on request.
    fn headers(&self, each: &Each, headers: &str, cx: &mut Context<Self>) -> AnyElement {
        let id = each.stored.id;
        let key = if each.headers {
            "mail.reader.headers.hide"
        } else {
            "mail.reader.headers.show"
        };
        let this = cx.weak_entity();
        let name = format!("mail-headers-{}", id.0);
        let toggle = Button::new(SharedString::from(name.clone()), text(cx, key, &[]))
            .variant(ButtonVariant::Ghost)
            .on_click(move |_, _, cx| {
                this.update(cx, |mail, cx| mail.toggle_headers(id, cx)).ok();
            });
        let toggle = div().debug_selector(move || name.clone()).child(toggle);
        let mut part = div().flex().flex_col().gap_2().child(toggle);
        if each.headers {
            let code: std::rc::Rc<[MarkdownBlock]> = vec![MarkdownBlock::Code {
                language: None,
                text: headers.trim_end().to_owned(),
                from: 0,
            }]
            .into();
            let name = format!("mail-headers-text-{}", id.0);
            part = part.child(MarkdownRenderer::blocks(name, code));
        }
        part.into_any_element()
    }
}

/// Its sender, people, date, files and first words.
fn message(each: &Each, cx: &mut Context<Mail>) -> Message {
    let stored = &each.stored;
    let from = match stored.from.first() {
        Some(from) => contact(from),
        None => Contact::new(text(cx, "mail.no_sender", &[]), ""),
    };
    let key = stored.id.0.to_string();
    let message = match stored.date.map(jiff::Timestamp::from_second) {
        Some(Ok(at)) => Message::new(key, from, at),
        Some(Err(error)) => {
            log::warn!("mail: message {} date unshown: {error}", stored.id.0);
            Message::undated(key, from)
        }
        None => Message::undated(key, from),
    };
    let mut message = message
        .to(stored.to.iter().map(contact))
        .cc(stored.cc.iter().map(contact));
    let Some(body) = each.open.as_ref().ok().and_then(Open::body) else {
        return message;
    };
    for (name, size) in &body.files {
        message = match size {
            Some(size) => message.file(name.clone(), *size),
            None => message.file_waiting(name.clone()),
        };
    }
    message.snippet(body.snippet.clone())
}
