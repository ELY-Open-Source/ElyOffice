//! A file's history or blame, over the middle (TREE-06).

use std::rc::Rc;

use ely_gpui_component::git::Commit;
use gpui::{AppContext as _, Context};

use crate::{
    blame::{self, Blamed},
    commits::{self, Touch},
    hist,
    shown::{self, View},
    tree::Tree,
};

/// A commit and its added and removed line counts.
type Row = (Commit, Option<(usize, usize)>);

pub enum Look {
    FileLog {
        repo: u64,
        /// Unique per opening, so late reads are dropped.
        serial: u64,
        path: String,
        touches: Option<Result<Vec<Touch>, String>>,
        rows: Rc<Vec<Row>>,
        picked: Option<usize>,
        view: Option<Result<View, String>>,
    },
    Blame {
        repo: u64,
        serial: u64,
        path: String,
        read: Option<Result<Blamed, String>>,
    },
}

impl Look {
    pub fn repo(&self) -> u64 {
        match self {
            Look::FileLog { repo, .. } | Look::Blame { repo, .. } => *repo,
        }
    }
}

impl Tree {
    /// Opens `path`'s commits, newest first, through renames.
    pub fn file_log(&mut self, path: &str, cx: &mut Context<Self>) {
        let Some(git) = self.git(cx) else { return };
        let Some((repo, root)) = self
            .active()
            .map(|opened| (opened.id, opened.repo.root.clone()))
        else {
            return;
        };
        let path = path.to_string();
        log::info!("tree: file history of {path}");
        self.looks += 1;
        let serial = self.looks;
        self.look = Some(Look::FileLog {
            repo,
            serial,
            path: path.clone(),
            touches: None,
            rows: Rc::default(),
            picked: None,
            view: None,
        });
        let task = cx.background_spawn({
            let path = path.clone();
            async move { commits::file(&git, &root, &path) }
        });
        cx.spawn(async move |this, cx| {
            let read = task.await.map_err(|error| error.to_string());
            this.update(cx, |tree, cx| {
                let Some(Look::FileLog {
                    serial: now,
                    touches,
                    rows,
                    ..
                }) = &mut tree.look
                else {
                    return;
                };
                if *now != serial {
                    return;
                }
                if let Ok(read) = &read {
                    let logged: Vec<_> = read.iter().map(|touch| touch.commit.clone()).collect();
                    *rows = Rc::new(
                        hist::rows(&logged)
                            .into_iter()
                            .zip(read.iter().map(|touch| touch.lines))
                            .collect(),
                    );
                }
                *touches = Some(read);
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Shows the file's change in one of its commits.
    pub fn pick_touch(&mut self, at: usize, cx: &mut Context<Self>) {
        let Some(git) = self.git(cx) else { return };
        let Some(Look::FileLog {
            repo,
            serial,
            touches: Some(Ok(touches)),
            picked,
            view,
            ..
        }) = &mut self.look
        else {
            return;
        };
        let Some(touch) = touches.get(at).cloned() else {
            return;
        };
        (*picked, *view) = (Some(at), None);
        let (repo, serial) = (*repo, *serial);
        let Some(root) = self.opened(repo).map(|opened| opened.repo.root.clone()) else {
            return;
        };
        let task = cx.background_spawn(async move {
            let commit = &touch.commit;
            crate::detail::change(
                &git,
                &root,
                &commit.id,
                &commit.parents,
                &touch.path,
                touch.from.clone(),
            )
        });
        cx.spawn(async move |this, cx| {
            let found = task.await;
            this.update(cx, |tree, cx| {
                let Some(Look::FileLog {
                    serial: now,
                    picked: Some(was),
                    view,
                    ..
                }) = &mut tree.look
                else {
                    return;
                };
                if *now == serial && *was == at {
                    *view = Some(
                        found
                            .and_then(shown::view)
                            .map_err(|error| error.to_string()),
                    );
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Who last changed each line of `path`.
    pub fn blame(&mut self, path: &str, cx: &mut Context<Self>) {
        let Some(git) = self.git(cx) else { return };
        let Some((repo, root)) = self
            .active()
            .map(|opened| (opened.id, opened.repo.root.clone()))
        else {
            return;
        };
        let path = path.to_string();
        self.looks += 1;
        let serial = self.looks;
        self.look = Some(Look::Blame {
            repo,
            serial,
            path: path.clone(),
            read: None,
        });
        let task = cx.background_spawn({
            let path = path.clone();
            async move { blame::read(&git, &root, &path) }
        });
        cx.spawn(async move |this, cx| {
            let read = task.await;
            this.update(cx, |tree, cx| {
                let read = read.map_err(|error| ely_ui::i18n::error(cx, &error).to_string());
                let Some(Look::Blame {
                    serial: now,
                    read: slot,
                    ..
                }) = &mut tree.look
                else {
                    return;
                };
                if *now == serial {
                    *slot = Some(read);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Closes a file's history or blame.
    pub fn close_look(&mut self, cx: &mut Context<Self>) {
        self.look = None;
        cx.notify();
    }
}
