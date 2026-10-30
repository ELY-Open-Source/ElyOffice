//! A message's files: download, save, open a copy (MAIL-10, §16).

use std::path::PathBuf;

use ely_gpui_component::primitives::Severity;
use ely_mail_engine::{Error, MessageId, Part, Store, sync, whole::whole};
use gpui::{AppContext as _, Context, SharedString};

use crate::mail::{Mail, text};

/// The file at `index` of `message`, as kept here.
fn part(store: &Store, message: MessageId, index: usize) -> ely_mail_engine::Result<Part> {
    let raw = store
        .original(message)?
        .ok_or_else(|| Error::Store(format!("message {} has only its header here", message.0)))?;
    let mut files = ely_mail_engine::read(&raw)?.attachments;
    if index >= files.len() {
        return Err(Error::Store(format!("message {} has no file {index}", message.0)));
    }
    Ok(files.swap_remove(index))
}

/// The file's name, or the words for an unnamed one.
fn named(part: &Part, cx: &gpui::App) -> String {
    part.name.clone().unwrap_or_else(|| text(cx, "mail.file.unnamed", &[]).to_string())
}

impl Mail {
    /// The file's button: download it, or save it once here.
    pub(crate) fn file_button(&mut self, key: &SharedString, index: usize, cx: &mut Context<Self>) {
        let Ok(id) = key.parse().map(MessageId) else {
            return log::error!("mail: a message key is not a number");
        };
        let waiting = self.reading.files(id).and_then(|files| files.get(index)).is_some_and(|(_, size)| size.is_none());
        match waiting {
            true => self.download(id, cx),
            false => self.save_file(id, index, cx),
        }
    }

    /// Brings the message down whole, its files kept here.
    pub(crate) fn download(&mut self, id: MessageId, cx: &mut Context<Self>) {
        if !self.downloading.insert(id) {
            return log::info!("mail: message {} is already coming down", id.0);
        }
        let Ok(store) = self.store.clone() else {
            self.downloading.remove(&id);
            return self.failed("mail.file.download_failed", "no store", cx);
        };
        let tls = match self.tls.clone() {
            Ok(tls) => tls,
            Err(error) => {
                self.downloading.remove(&id);
                return self.failed("mail.file.download_failed", &error, cx);
            }
        };
        let (vault, accounts) = (self.vault.clone(), self.accounts().to_vec());
        self.toast(text(cx, "mail.file.downloading", &[]), Severity::Info, cx);
        let work = async move {
            let owner = sync::lock(&store)?.owner(id)?;
            let (_, account) = accounts
                .into_iter()
                .find(|(each, _)| *each == owner)
                .ok_or_else(|| Error::Store("the message's account is gone".into()))?;
            let secret = sync::secret(&account, &vault).await?;
            whole(&account, owner, secret, &tls, &store, id).await
        };
        self.on_runtime(cx, work, move |mail, done, cx| {
            mail.downloading.remove(&id);
            match done.and_then(|done| done.map_err(|error| error.to_string())) {
                Ok(()) => mail.read_letter(cx),
                Err(error) => mail.failed("mail.file.download_failed", &error, cx),
            }
        });
    }

    /// Asks where, then writes the file there.
    fn save_file(&mut self, id: MessageId, index: usize, cx: &mut Context<Self>) {
        self.with_store(
            cx,
            move |store| part(store, id, index),
            move |mail, read, cx| {
                let part = match read {
                    Ok(part) => part,
                    Err(error) => return mail.failed("mail.file.unread", &error, cx),
                };
                let Some(dirs) = directories::UserDirs::new() else {
                    return mail.failed("mail.file.unsaved", "no home folder", cx);
                };
                let dir = dirs.download_dir().unwrap_or(dirs.home_dir()).to_path_buf();
                let chosen = cx.prompt_for_new_path(&dir, Some(&named(&part, cx)));
                cx.spawn(async move |this, cx| {
                    let chosen = match chosen.await {
                        Ok(Ok(chosen)) => chosen,
                        Ok(Err(error)) => return this.update(cx, |mail, cx| mail.failed("mail.file.unsaved", &error.to_string(), cx)).ok(),
                        Err(error) => return this.update(cx, |mail, cx| mail.failed("mail.file.unsaved", &error.to_string(), cx)).ok(),
                    };
                    let Some(path) = chosen else {
                        log::info!("mail: saving a file was cancelled");
                        return Some(());
                    };
                    let target = path.clone();
                    let written = cx
                        .background_spawn(async move { ely_files::export(&target, None, &part.bytes, 0) })
                        .await;
                    this.update(cx, |mail, cx| match written {
                        Ok(_) => mail.saved_file(path, cx),
                        Err(error) => mail.failed("mail.file.unsaved", &error.to_string(), cx),
                    })
                    .ok()
                })
                .detach();
            },
        );
    }

    pub(crate) fn saved_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        log::info!("mail: a file was saved");
        let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        let words = text(cx, "mail.file.saved", &[("name", &name)]);
        match self.root.upgrade() {
            Some(root) => root.update(cx, |root, cx| root.toast_output(words, Severity::Success, path, cx)),
            None => log::info!("mail: the window is closed, so a note is not shown"),
        }
    }

    /// A press on a file opens a working copy.
    pub(crate) fn open_file(&mut self, key: &SharedString, index: usize, cx: &mut Context<Self>) {
        let Ok(id) = key.parse().map(MessageId) else {
            return log::error!("mail: a message key is not a number");
        };
        let waiting = self.reading.files(id).and_then(|files| files.get(index)).is_some_and(|(_, size)| size.is_none());
        if waiting {
            return self.download(id, cx);
        }
        let (dir, open) = (self.copies.join(id.0.to_string()), self.open_copy.clone());
        let unnamed = text(cx, "mail.file.unnamed", &[]).to_string();
        // A copy: what the app saves never touches the mail.
        self.with_store(
            cx,
            move |store| {
                let part = part(store, id, index)?;
                let name = crate::migrate::file_name_of(part.name.as_deref().unwrap_or(&unnamed), &unnamed);
                let copy = ely_files::write_attachment(&name, &part.bytes, &dir).map_err(|err| Error::Store(err.to_string()))?;
                // From mail: the system warns before running it.
                ely_platform::apps::from_internet(&copy).map_err(|err| Error::Store(err.to_string()))?;
                Ok(copy)
            },
            |mail, copy, cx| {
                let copy = match copy {
                    Ok(copy) => copy,
                    Err(error) => return mail.failed("mail.file.unopened", &error, cx),
                };
                // A program from mail never runs on one press.
                if ely_platform::apps::runs_code(&copy) {
                    log::warn!("mail: a file that runs code was kept unopened");
                    return mail.toast(text(cx, "mail.file.unsafe", &[]), Severity::Warning, cx);
                }
                cx.spawn(async move |this, cx| {
                    let opened = cx.background_spawn(async move { open(&copy) }).await;
                    match opened {
                        Ok(()) => log::info!("mail: a file's copy opened"),
                        Err(error) => {
                            this.update(cx, |mail, cx| mail.failed("mail.file.unopened", &error.to_string(), cx)).ok();
                        }
                    }
                })
                .detach();
            },
        );
    }
}
