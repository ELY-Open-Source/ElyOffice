//! Cleaning and moving back, each previewed; lock retries.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::Checkbox,
    overlays::{ConfirmDialog, Dialog},
    theme::ActiveTheme as _,
};
use gpui::{
    AnyElement, App, AppContext as _, Context, InteractiveElement as _, IntoElement,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};

use crate::{
    asks::Ask,
    recover,
    tree::{Tree, text},
    work::Write,
};

/// Each file, then each folder, as the dialog lists them.
fn shown(clean: &recover::Clean) -> Vec<String> {
    clean.files.iter().chain(&clean.dirs).cloned().collect()
}

impl Tree {
    /// Opens the clean dialog; its list comes from Git.
    pub fn ask_clean(&mut self, cx: &mut Context<Self>) {
        let Some(repo) = self.active else { return };
        self.ask = Some(Ask::Clean {
            repo,
            ignored: false,
            preview: None,
        });
        self.preview_clean(cx);
    }

    pub fn clean_ignored(&mut self, on: bool, cx: &mut Context<Self>) {
        if let Some(Ask::Clean {
            ignored, preview, ..
        }) = &mut self.ask
        {
            (*ignored, *preview) = (on, None);
            self.preview_clean(cx);
        }
    }

    /// Only the answer to the dialog as it stands lands.
    fn preview_clean(&mut self, cx: &mut Context<Self>) {
        let Some(git) = self.git.as_ref().ok().cloned() else {
            return;
        };
        let Some(Ask::Clean { repo, ignored, .. }) = &self.ask else {
            return;
        };
        let (repo, ignored) = (*repo, *ignored);
        let Some(root) = self.opened(repo).map(|opened| opened.repo.root.clone()) else {
            return;
        };
        let task = cx.background_spawn(async move { recover::clean_preview(&git, &root, ignored) });
        cx.spawn(async move |this, cx| {
            let read = task.await.map_err(|error| error.to_string());
            this.update(cx, |tree, cx| {
                if let Some(Ask::Clean {
                    repo: now,
                    ignored: still,
                    preview,
                }) = &mut tree.ask
                    && (*now, *still) == (repo, ignored)
                {
                    *preview = Some(read);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Removes exactly the previewed paths, nothing found later.
    fn clean(&mut self, cx: &mut Context<Self>) {
        let Some(Ask::Clean {
            repo,
            ignored,
            preview: Some(Ok(clean)),
        }) = &self.ask
        else {
            return;
        };
        let write = Write::Clean {
            files: clean.files.clone(),
            dirs: clean.dirs.clone(),
            ignored: *ignored,
        };
        let (repo, count) = (*repo, clean.files.len());
        self.ask = None;
        log::info!("tree: cleaning {count} files");
        self.write(repo, write, cx);
    }

    /// Reads what moving the branch to `to` leaves behind.
    pub fn ask_move(&mut self, to: &str, cx: &mut Context<Self>) {
        let Some(git) = self.git.as_ref().ok().cloned() else {
            return;
        };
        let Some(opened) = self.active() else { return };
        let (repo, root, to) = (opened.id, opened.repo.root.clone(), to.to_string());
        let task = cx.background_spawn(async move { recover::plan_move(&git, &root, &to) });
        cx.spawn(async move |this, cx| {
            let plan = task.await;
            this.update(cx, |tree, cx| {
                if tree.active != Some(repo) {
                    return;
                }
                let ask = match plan {
                    Ok(plan) => Ask::Move { repo, plan },
                    Err(error) => {
                        log::warn!("tree: a move plan failed: {}", ely_ipc::Worded::logged(&error));
                        Ask::Failed { title: text(cx, "tree.op.failed", &[]), detail: ely_ui::i18n::error(cx, &error) }
                    }
                };
                tree.offer(ask, cx);
            })
            .ok();
        })
        .detach();
    }

    pub fn recover_dialog(
        &self,
        dismiss: impl Fn(&mut Window, &mut App) + Clone + 'static,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let this = cx.weak_entity();
        let label = |key: &str| text(cx, key, &[]);
        Some(match self.ask.as_ref()? {
            Ask::Clean {
                ignored, preview, ..
            } => {
                let colors = cx.theme().colors.clone();
                let toggle = {
                    let this = this.clone();
                    Checkbox::new("tree-clean-ignored", *ignored)
                        .label(label("tree.clean.ignored"))
                        .on_change(move |on, _, cx| {
                            this.update(cx, |tree, cx| tree.clean_ignored(on, cx)).ok();
                        })
                };
                let (body, ready): (AnyElement, bool) = match preview {
                    None => (
                        div().child(label("tree.clean.reading")).into_any_element(),
                        false,
                    ),
                    Some(Err(error)) => (
                        div()
                            .text_color(colors.danger)
                            .child(error.clone())
                            .into_any_element(),
                        false,
                    ),
                    Some(Ok(clean)) if shown(clean).is_empty() => (
                        div().child(label("tree.clean.nothing")).into_any_element(),
                        false,
                    ),
                    Some(Ok(clean)) => (
                        div()
                            .id("tree-clean-paths")
                            .text_color(colors.fg)
                            .max_h(px(280.0))
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .children(shown(clean).into_iter().map(|path| div().child(path)))
                            .into_any_element(),
                        true,
                    ),
                };
                let count = match preview {
                    Some(Ok(clean)) => shown(clean).len(),
                    _ => 0,
                };
                let (go, cancel) = (
                    text(cx, "tree.clean.go", &[("n", &count.to_string())]),
                    label("tree.cancel"),
                );
                Dialog::new("tree-clean", label("tree.clean.title"), dismiss)
                    .child(toggle)
                    .child(body)
                    .action(move |close| {
                        Button::new("tree-clean-cancel", cancel)
                            .on_click(move |_, window, cx| close(window, cx))
                    })
                    .action(move |_| {
                        Button::new("tree-clean-go", go)
                            .variant(ButtonVariant::Danger)
                            .disabled(!ready)
                            .on_click(move |_, _, cx| {
                                this.update(cx, |tree, cx| tree.clean(cx)).ok();
                            })
                    })
                    .into_any_element()
            }
            Ask::Move { repo, plan } => {
                let short = |oid: &str| oid[..oid.len().min(7)].to_string();
                let left = match plan.left.len() {
                    0 => label("tree.move.nothing"),
                    n => ely_ui::i18n::counted(
                        cx,
                        "tree.move.left",
                        n as u64,
                        &[("n", &n.to_string()), ("commits", &plan.left.join("\n"))],
                    ),
                };
                let what = match &plan.branch {
                    Some(branch) => branch.trim_start_matches("refs/heads/").to_string(),
                    None => label("tree.move.detached").to_string(),
                };
                let message = text(
                    cx,
                    "tree.move.message",
                    &[
                        ("what", &what),
                        ("from", &short(&plan.head)),
                        ("to", &short(&plan.to)),
                        ("left", &left),
                    ],
                );
                let (repo, write) = (
                    *repo,
                    Write::Move {
                        branch: plan.branch.clone(),
                        head: plan.head.clone(),
                        to: plan.to.clone(),
                    },
                );
                ConfirmDialog::new("tree-move", label("tree.move.title"), message, dismiss)
                    .confirm(label("tree.move.go"))
                    .destructive()
                    .on_confirm(move |_, cx| {
                        this.update(cx, |tree, cx| tree.write(repo, write.clone(), cx))
                            .ok();
                    })
                    .into_any_element()
            }
            Ask::Locked {
                repo,
                write,
                detail,
            } => {
                let (repo, write) = (*repo, write.clone());
                let message = format!("{}\n\n{detail}", label("tree.locked.message"));
                ConfirmDialog::new("tree-locked", label("tree.locked.title"), message, dismiss)
                    .confirm(label("tree.locked.retry"))
                    .on_confirm(move |_, cx| {
                        this.update(cx, |tree, cx| tree.write(repo, write.clone(), cx))
                            .ok();
                    })
                    .into_any_element()
            }
            _ => return None,
        })
    }
}
