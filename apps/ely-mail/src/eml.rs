//! A message file, read in its own window (C12.20).

use std::{path::PathBuf, rc::Rc};

use ely_gpui_component::{
    documents::{MarkdownBlock, MarkdownRenderer, Pictures},
    feedback::{Alert, EmptyState, ErrorView},
    layout::ScrollArea,
    mail::{Contact, MailReader, Message},
    primitives::{IconName, Severity},
};
use ely_mail_engine::{Mailbox, Part};
use ely_platform::AppId;
use ely_ui::Shell;
use gpui::{
    AnyElement, App, AppContext as _, Context, FocusHandle, IntoElement, ParentElement as _, Render,
    Styled as _, Window, div, px, size,
};

use crate::{
    letter::{self, Body},
    mail::{Mail, text},
    reading::words,
};

/// The file as read off the UI thread.
struct Read {
    mail: ely_mail_engine::Mail,
    body: Body,
}

/// The file as drawn.
pub struct Shown {
    pub subject: Option<String>,
    from: Vec<Mailbox>,
    to: Vec<Mailbox>,
    cc: Vec<Mailbox>,
    date: Option<i64>,
    blocks: Rc<[MarkdownBlock]>,
    pictures: Rc<Pictures>,
    blocked: usize,
    pub files: Vec<Part>,
}

impl From<Read> for Shown {
    fn from(read: Read) -> Self {
        let (mail, body) = (read.mail, read.body);
        Shown {
            subject: mail.subject,
            from: mail.from,
            to: mail.to,
            cc: mail.cc,
            date: mail.date,
            blocks: body.blocks.into(),
            pictures: Rc::new(body.pictures),
            blocked: body.remote.len() + body.trackers,
            files: mail.attachments,
        }
    }
}

pub struct Eml {
    pub shown: Option<Result<Shown, String>>,
    /// What the last save did.
    pub said: Option<(Severity, gpui::SharedString)>,
    focus: FocusHandle,
}

/// True for a file ElyMail reads rather than attaches.
pub fn is_eml(path: &std::path::Path) -> bool {
    path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("eml"))
}

impl Mail {
    /// Links and messages open; other files join a draft.
    pub fn take_files(&mut self, files: Vec<PathBuf>, cx: &mut Context<Self>) {
        let (links, files): (Vec<_>, Vec<_>) = files.into_iter().partition(|opened| ely_ui::mail_link(opened).is_some());
        for link in links {
            match ely_mail_engine::mailto::parse(ely_ui::mail_link(&link).expect("a mail link")) {
                Ok(asked) => self.write_asked(asked, cx),
                Err(error) => self.failed("mail.write.link", &error, cx),
            }
        }
        let (messages, others): (Vec<_>, Vec<_>) = files.into_iter().partition(|path| is_eml(path));
        for path in messages {
            self.open_eml(path, cx);
        }
        if !others.is_empty() {
            self.write_with(others, cx);
        }
    }

    /// Asks for message files, then opens each.
    pub fn pick_eml(&mut self, cx: &mut Context<Self>) {
        let options = gpui::PathPromptOptions { files: true, directories: false, multiple: true, prompt: None };
        let picked = cx.prompt_for_paths(options);
        cx.spawn(async move |this, cx| {
            let paths = match picked.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) => return log::info!("mail: opening a message file was cancelled"),
                Ok(Err(error)) => return log::error!("mail: the open panel failed: {error}"),
                Err(error) => return log::error!("mail: the open panel went away: {error}"),
            };
            this.update(cx, |mail, cx| paths.into_iter().for_each(|path| mail.open_eml(path, cx))).ok();
        })
        .detach();
    }

    pub fn open_eml(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let mut options = ely_ui::window_options(AppId::Mail, size(px(760.0), px(680.0)), cx);
        if let (Some(bar), Some(name)) = (options.titlebar.as_mut(), path.file_name()) {
            bar.title = Some(name.to_string_lossy().into_owned().into());
        }
        match cx.open_window(options, |_, cx| cx.new(|cx| Eml::new(path, cx))) {
            Ok(_) => log::info!("mail: a message file opened"),
            Err(error) => self.failed("mail.eml.unopened", &format!("{error:#}"), cx),
        }
    }
}

impl Eml {
    fn new(path: PathBuf, cx: &mut Context<Self>) -> Self {
        let words = words(cx);
        cx.spawn(async move |this, cx| {
            let read = cx.background_spawn(async move { read_file(&path, &words) }).await;
            this.update(cx, |eml, cx| {
                eml.shown = Some(read.map(Shown::from));
                cx.notify();
            })
            .ok();
        })
        .detach();
        Eml { shown: None, said: None, focus: cx.focus_handle() }
    }

    pub(crate) fn save_file(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(Ok(shown)) = &self.shown else {
            return log::error!("mail: a file of an unread message file");
        };
        let Some(part) = shown.files.get(index).cloned() else {
            return log::error!("mail: no file {index} in the message file");
        };
        let Some(dirs) = directories::UserDirs::new() else {
            return log::error!("mail: no home folder to save into");
        };
        let dir = dirs.download_dir().unwrap_or(dirs.home_dir()).to_path_buf();
        let name = part.name.clone().unwrap_or_else(|| text(cx, "mail.file.unnamed", &[]).to_string());
        let chosen = cx.prompt_for_new_path(&dir, Some(&name));
        cx.spawn(async move |this, cx| {
            let path = match chosen.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) => return log::info!("mail: saving a file was cancelled"),
                Ok(Err(error)) => return log::error!("mail: the save panel failed: {error}"),
                Err(error) => return log::error!("mail: the save panel went away: {error}"),
            };
            let target = path.clone();
            let written = cx.background_spawn(async move { ely_files::export(&target, None, &part.bytes, 0) }).await;
            this.update(cx, |eml, cx| {
                let shown = path.display().to_string();
                eml.said = Some(match written {
                    Ok(_) => (Severity::Success, text(cx, "mail.eml.saved", &[("path", &shown)])),
                    Err(error) => (Severity::Danger, text(cx, "mail.file.unsaved", &[("error", &error.to_string())])),
                });
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

fn read_file(path: &std::path::Path, words: &letter::Words) -> Result<Read, String> {
    let bytes = std::fs::read(path).map_err(|err| err.to_string())?;
    let mail = ely_mail_engine::read(&bytes).map_err(|err| err.to_string())?;
    let body = letter::body(&[], mail.clone(), words);
    Ok(Read { mail, body })
}

fn contact(mailbox: &Mailbox) -> Contact {
    let name = mailbox.name.clone().filter(|name| !name.trim().is_empty());
    Contact::new(name.unwrap_or_else(|| mailbox.email.clone()), mailbox.email.clone())
}

impl Render for Eml {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = text(cx, "mail.eml.title", &[]);
        Shell::new(title, &self.focus).secondary().child(
            ScrollArea::new("mail-eml-scroll")
                .vertical()
                .size_full()
                .child(div().p_6().child(self.page(cx))),
        )
    }
}

impl Eml {
    fn page(&self, cx: &mut Context<Self>) -> AnyElement {
        let shown = match &self.shown {
            None => {
                let words = text(cx, "mail.reader.reading", &[]);
                return EmptyState::new("mail-eml-reading", IconName::Mail, words).into_any_element();
            }
            Some(Err(error)) => {
                let words = text(cx, "mail.eml.unread", &[]);
                return ErrorView::new("mail-eml-error", IconName::TriangleAlert, words)
                    .detail(error.clone())
                    .into_any_element();
            }
            Some(Ok(shown)) => shown,
        };
        let from = match shown.from.first() {
            Some(from) => contact(from),
            None => Contact::new(text(cx, "mail.no_sender", &[]), ""),
        };
        let message = match shown.date.map(jiff::Timestamp::from_second) {
            Some(Ok(at)) => Message::new("eml", from, at),
            _ => Message::undated("eml", from),
        };
        let mut message = message.to(shown.to.iter().map(contact)).cc(shown.cc.iter().map(contact));
        for file in &shown.files {
            let name = file.name.clone().unwrap_or_else(|| text(cx, "mail.file.unnamed", &[]).to_string());
            message = message.file(name, file.bytes.len() as u64);
        }
        let subject = match &shown.subject {
            Some(subject) => subject.clone().into(),
            None => text(cx, "mail.no_subject", &[]),
        };
        let blocked = shown.blocked;
        // A file from disk never loads web pictures.
        let notice = (blocked > 0).then(|| {
            let words = text(cx, "mail.eml.pictures", &[("n", &blocked.to_string())]);
            Alert::new("mail-eml-pictures", Severity::Info, words)
        });
        let this = cx.weak_entity();
        let drawn = MarkdownRenderer::blocks("mail-eml-body", shown.blocks.clone())
            .pictures(shown.pictures.clone())
            .on_link(|url, _, cx: &mut App| {
                log::info!("mail: a link in a message file opens");
                cx.open_url(url);
            });
        let said = self.said.clone().map(|(severity, words)| Alert::new("mail-eml-said", severity, words));
        MailReader::new("mail-eml", subject, message)
            .on_download(move |_, index, _, cx| {
                this.update(cx, |eml, cx| eml.save_file(index, cx)).ok();
            })
            .children(said)
            .children(notice)
            .child(drawn)
            .into_any_element()
    }
}
