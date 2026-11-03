//! The window's dialogs: clone, discard, failures, identity.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::{Checkbox, Input, TextInput},
    overlays::{AlertDialog, ConfirmDialog, Dialog, PromptDialog},
    primitives::Severity,
};
use gpui::{
    AnyElement, App, Context, Entity, IntoElement, ParentElement as _, SharedString, Styled as _,
    Window, div,
};

use crate::{
    status::{Entry, State},
    tree::{Tree, text},
    work::Write,
};

/// What a dialog asks.
pub enum Ask {
    Url(Entity<TextInput>),
    Discard {
        repo: u64,
        entry: Entry,
    },
    /// A failure with Git's whole account, such as a hook's.
    Failed {
        title: SharedString,
        detail: SharedString,
    },
    /// A command that waits for a yes, this once.
    Confirm {
        repo: u64,
        title: SharedString,
        message: SharedString,
        go: SharedString,
        write: Write,
    },
    NewBranch {
        /// The repository it was opened for.
        repo: u64,
        name: Entity<TextInput>,
        at: Option<String>,
        switch: bool,
    },
    NewTag {
        /// The repository it was opened for.
        repo: u64,
        name: Entity<TextInput>,
        message: Entity<TextInput>,
        at: Option<String>,
    },
    Rename {
        /// The repository it was opened for.
        repo: u64,
        old: String,
        name: Entity<TextInput>,
    },
    Identity {
        /// The repository it was opened for.
        repo: u64,
        name: Entity<TextInput>,
        email: Entity<TextInput>,
        global: bool,
    },
    Stash {
        /// The repository it was opened for.
        repo: u64,
        message: Entity<TextInput>,
        untracked: bool,
    },
    /// A new remote, or an old one's name and URL.
    Remote {
        /// The repository it was opened for.
        repo: u64,
        old: Option<(String, String)>,
        name: Entity<TextInput>,
        url: Entity<TextInput>,
    },
    Worktree {
        /// The repository it was opened for.
        repo: u64,
        path: std::path::PathBuf,
        branch: Entity<TextInput>,
        new: bool,
    },
    /// A forced push, with what it would overwrite.
    Force {
        repo: u64,
        plan: crate::remotes::Force,
    },
    /// One of several names, each a button.
    Choose {
        /// The repository it was opened for.
        repo: u64,
        title: SharedString,
        choices: Vec<String>,
        then: Choice,
    },
    /// What a clean would remove, read before the yes.
    Clean {
        repo: u64,
        ignored: bool,
        preview: Option<Result<crate::recover::Clean, String>>,
    },
    /// HEAD's branch to a reflog commit, previewed.
    Move {
        repo: u64,
        plan: crate::recover::Move,
    },
    /// Another Git holds a lock; retry, never delete it.
    Locked {
        repo: u64,
        write: Write,
        detail: SharedString,
    },
}

/// What a chosen name is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Push this branch there and track it.
    Publish(String),
    /// Track that remote branch from this one.
    Upstream(String),
}

impl Tree {
    /// Opens a read's dialog unless another opened since.
    pub fn offer(&mut self, ask: Ask, cx: &mut Context<Self>) {
        match (&self.ask, ask) {
            (None, ask) => self.ask = Some(ask),
            // The read logged its error; a toast says it.
            (Some(_), Ask::Failed { title, detail }) => {
                self.toast_unlogged(format!("{title}: {detail}").into(), Severity::Danger, cx)
            }
            (Some(_), _) => log::info!("tree: a later dialog stands; read dropped"),
        }
        cx.notify();
    }

    pub fn dialog(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let this = cx.weak_entity();
        let dismiss = {
            let this = this.clone();
            move |_: &mut Window, cx: &mut App| {
                this.update(cx, |tree, cx| {
                    tree.ask = None;
                    cx.notify();
                })
                .ok();
            }
        };
        match self.ask.as_ref()? {
            Ask::Url(field) => {
                let empty = text(cx, "tree.clone.empty", &[]);
                let prompt = PromptDialog::new(
                    "tree-clone-url",
                    text(cx, "tree.clone.title", &[]),
                    field,
                    dismiss,
                )
                .label(text(cx, "tree.clone.url", &[]))
                .submit(text(cx, "tree.clone.next", &[]))
                .check(move |url| {
                    if url.trim().is_empty() {
                        Err(empty.clone())
                    } else {
                        Ok(())
                    }
                })
                .on_submit(move |url, _, cx| {
                    let url = url.trim().to_string();
                    this.update(cx, |tree, cx| tree.pick_clone_folder(url, cx))
                        .ok();
                });
                Some(prompt.into_any_element())
            }
            Ask::Discard { repo, entry } => {
                let key = if entry.state == State::Untracked {
                    "tree.discard.untracked"
                } else {
                    "tree.discard.tracked"
                };
                let message = text(cx, key, &[("path", &entry.path)]);
                let (repo, write) = (*repo, Write::Discard(entry.clone()));
                let confirm = ConfirmDialog::new(
                    "tree-discard",
                    text(cx, "tree.discard.title", &[("path", &entry.path)]),
                    message,
                    dismiss,
                )
                .confirm(text(cx, "tree.discard.go", &[]))
                .destructive()
                .on_confirm(move |_, cx| {
                    this.update(cx, |tree, cx| tree.write(repo, write.clone(), cx))
                        .ok();
                });
                Some(confirm.into_any_element())
            }
            Ask::Failed { title, detail } => Some(
                AlertDialog::new(
                    "tree-failed",
                    Severity::Danger,
                    title.clone(),
                    detail.clone(),
                    dismiss,
                )
                .label(text(cx, "tree.ok", &[]))
                .into_any_element(),
            ),
            Ask::Confirm {
                repo,
                title,
                message,
                go,
                write,
            } => {
                let (repo, write) = (*repo, write.clone());
                let confirm =
                    ConfirmDialog::new("tree-confirm", title.clone(), message.clone(), dismiss)
                        .confirm(go.clone())
                        .destructive()
                        .on_confirm(move |_, cx| {
                            this.update(cx, |tree, cx| tree.write(repo, write.clone(), cx))
                                .ok();
                        });
                Some(confirm.into_any_element())
            }
            Ask::NewBranch { name, switch, .. } => {
                let label = |key: &str| text(cx, key, &[]);
                let rows = DescriptionList::new().item(
                    label("tree.branch.name"),
                    div().w_full().child(Input::new(name)),
                );
                let toggle = {
                    let this = this.clone();
                    Checkbox::new("tree-branch-switch", *switch)
                        .label(label("tree.branch.switch"))
                        .on_change(move |on, _, cx| {
                            this.update(cx, |tree, cx| {
                                if let Some(Ask::NewBranch { switch, .. }) = &mut tree.ask {
                                    *switch = on;
                                    cx.notify();
                                }
                            })
                            .ok();
                        })
                };
                let (title, go) = (label("tree.branch.title"), label("tree.branch.create"));
                Some(self.form(
                    "tree-new-branch",
                    (title, go),
                    rows,
                    Some(toggle.into_any_element()),
                    dismiss,
                    cx,
                ))
            }
            Ask::NewTag { name, message, .. } => {
                let label = |key: &str| text(cx, key, &[]);
                let rows = DescriptionList::new()
                    .item(
                        label("tree.tag.name"),
                        div().w_full().child(Input::new(name)),
                    )
                    .item(
                        label("tree.tag.message"),
                        div().w_full().child(Input::new(message)),
                    );
                let (title, go) = (label("tree.tag.title"), label("tree.tag.create"));
                Some(self.form("tree-new-tag", (title, go), rows, None, dismiss, cx))
            }
            Ask::Rename { old, name, .. } => {
                let label = |key: &str| text(cx, key, &[]);
                let rows = DescriptionList::new().item(
                    label("tree.branch.name"),
                    div().w_full().child(Input::new(name)),
                );
                let (title, go) = (
                    text(cx, "tree.rename.title", &[("name", old)]),
                    label("tree.rename.go"),
                );
                Some(self.form("tree-rename", (title, go), rows, None, dismiss, cx))
            }
            Ask::Stash { .. }
            | Ask::Remote { .. }
            | Ask::Worktree { .. }
            | Ask::Force { .. }
            | Ask::Choose { .. } => Some(self.place_dialog(dismiss, cx)?),
            Ask::Clean { .. } | Ask::Move { .. } | Ask::Locked { .. } => {
                Some(self.recover_dialog(dismiss, cx)?)
            }
            Ask::Identity {
                name,
                email,
                global,
                ..
            } => {
                let label = |key: &str| text(cx, key, &[]);
                let rows = DescriptionList::new()
                    .item(
                        label("tree.identity.name"),
                        div().w_full().child(Input::new(name)),
                    )
                    .item(
                        label("tree.identity.email"),
                        div().w_full().child(Input::new(email)),
                    );
                let scope = {
                    let this = this.clone();
                    Checkbox::new("tree-identity-global", *global)
                        .label(label("tree.identity.global"))
                        .on_change(move |on, _, cx| {
                            this.update(cx, |tree, cx| {
                                if let Some(Ask::Identity { global, .. }) = &mut tree.ask {
                                    *global = on;
                                    cx.notify();
                                }
                            })
                            .ok();
                        })
                };
                let (save, cancel) = (label("tree.identity.save"), label("tree.cancel"));
                let card = Dialog::new("tree-identity", label("tree.identity.title"), dismiss)
                    .child(rows)
                    .child(scope)
                    .action(move |close| {
                        Button::new("tree-identity-cancel", cancel)
                            .on_click(move |_, window, cx| close(window, cx))
                    })
                    .action(move |_| {
                        Button::new("tree-identity-save", save)
                            .variant(ButtonVariant::Primary)
                            .on_click(move |_, _, cx| {
                                this.update(cx, |tree, cx| tree.set_identity(cx)).ok();
                            })
                    });
                Some(card.into_any_element())
            }
        }
    }

    /// A dialog of fields whose go button answers it.
    pub(crate) fn form(
        &self,
        id: &'static str,
        (title, go): (gpui::SharedString, gpui::SharedString),
        rows: DescriptionList,
        extra: Option<AnyElement>,
        dismiss: impl Fn(&mut Window, &mut App) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let this = cx.weak_entity();
        let cancel = text(cx, "tree.cancel", &[]);
        Dialog::new(id, title, dismiss)
            .child(rows)
            .children(extra)
            .action(move |close| {
                Button::new("tree-form-cancel", cancel)
                    .on_click(move |_, window, cx| close(window, cx))
            })
            .action(move |_| {
                Button::new("tree-form-go", go)
                    .variant(ButtonVariant::Primary)
                    .on_click(move |_, _, cx| {
                        this.update(cx, |tree, cx| tree.answer(cx)).ok();
                    })
            })
            .into_any_element()
    }
}
