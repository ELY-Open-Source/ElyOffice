//! The shell's dialogs: unsaved, conflict, password (FILE-05).

use std::{path::PathBuf, rc::Rc};

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::TextInput,
    overlays::{Close, Dialog as Card, PromptDialog},
};
use gpui::{AnyElement, App, AppContext as _, Context, Entity, IntoElement, SharedString, Window};

use crate::files::file_name;
use crate::{After, Closing, Host, Opening, Saving, Source, Tabs, text};

pub enum Dialog {
    Unsaved {
        tab: u64,
        after: After,
    },
    Conflict {
        tab: u64,
        then: Option<After>,
    },
    Password {
        path: PathBuf,
        wrong: bool,
        /// An untitled copy, not the user's file.
        copy: bool,
        field: Entity<TextInput>,
    },
}

pub trait Dialogs: Host {
    /// Asks for `path`'s password; `wrong` when one just failed.
    fn ask_password(
        &mut self,
        path: PathBuf,
        wrong: bool,
        copy: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        log::info!(
            "office: {} is locked, a password failed: {wrong}",
            path.display()
        );
        if self.core().dialog.is_some() {
            return self.core_mut().locked.push_back((path, wrong, copy));
        }
        let field = cx.new(|cx| TextInput::new(window, cx).masked());
        self.core_mut().dialog = Some(Dialog::Password { path, wrong, copy, field });
        cx.notify();
    }

    /// Clears the dialog; a waiting locked file asks next.
    fn dialog_done(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.core_mut().dialog = None;
        if let Some((path, wrong, copy)) = self.core_mut().locked.pop_front() {
            self.ask_password(path, wrong, copy, window, cx);
        }
        cx.notify();
    }

    /// The shell's open dialog, if any.
    fn core_dialog(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let this = cx.entity();
        let dismiss = {
            let this = this.clone();
            move |window: &mut Window, cx: &mut App| {
                this.update(cx, |host, cx| host.dialog_done(window, cx))
            }
        };
        let name = |tab| {
            self.tab(tab)
                .map(|tab| tab.name.clone())
                .unwrap_or_default()
        };
        let card = match self.core().dialog.as_ref()? {
            Dialog::Password { path, wrong, copy, field } => {
                let prompt = password(this, (path, *wrong, *copy, field), dismiss, cx);
                return Some(prompt.into_any_element());
            }
            Dialog::Unsaved { tab, after } => {
                let (tab, after) = (*tab, *after);
                let title = text(cx, "office.unsaved.title", &[("name", &name(tab))]);
                let owner = this.clone();
                // Cancel stops the whole close.
                let cancelled = move |window: &mut Window, cx: &mut App| {
                    owner.update(cx, |host, cx| {
                        host.close_stopped(cx);
                        host.dialog_done(window, cx)
                    })
                };
                Card::new("office-unsaved", title, cancelled)
                    .detail(text(cx, "office.unsaved.detail", &[]))
                    .action(cancel(cx))
                    .action(choice(
                        &this,
                        "office-unsaved-discard",
                        text(cx, "office.unsaved.discard", &[]),
                        ButtonVariant::Danger,
                        move |host, window, cx| {
                            host.close_tab(tab, cx);
                            host.proceed(after, window, cx);
                        },
                    ))
                    .action(choice(
                        &this,
                        "office-unsaved-save",
                        text(cx, "office.unsaved.save", &[]),
                        ButtonVariant::Primary,
                        move |host, window, cx| host.save(tab, Some(after), window, cx),
                    ))
            }
            Dialog::Conflict { tab, then } => {
                let (tab, then) = (*tab, *then);
                let title = text(cx, "office.conflict.title", &[("name", &name(tab))]);
                let owner = this.clone();
                let cancelled = move |window: &mut Window, cx: &mut App| {
                    owner.update(cx, |host, cx| {
                        if then.is_some() {
                            host.close_stopped(cx);
                        }
                        host.dialog_done(window, cx)
                    })
                };
                Card::new("office-conflict", title, cancelled)
                    .detail(text(cx, "office.conflict.detail", &[]))
                    .action(cancel(cx))
                    .action(choice(
                        &this,
                        "office-conflict-reload",
                        text(cx, "office.conflict.reload", &[]),
                        ButtonVariant::Secondary,
                        move |host, _, cx| {
                            // Reloading ends the close that waited.
                            if then.is_some() {
                                host.close_stopped(cx);
                            }
                            host.reload(tab, cx)
                        },
                    ))
                    .action(choice(
                        &this,
                        "office-conflict-copy",
                        text(cx, "office.conflict.copy", &[]),
                        ButtonVariant::Secondary,
                        move |host, _, cx| host.save_as(tab, then, cx),
                    ))
                    .action(choice(
                        &this,
                        "office-conflict-keep",
                        text(cx, "office.conflict.keep", &[]),
                        ButtonVariant::Primary,
                        move |host, window, cx| host.keep_mine(tab, then, window, cx),
                    ))
            }
        };
        Some(card.into_any_element())
    }
}

impl<H: Host> Dialogs for H {}

/// A dialog's Cancel button.
pub fn cancel(cx: &App) -> impl FnOnce(Close) -> Button + 'static {
    let label = text(cx, "office.cancel", &[]);
    move |close| {
        Button::new("office-dialog-cancel", label).on_click(move |_, window, cx| close(window, cx))
    }
}

/// A dialog button that closes the dialog, then runs `run`.
fn choice<H: Host>(
    this: &Entity<H>,
    id: &'static str,
    label: SharedString,
    variant: ButtonVariant,
    run: impl Fn(&mut H, &mut Window, &mut Context<H>) + 'static,
) -> impl FnOnce(Close) -> Button + 'static {
    let this = this.clone();
    let run = Rc::new(run);
    move |_| {
        Button::new(id, label)
            .variant(variant)
            .on_click(move |_, window, cx| {
                let run = run.clone();
                this.update(cx, |host, cx| {
                    host.core_mut().dialog = None;
                    run(host, window, cx);
                    if host.core().dialog.is_none() {
                        host.dialog_done(window, cx);
                    }
                })
            })
    }
}

/// The masked prompt for a locked file.
fn password<H: Host>(
    this: Entity<H>,
    (path, wrong, copy, field): (&PathBuf, bool, bool, &Entity<TextInput>),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> PromptDialog {
    let name = file_name(path);
    let title = text(cx, "office.locked.title", &[("name", &name)]);
    let empty = text(cx, "office.locked.empty", &[]);
    // Each ask its own dialog, so focus is its own.
    let id = ("office-locked", field.entity_id().as_u64());
    let mut prompt = PromptDialog::new(id, title, field, dismiss)
        .label(text(cx, "office.locked.label", &[]))
        .submit(text(cx, "office.locked.open", &[]));
    if wrong {
        prompt = prompt.label(text(cx, "office.locked.wrong", &[]));
    }
    let path = path.clone();
    prompt
        .check(move |text| {
            if text.is_empty() {
                Err(empty.clone())
            } else {
                Ok(())
            }
        })
        // The dialog closed first; the next ask may show.
        .on_submit(move |text, _, cx| {
            let (path, password) = (path.clone(), text.to_string());
            this.update(cx, |host, cx| match copy {
                true => host.load(Source::Locked(path, password), cx),
                false => host.open_with(path, password, cx),
            })
        })
}
