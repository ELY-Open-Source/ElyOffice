//! What the compose window draws.

use ely_gpui_component::{
    buttons::SegmentedControl,
    chat::Attachment,
    feedback::{Alert, InlineMessage},
    forms::{Choice, Select},
    layout::ScrollArea,
    mail::{MailComposer, QuotedText},
    overlays::ConfirmDialog,
    primitives::Severity,
    theme::{ActiveTheme as _, Radius},
    typography::format,
};
use ely_mail_engine::{AccountId, compose};
use ely_ui::Shell;
use gpui::{
    AnyElement, App, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    Styled as _, Window, div, prelude::FluentBuilder as _, px,
};

use super::{Compose, file_name};
use ely_mail_engine::compose::Earlier;

use crate::{mail::text, writing::Sender};

impl Compose {
    fn sender_picker(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.senders.len() < 2 {
            return None;
        }
        let choices = self
            .senders
            .iter()
            .map(|sender| Choice::new(sender.id.0.to_string(), sender.address.clone()));
        let this = cx.weak_entity();
        let select = Select::new("mail-write-from", choices)
            .label(text(cx, "mail.write.from", &[]))
            .selected(self.from.0.to_string())
            .on_change(move |value, _, cx| {
                let Ok(id) = value.parse() else {
                    return log::error!("mail: a sender key is not a number");
                };
                this.update(cx, |compose, cx| compose.choose_sender(AccountId(id), cx))
                    .ok();
            });
        Some(div().max_w_96().child(select).into_any_element())
    }

    fn mode(&self, cx: &mut Context<Self>) -> SegmentedControl {
        let this = cx.weak_entity();
        let chosen = if self.markdown { "markdown" } else { "plain" };
        SegmentedControl::new("mail-write-mode", chosen)
            .segment("plain", text(cx, "mail.write.plain", &[]), None)
            .segment("markdown", text(cx, "mail.write.markdown", &[]), None)
            .on_change(move |value, window, cx| {
                let markdown = value == "markdown";
                this.update(cx, |compose, cx| compose.set_markdown(markdown, window, cx))
                    .ok();
            })
    }

    fn notices(&self, cx: &App) -> Vec<AnyElement> {
        let mut notices = Vec::new();
        if self.missing > 0 {
            let count = self.missing.to_string();
            let words = text(cx, "mail.write.missing", &[("n", &count)]);
            let alert = Alert::new("mail-write-missing", Severity::Warning, words);
            let shown = div().debug_selector(|| "write-missing".into()).child(alert);
            notices.push(shown.into_any_element());
        }
        if self.reading > 0 {
            let count = self.reading.to_string();
            let words = text(cx, "mail.write.reading", &[("n", &count)]);
            notices
                .push(Alert::new("mail-write-reading", Severity::Info, words).into_any_element());
        }
        notices.extend(self.size_notice(cx));
        if self.headers_only {
            let words = text(cx, "mail.write.headers_only", &[]);
            notices
                .push(Alert::new("mail-write-headers", Severity::Info, words).into_any_element());
        }
        if let Some(before) = &self.before {
            let (lead, words) = match before {
                Earlier::Quoted { lead, text } => (lead.clone(), text.clone()),
                Earlier::Forwarded { lead, text } => (lead.join("\n"), text.clone()),
            };
            let lines = format!("{lead}\n\n{words}");
            let quoted = QuotedText::new("mail-write-earlier")
                .children(lines.lines().map(|line| div().child(line.to_owned())));
            notices.push(quoted.into_any_element());
        }
        notices
    }

    /// Past the sender's known limit: it, the size, the limit.
    pub fn over_limit(&self, cx: &App) -> Option<(&Sender, u64, u64)> {
        let files: u64 = self
            .files
            .iter()
            .map(|file| file.part.bytes.len() as u64)
            .sum();
        // Base64 grows files by a third.
        let whole = files * 4 / 3 + self.document.read(cx).text(cx).len() as u64 * 2;
        let sender = self.sender()?;
        let limit = sender.limit?;
        (whole > limit).then_some((sender, whole, limit))
    }

    /// The files' size; a warning past the server's limit.
    fn size_notice(&self, cx: &App) -> Option<AnyElement> {
        if self.files.is_empty() {
            return None;
        }
        let size = |bytes| format::file_size(bytes, false);
        let alert = match self.over_limit(cx) {
            Some((sender, whole, limit)) => {
                let (whole, limit) = (size(whole), size(limit));
                let args = [
                    ("size", whole.as_str()),
                    ("address", sender.address.as_str()),
                    ("limit", limit.as_str()),
                ];
                let words = text(cx, "mail.outbox.too_large", &args);
                Alert::new("mail-write-size", Severity::Warning, words)
            }
            None => {
                let files = self.files.iter().map(|file| file.part.bytes.len() as u64);
                let words = text(
                    cx,
                    "mail.outbox.files_size",
                    &[("size", &size(files.sum()))],
                );
                Alert::new("mail-write-size", Severity::Info, words)
            }
        };
        let shown = div().debug_selector(|| "write-size".into()).child(alert);
        Some(shown.into_any_element())
    }

    fn discard_dialog(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.discarding {
            return None;
        }
        let (this, confirm) = (cx.weak_entity(), cx.weak_entity());
        let dialog = ConfirmDialog::new(
            "mail-write-discard",
            text(cx, "mail.write.discard.title", &[]),
            text(cx, "mail.write.discard.message", &[]),
            move |_, cx| {
                this.update(cx, |compose, cx| compose.keep_writing(cx)).ok();
            },
        )
        .confirm(text(cx, "mail.write.discard.go", &[]))
        .destructive()
        .on_confirm(move |window, cx| {
            confirm
                .update(cx, |compose, cx| compose.confirm_discard(window, cx))
                .ok();
        });
        Some(dialog.into_any_element())
    }
}

impl Render for Compose {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let body = div()
            .debug_selector(|| "write-words".into())
            .h(px(260.0))
            .border_1()
            .border_color(theme.colors.border)
            .rounded(theme.radius(Radius::Md))
            .overflow_hidden()
            .child(self.document.clone());
        let files = self.files.iter().map(|file| Attachment {
            key: file.key.to_string().into(),
            name: file_name(&file.part, cx),
            bytes: file.part.bytes.len() as u64,
            preview: None,
            progress: None,
        });
        let signature = self
            .sender()
            .map(|sender| sender.identity.signature.trim().to_owned())
            .filter(|signature| !signature.is_empty());
        let (send, discard, attach, remove, edit) = (
            cx.weak_entity(),
            cx.weak_entity(),
            cx.weak_entity(),
            cx.weak_entity(),
            cx.weak_entity(),
        );
        let composer = MailComposer::new("mail-composer", body)
            .to(self.to.clone())
            .cc(self.cc.clone())
            .bcc(self.bcc.clone())
            .subject(self.subject.clone())
            .contacts(self.offered.clone())
            .busy(self.sending || self.reading > 0)
            .check(compose::is_address)
            .on_change(move |draft, _, cx| {
                edit.update(cx, |compose, cx| compose.edited(draft.clone(), cx))
                    .ok();
            })
            .files(files)
            .when_some(signature, |composer, signature| {
                composer.signature(signature)
            })
            .on_send(move |draft, window, cx| {
                send.update(cx, |compose, cx| compose.send(draft, window, cx))
                    .ok();
            })
            .on_discard(move |window, cx| {
                discard
                    .update(cx, |compose, cx| compose.discard(window, cx))
                    .ok();
            })
            .on_attach(move |_, cx| {
                attach.update(cx, |compose, cx| compose.pick_files(cx)).ok();
            })
            .on_remove(move |key, _, cx| {
                remove
                    .update(cx, |compose, cx| {
                        compose
                            .files
                            .retain(|file| file.key.to_string() != key.as_ref());
                        compose.touched(cx);
                        cx.notify();
                    })
                    .ok();
            });
        let problem = self.problem.clone().map(|(severity, words)| {
            div()
                .debug_selector(|| "write-problem".into())
                .child(InlineMessage::new(severity, words))
        });
        let page = div()
            .p_6()
            .flex()
            .flex_col()
            .gap_4()
            .children(self.sender_picker(cx))
            .child(div().flex().child(self.mode(cx)))
            .child(composer)
            .children(problem)
            .children(self.notices(cx));
        Shell::new(text(cx, "mail.write.title", &[]), &self.focus)
            .secondary()
            .child(
                ScrollArea::new("mail-write-scroll")
                    .vertical()
                    .size_full()
                    .child(page),
            )
            .children(self.discard_dialog(cx))
    }
}
