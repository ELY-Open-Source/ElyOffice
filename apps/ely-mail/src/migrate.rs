//! Message files and mbox files, in and out (C12.20, MAIL-13).

use std::{fs::File, io::BufReader, path::PathBuf};

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    overlays::ConfirmDialog,
    primitives::Severity,
    theme::ControlSize,
};
use ely_mail_engine::{AccountId, Error, MailboxId, MessageId};
use gpui::{
    AnyElement, App, AppContext as _, Context, InteractiveElement as _, IntoElement, ParentElement as _, PathPromptOptions,
    SharedString, Window, div,
};

use crate::{
    boxes::Open,
    mail::{Mail, text},
};

/// An imported folder whose removal waits for an answer.
#[derive(Clone)]
pub struct Unimporting {
    pub account: AccountId,
    pub mailbox: MailboxId,
    pub name: SharedString,
}

/// A name a file system takes, from a subject.
fn file_name(subject: Option<&str>, fallback: &str, extension: &str) -> String {
    format!("{}.{extension}", file_stem(subject, fallback))
}

/// `words` as a file name without extension.
pub(crate) fn file_stem(words: Option<&str>, fallback: &str) -> String {
    let cleaned: String = words
        .unwrap_or("")
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .take(80)
        .collect();
    let cleaned = cleaned.trim().trim_end_matches('.');
    let base = cleaned.split('.').next().unwrap_or_default().trim_end().to_ascii_uppercase();
    let numbered = matches!(base.as_bytes(), [b'C', b'O', b'M', b'1'..=b'9'] | [b'L', b'P', b'T', b'1'..=b'9']);
    let device = numbered || ["CON", "PRN", "AUX", "NUL"].contains(&base.as_str());
    match (cleaned.is_empty(), device) {
        (true, _) => fallback.to_owned(),
        // Windows reads these as devices.
        (false, true) => format!("_{cleaned}"),
        (false, false) => cleaned.to_owned(),
    }
}

/// A file's own name, cleaned, its extension kept.
pub(crate) fn file_name_of(name: &str, fallback: &str) -> String {
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem, file_stem(Some(extension), "")),
        _ => (name, String::new()),
    };
    match extension.is_empty() {
        true => file_stem(Some(stem), fallback),
        false => format!("{}.{extension}", file_stem(Some(stem), fallback)),
    }
}

impl Mail {
    /// The original as a file; a partial message downloads first.
    pub fn save_eml(&mut self, id: MessageId, cx: &mut Context<Self>) {
        let read = move |store: &mut ely_mail_engine::Store| {
            let stored = store.get(id)?;
            let whole = match stored.missing.is_empty() {
                true => store.original(id)?,
                false => None,
            };
            Ok((stored.subject, whole))
        };
        self.with_store(cx, read, move |mail, read, cx| {
            let (subject, whole) = match read {
                Ok(read) => read,
                Err(error) => return mail.failed("mail.eml.unsaved", &error, cx),
            };
            let Some(bytes) = whole else {
                mail.toast(text(cx, "mail.eml.incomplete", &[]), Severity::Info, cx);
                return mail.download(id, cx);
            };
            let name = file_name(subject.as_deref(), "message", "eml");
            mail.save_new(name, "mail.eml.unsaved", cx, move |path| {
                ely_files::export(path, None, &bytes, 0).map(|_| 0).map_err(|err| err.to_string())
            });
        });
    }

    /// Asks where, then writes; `write` counts messages left out.
    fn save_new(
        &mut self,
        name: String,
        failed: &'static str,
        cx: &mut Context<Self>,
        write: impl FnOnce(&std::path::Path) -> Result<usize, String> + Send + 'static,
    ) {
        let Some(dirs) = directories::UserDirs::new() else {
            return self.failed(failed, &ely_ui::i18n::text(cx, "error.no_home", &[]), cx);
        };
        let dir = dirs.document_dir().unwrap_or(dirs.home_dir()).to_path_buf();
        let chosen = cx.prompt_for_new_path(&dir, Some(&name));
        cx.spawn(async move |this, cx| {
            let path = match chosen.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) => return log::info!("mail: saving was cancelled"),
                Ok(Err(error)) => return log::error!("mail: the save panel failed: {error}"),
                Err(error) => return log::error!("mail: the save panel went away: {error}"),
            };
            let target = path.clone();
            let written = cx.background_spawn(async move { write(&target) }).await;
            this.update(cx, |mail, cx| match written {
                Ok(left) => {
                    mail.saved_file(path, cx);
                    if left > 0 {
                        let n = left as u64;
                        let words = ely_ui::i18n::counted(cx, "mail.mbox.partial", n, &[("n", &n.to_string())]);
                        mail.toast(words, Severity::Warning, cx);
                    }
                }
                Err(error) => mail.failed(failed, &error, cx),
            })
            .ok();
        })
        .detach();
    }

    pub fn pick_mbox(&mut self, account: AccountId, cx: &mut Context<Self>) {
        let options = PathPromptOptions { files: true, directories: false, multiple: false, prompt: None };
        let picked = cx.prompt_for_paths(options);
        cx.spawn(async move |this, cx| {
            let path = match picked.await {
                Ok(Ok(Some(mut paths))) => paths.remove(0),
                Ok(Ok(None)) => return log::info!("mail: importing an mbox file was cancelled"),
                Ok(Err(error)) => return log::error!("mail: the import panel failed: {error}"),
                Err(error) => return log::error!("mail: the import panel went away: {error}"),
            };
            this.update(cx, |mail, cx| mail.import_mbox(account, path, cx)).ok();
        })
        .detach();
    }

    /// A new folder named for the file; never a server.
    pub fn import_mbox(&mut self, account: AccountId, path: PathBuf, cx: &mut Context<Self>) {
        let name = path.file_stem().map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
        self.toast(text(cx, "mail.mbox.importing", &[("name", &name)]), Severity::Info, cx);
        let shown = name.clone();
        let work = move |store: &mut ely_mail_engine::Store| {
            let file = File::open(&path).map_err(|err| Error::Store(format!("{}: {err}", path.display())))?;
            store.import_mbox(account, &name, BufReader::new(file))
        };
        self.with_store(cx, work, move |mail, done, cx| match done {
            Ok(unpacked) => {
                let n = unpacked.messages as u64;
                let args = [("n", n.to_string()), ("name", shown)];
                let args: Vec<(&str, &str)> = args.iter().map(|(key, value)| (*key, value.as_str())).collect();
                let words = ely_ui::i18n::counted(cx, "mail.mbox.imported", n, &args);
                mail.toast(words, Severity::Success, cx);
                mail.read_synced(account, cx);
            }
            Err(error) => mail.failed("mail.mbox.unimported", &error, cx),
        });
    }

    pub fn export_mbox(&mut self, mailbox: MailboxId, name: &str, cx: &mut Context<Self>) {
        let Ok(store) = self.store.clone() else {
            return self.failed("mail.mbox.unexported", &ely_ui::i18n::text(cx, "error.no_store", &[]), cx);
        };
        let file = file_name(Some(name), "mail", "mbox");
        self.save_new(file, "mail.mbox.unexported", cx, move |path| {
            let packed = ely_mail_engine::sync::lock(&store)
                .and_then(|store| store.export_mbox(mailbox, path, true))
                .map_err(|err| err.to_string())?;
            Ok(packed.left)
        });
    }

    pub fn ask_remove_imported(&mut self, account: AccountId, mailbox: MailboxId, name: SharedString, cx: &mut Context<Self>) {
        self.unimporting = Some(Unimporting { account, mailbox, name });
        cx.notify();
    }

    fn remove_imported(&mut self, gone: Unimporting, cx: &mut Context<Self>) {
        self.with_store(cx, move |store| store.remove_imported(gone.mailbox), move |mail, done, cx| match done {
            Ok(()) => {
                if mail.open == Open::Mailbox(gone.account, gone.mailbox) {
                    mail.open_box(Open::Inboxes, cx);
                }
                mail.read_synced(gone.account, cx);
            }
            Err(error) => mail.failed("mail.mbox.unremoved", &error, cx),
        });
    }

    pub fn unimport_dialog(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let gone = self.unimporting.clone()?;
        let this = cx.weak_entity();
        let dismiss = {
            let this = this.clone();
            move |_: &mut Window, cx: &mut App| {
                this.update(cx, |mail, cx| {
                    mail.unimporting = None;
                    cx.notify();
                })
                .ok();
            }
        };
        let args = [("name", gone.name.as_ref())];
        let dialog = ConfirmDialog::new(
            "mail-unimport",
            text(cx, "mail.mbox.remove_title", &args),
            text(cx, "mail.mbox.remove_message", &args),
            dismiss,
        )
        .confirm(text(cx, "mail.mbox.remove_go", &[]))
        .destructive()
        .on_confirm(move |_, cx| {
            let gone = gone.clone();
            this.update(cx, |mail, cx| mail.remove_imported(gone, cx)).ok();
        });
        Some(dialog.into_any_element())
    }

    /// The account page's way in for an mbox file.
    pub fn import_button(&self, account: AccountId, cx: &mut Context<Self>) -> AnyElement {
        let this = cx.weak_entity();
        let button = Button::new("mail-mbox-import", text(cx, "mail.mbox.import", &[]))
            .variant(ButtonVariant::Secondary)
            .size(ControlSize::Sm)
            .on_click(move |_, _, cx| {
                this.update(cx, |mail, cx| mail.pick_mbox(account, cx)).ok();
            });
        div().debug_selector(|| "mail-mbox-import".into()).child(button).into_any_element()
    }
}
