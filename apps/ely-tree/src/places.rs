//! Stashes, remotes, worktrees, submodules, LFS (TREE-10 to 12).

use std::{
    io,
    path::{Path, PathBuf},
};

use ely_gpui_component::{forms::TextInput, git::StashAction, primitives::Severity};
use gpui::{AppContext as _, Context, Window};

use crate::{
    asks::Ask,
    git::Git,
    hist::{Picked, Whose},
    recover,
    remotes::{self, Remote},
    stash::{self, Stash},
    structure::{self, LfsFile, Submodule, Worktree},
    tree::{Tree, text},
    work::Write,
};

/// The Repository panel's lists for one repository.
#[derive(Default)]
pub struct Places {
    pub read: Option<Read>,
    /// Reads started; only the latest lands.
    pub asks: u64,
    /// The stash shown in the middle.
    pub pick: Picked,
}

/// Each list apart; one failing hides none.
pub struct Read {
    pub stashes: Result<Vec<Stash>, String>,
    pub remotes: Result<Vec<Remote>, String>,
    pub worktrees: Result<Vec<Worktree>, String>,
    pub submodules: Result<Vec<Submodule>, String>,
    pub lfs: Result<Vec<LfsFile>, String>,
    pub reflog: Result<Vec<recover::Moved>, String>,
}

fn read_all(git: &Git, root: &Path) -> Read {
    let said = |error: std::io::Error| error.to_string();
    Read {
        stashes: stash::list(git, root).map_err(said),
        remotes: remotes::list(git, root).map_err(said),
        worktrees: structure::worktrees(git, root).map_err(said),
        submodules: structure::submodules(git, root).map_err(said),
        lfs: structure::lfs(git, root).map_err(said),
        reflog: recover::reflog(git, root).map_err(said),
    }
}

/// What the Repository panel has picked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    Stash(String),
    Remote(String),
    Worktree(PathBuf),
    Submodule(String),
    /// One reflog event, known by itself, not its place.
    Reflog(recover::Moved),
}

fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

impl Tree {
    pub fn read_places(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(git) = self.git.as_ref().ok().cloned() else {
            return;
        };
        let Some(opened) = self.opened_mut(id) else {
            return;
        };
        opened.places.asks += 1;
        let (root, ask) = (opened.repo.root.clone(), opened.places.asks);
        let task = cx.background_spawn(async move { read_all(&git, &root) });
        cx.spawn(async move |this, cx| {
            let read = task.await;
            this.update(cx, |tree, cx| {
                let Some(places) = tree
                    .opened_mut(id)
                    .map(|opened| &mut opened.places)
                    .filter(|places| places.asks == ask)
                else {
                    return;
                };
                places.read = Some(read);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn pick_place(&mut self, place: Place, cx: &mut Context<Self>) {
        self.look = None;
        match &place {
            Place::Stash(oid) => self.pick_in(Whose::Stash, oid, cx),
            Place::Reflog(moved) => self.pick_in(Whose::Reflog, &moved.oid, cx),
            _ => {}
        }
        if let Some(opened) = self.active.and_then(|id| self.opened_mut(id)) {
            opened.picked_place = Some(place);
        }
        cx.notify();
    }

    /// The stash list as last read.
    pub fn stashes(&self) -> &[Stash] {
        match self.active().and_then(|opened| opened.places.read.as_ref()) {
            Some(Read {
                stashes: Ok(stashes),
                ..
            }) => stashes,
            _ => &[],
        }
    }

    pub fn ask_stash(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(repo) = self.active else { return };
        let label = text(cx, "tree.stash.message", &[]);
        let message = cx.new(|cx| TextInput::new(window, cx).label(label));
        self.ask = Some(Ask::Stash {
            repo,
            message,
            untracked: false,
        });
        cx.notify();
    }

    /// Apply and pop run; a drop asks each time.
    pub fn stash_act(&mut self, at: usize, action: StashAction, cx: &mut Context<Self>) {
        let Some(repo) = self.active else { return };
        let Some(oid) = self.stashes().get(at).map(|stash| stash.oid.clone()) else {
            return;
        };
        let verb = match action {
            StashAction::Apply => "apply",
            StashAction::Pop => "pop",
            StashAction::Drop => "drop",
        };
        let write = Write::Stash { verb, at, oid };
        if action != StashAction::Drop {
            return self.write(repo, write, cx);
        }
        let name = format!("stash@{{{at}}}");
        self.ask = Some(Ask::Confirm {
            repo,
            title: text(cx, "tree.stash.drop.title", &[("name", &name)]),
            message: text(cx, "tree.stash.drop.message", &[("name", &name)]),
            go: text(cx, "tree.stash.drop.go", &[]),
            write,
        });
        cx.notify();
    }

    /// A new remote, or `old` renamed or moved.
    pub fn ask_remote(&mut self, old: Option<Remote>, window: &mut Window, cx: &mut Context<Self>) {
        let (name, url) = match &old {
            Some(remote) => (
                remote.name.clone(),
                remote.urls.first().cloned().unwrap_or_default(),
            ),
            None => (String::new(), String::new()),
        };
        let name = self.field_with(&name, "tree.remote.name", window, cx);
        let url = self.field_with(&url, "tree.remote.url", window, cx);
        let old = old.map(|remote| {
            let url = remote.urls.first().cloned().unwrap_or_default();
            (remote.name, url)
        });
        let Some(repo) = self.active else { return };
        self.ask = Some(Ask::Remote {
            repo,
            old,
            name,
            url,
        });
        cx.notify();
    }

    pub fn remove_remote(&mut self, name: &str, cx: &mut Context<Self>) {
        let words = args(&["remote", "remove", "--", name]);
        self.confirm("tree.remote.remove", name, words, cx);
    }

    /// Asks for a new folder, then the branch it holds.
    pub fn add_worktree(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(opened) = self.active() else { return };
        let repo = opened.id;
        let Some(near) = opened.repo.root.parent().map(Path::to_path_buf) else {
            return;
        };
        let picked = cx.prompt_for_new_path(&near, None);
        cx.spawn_in(window, async move |this, cx| {
            let picked = picked
                .await
                .map_err(io::Error::other)
                .and_then(|picked| picked.map_err(io::Error::other));
            this.update_in(cx, |tree, window, cx| match picked {
                Ok(Some(path)) => tree.ask_worktree(repo, path, window, cx),
                Ok(None) => log::info!("tree: the worktree folder panel was cancelled"),
                Err(error) => tree.failed("tree.error.panel", &error, cx),
            })
            .ok();
        })
        .detach();
    }

    fn ask_worktree(
        &mut self,
        repo: u64,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.opened(repo).is_none() {
            return log::info!("tree: the worktree's repository closed");
        }
        let branch = self.field_with("", "tree.worktree.branch", window, cx);
        self.ask = Some(Ask::Worktree {
            repo,
            path,
            branch,
            new: true,
        });
        cx.notify();
    }

    pub fn remove_worktree(&mut self, path: &Path, cx: &mut Context<Self>) {
        let shown = path.display().to_string();
        let words = args(&["worktree", "remove", "--", &shown]);
        self.confirm("tree.worktree.remove", &shown, words, cx);
    }

    pub fn prune_worktrees(&mut self, cx: &mut Context<Self>) {
        self.run_git(args(&["worktree", "prune"]), cx);
    }

    pub fn update_submodule(&mut self, path: &str, cx: &mut Context<Self>) {
        let Some(id) = self.active else { return };
        let words = ["submodule", "update", "--init", "--progress", "--", path];
        self.write(id, Write::Remote(args(&words)), cx);
    }

    pub fn open_submodule(&mut self, path: &str, cx: &mut Context<Self>) {
        let Some(root) = self.active().map(|opened| opened.repo.root.join(path)) else {
            return;
        };
        self.open_path(root, cx);
    }

    /// The Repository panel's dialogs, as answered.
    pub fn answer_places(&mut self, cx: &mut Context<Self>) {
        let field = |field: &gpui::Entity<TextInput>, cx: &Context<Self>| {
            field.read(cx).text().trim().to_string()
        };
        let (repo, words): (u64, Vec<Vec<String>>) = match &self.ask {
            Some(Ask::Stash {
                repo,
                message,
                untracked,
            }) => {
                let mut words = args(&["stash", "push"]);
                words.extend(untracked.then(|| "--include-untracked".into()));
                let message = field(message, cx);
                if !message.is_empty() {
                    words.extend(["-m".into(), message]);
                }
                (*repo, vec![words])
            }
            Some(Ask::Remote {
                repo,
                old,
                name,
                url,
            }) => {
                let (name, url) = (field(name, cx), field(url, cx));
                if name.is_empty() || url.is_empty() {
                    let words = text(cx, "tree.remote.empty", &[]);
                    return self.toast(words, Severity::Warning, cx);
                }
                let words = match old {
                    None => vec![args(&["remote", "add", "--", &name, &url])],
                    Some((was, was_url)) => {
                        let mut words = Vec::new();
                        if *was != name {
                            words.push(args(&["remote", "rename", "--", was, &name]));
                        }
                        if *was_url != url {
                            words.push(args(&["remote", "set-url", "--", &name, &url]));
                        }
                        words
                    }
                };
                (*repo, words)
            }
            Some(Ask::Worktree {
                repo,
                path,
                branch,
                new,
            }) => {
                let branch = field(branch, cx);
                let path = path.display().to_string();
                let words = match (branch.is_empty(), new) {
                    (true, _) => args(&["worktree", "add", "--detach", "--", &path]),
                    (false, true) => args(&["worktree", "add", "-b", &branch, "--", &path]),
                    (false, false) => args(&["worktree", "add", "--", &path, &branch]),
                };
                (*repo, vec![words])
            }
            _ => return,
        };
        self.ask = None;
        for words in words {
            self.write(repo, Write::Git(words), cx);
        }
        cx.notify();
    }
}
