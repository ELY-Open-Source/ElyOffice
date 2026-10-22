//! Mail's rows on the settings storage page (C12.22, §18.3).

use std::{collections::HashSet, path::PathBuf, rc::Rc};

use ely_gpui_component::{
    buttons::Button,
    overlays::ConfirmDialog,
    primitives::Severity,
    settings::SettingsRow,
    typography::format,
};
use ely_mail_engine::{Error, Sizes, Store};
use gpui::{AnyElement, App, Context, IntoElement, PathPromptOptions, WeakEntity};

use crate::{
    mail::{Mail, text},
    migrate::file_stem,
};

/// The sizes last read; a clean up awaiting its yes.
#[derive(Default)]
pub struct Cache {
    pub sizes: Option<Result<Sizes, String>>,
    pub asking: bool,
}

/// Every settings window shows mail's rows.
pub fn install(mail: WeakEntity<Mail>, cx: &mut App) {
    let (read, rows, dialog) = (mail.clone(), mail.clone(), mail);
    let storage = ely_ui::settings::AppStorage {
        read: Rc::new(move |cx| {
            read.update(cx, |mail, cx| mail.read_sizes(cx)).ok();
        }),
        rows: Rc::new(move |cx| rows.upgrade().map_or_else(Vec::new, |mail| mail.read(cx).storage_rows(&rows, cx))),
        dialog: Rc::new(move |cx| dialog.upgrade().and_then(|mail| mail.read(cx).forget_dialog(&dialog, cx))),
    };
    ely_ui::settings::add_storage_rows(storage, cx);
}

impl Mail {
    pub fn read_sizes(&mut self, cx: &mut Context<Self>) {
        self.with_store(cx, |store| store.sizes(), |mail, sizes, cx| {
            mail.cache.sizes = Some(sizes);
            // The settings window draws these.
            cx.refresh_windows();
        });
    }

    fn storage_rows(&self, this: &WeakEntity<Mail>, cx: &App) -> Vec<AnyElement> {
        let size = |bytes: u64| format::file_size(bytes, false);
        let sizes = match &self.cache.sizes {
            None => return Vec::new(),
            Some(Err(error)) => {
                let words = text(cx, "mail.storage.unread", &[("error", error)]);
                return vec![SettingsRow::new(words).into_any_element()];
            }
            Some(Ok(sizes)) => sizes,
        };
        let this = this.clone();
        let forget = Button::new("mail-storage-forget", text(cx, "mail.storage.forget", &[]))
            .disabled(sizes.originals == 0)
            .on_click({
                let this = this.clone();
                move |_, _, cx| {
                    this.update(cx, |mail, cx| {
                        mail.cache.asking = true;
                        cx.refresh_windows();
                    })
                    .ok();
                }
            });
        let exporting = this.clone();
        let export = Button::new("mail-storage-export", text(cx, "mail.storage.export", &[])).on_click(move |_, _, cx| {
            exporting.update(cx, |mail, cx| mail.pick_export_folder(cx)).ok();
        });
        vec![
            SettingsRow::new(text(cx, "mail.storage.index", &[]))
                .description(text(cx, "mail.storage.index_hint", &[("size", &size(sizes.index)), ("search", &size(sizes.search))]))
                .into_any_element(),
            SettingsRow::new(text(cx, "mail.storage.originals", &[]))
                .description(text(cx, "mail.storage.originals_hint", &[("size", &size(sizes.originals))]))
                .control(forget)
                .into_any_element(),
            self.cap_row(&this, cx),
            SettingsRow::new(text(cx, "mail.storage.all", &[]))
                .description(text(cx, "mail.storage.all_hint", &[]))
                .control(export)
                .into_any_element(),
        ]
    }

    fn forget_dialog(&self, this: &WeakEntity<Mail>, cx: &App) -> Option<AnyElement> {
        if !self.cache.asking {
            return None;
        }
        let (close, go) = (this.clone(), this.clone());
        let dialog = ConfirmDialog::new(
            "mail-storage-ask",
            text(cx, "mail.storage.forget_title", &[]),
            text(cx, "mail.storage.forget_message", &[]),
            move |_, cx| {
                close
                    .update(cx, |mail, cx| {
                        mail.cache.asking = false;
                        cx.refresh_windows();
                    })
                    .ok();
            },
        )
        .confirm(text(cx, "mail.storage.forget_go", &[]))
        .destructive()
        .on_confirm(move |_, cx| {
            go.update(cx, |mail, cx| mail.forget_downloads(cx)).ok();
        });
        Some(dialog.into_any_element())
    }

    pub(crate) fn forget_downloads(&mut self, cx: &mut Context<Self>) {
        self.with_store(cx, |store| store.forget_downloads(), |mail, forgot, cx| {
            match forgot {
                Ok(n) => {
                    let words = ely_ui::i18n::counted(cx, "mail.storage.forgot", n as u64, &[("n", &n.to_string())]);
                    mail.toast(words, Severity::Success, cx);
                }
                Err(error) => mail.failed("mail.storage.unforgotten", &error, cx),
            }
            mail.read_sizes(cx);
            mail.read_letter(cx);
        });
    }

    fn pick_export_folder(&mut self, cx: &mut Context<Self>) {
        let options = PathPromptOptions { files: false, directories: true, multiple: false, prompt: None };
        let picked = cx.prompt_for_paths(options);
        cx.spawn(async move |this, cx| {
            let dir = match picked.await {
                Ok(Ok(Some(mut paths))) => paths.remove(0),
                Ok(Ok(None)) => return log::info!("mail: exporting all mail was cancelled"),
                Ok(Err(error)) => return log::error!("mail: the folder panel failed: {error}"),
                Err(error) => return log::error!("mail: the folder panel went away: {error}"),
            };
            this.update(cx, |mail, cx| mail.export_all(dir, cx)).ok();
        })
        .detach();
    }

    /// One mbox per folder; nothing if any file exists.
    pub fn export_all(&mut self, dir: PathBuf, cx: &mut Context<Self>) {
        self.with_store(cx, move |store| export_all(store, &dir), |mail, done, cx| match done {
            Ok((files, left)) => {
                let args = [("n", files.to_string()), ("left", left.to_string())];
                let args: Vec<(&str, &str)> = args.iter().map(|(key, value)| (*key, value.as_str())).collect();
                let words = ely_ui::i18n::counted(cx, "mail.storage.exported", files as u64, &args);
                mail.toast(words, Severity::Success, cx);
            }
            Err(error) => mail.failed("mail.mbox.unexported", &error, cx),
        });
    }
}

/// Files written and messages left out with only headers.
fn export_all(store: &mut Store, dir: &std::path::Path) -> ely_mail_engine::Result<(usize, usize)> {
    let mut targets = Vec::new();
    let mut accounts = HashSet::new();
    for (id, account) in store.accounts()? {
        let mut taken = HashSet::new();
        let folder = dir.join(unique(&mut accounts, file_stem(Some(&account.address), "account")));
        for each in store.folders(id)? {
            let name = unique(&mut taken, file_stem(Some(&each.shown), "folder"));
            targets.push((each.id, folder.join(format!("{name}.mbox"))));
        }
    }
    if let Some((_, there)) = targets.iter().find(|(_, path)| path.exists()) {
        return Err(Error::Invalid(format!("{} is there already", there.display())));
    }
    let (mut files, mut left) = (0, 0);
    for (mailbox, path) in &targets {
        let parent = path.parent().expect("each target is in an account folder");
        std::fs::create_dir_all(parent).map_err(|err| Error::Store(format!("{}: {err}", parent.display())))?;
        left += store.export_mbox(*mailbox, path, false)?.left;
        files += 1;
    }
    log::info!("mail: {files} folders exported, {left} messages left out");
    Ok((files, left))
}

/// `base`, else `base 2` on: names may clean alike.
fn unique(taken: &mut HashSet<String>, base: String) -> String {
    let mut name = base.clone();
    let mut number = 1;
    while !taken.insert(name.to_lowercase()) {
        number += 1;
        name = format!("{base} {number}");
    }
    name
}
