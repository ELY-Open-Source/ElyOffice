//! The window: one tab per repository (TREE-01).

use std::{
    collections::VecDeque,
    io,
    path::{Path, PathBuf},
};

use ely_gpui_component::{
    forms::{InputEvent, TextInput},
    primitives::Severity,
};
use ely_platform::AppId;
use ely_ui::Root;
use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, SharedString, Subscription, WeakEntity,
    Window,
};
use notify::RecommendedWatcher;

use crate::{
    changes::Changes,
    commit::Me,
    commits::Find,
    git::Git,
    hist::Hist,
    look::Look,
    repo::{self, Repo},
    status::{self, Status},
    work::Write,
};

/// An open repository and what was last read of it.
pub struct Opened {
    pub id: u64,
    pub repo: Repo,
    pub status: Option<Result<Status, String>>,
    /// A read runs; `again` asks one more.
    pub reading: bool,
    pub again: bool,
    /// Writes wait here and run one at a time (§13.3).
    pub writes: VecDeque<Write>,
    pub writing: bool,
    pub hist: Hist,
    pub refs: Option<Result<Vec<crate::branches::Ref>, String>>,
    pub refs_asks: u64,
    /// A merge, rebase, pick or revert under way.
    pub doing: Option<Result<Option<crate::ops::Doing>, String>>,
    pub doing_asks: u64,
    /// This repository's commit box while another is shown.
    pub held: crate::committing::Held,
    /// Who commits here, and whether Git signs.
    pub me: Option<Result<Me, String>>,
    /// Identity reads started; only the latest lands.
    pub me_asks: u64,
    pub places: crate::places::Places,
    /// The branch or tag picked in the Branches panel.
    pub picked_ref: Option<(crate::branches::Kind, String)>,
    pub picked_place: Option<crate::places::Place>,
    pub net: Option<crate::network::Net>,
    /// Kept alive; dropping it stops the watch.
    _watcher: Option<RecommendedWatcher>,
}

/// What the left panel lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Changes,
    History,
    Branches,
    Places,
}

pub struct Tree {
    pub git: Result<Git, String>,
    pub repos: Vec<Opened>,
    pub active: Option<u64>,
    next: u64,
    pub recent: Vec<PathBuf>,
    /// Why the recent list could not be read; shown once.
    unread: Option<String>,
    recent_file: PathBuf,
    pub tools: crate::tools::Tools,
    pub cloning: Option<crate::cloning::Cloning>,
    pub ask: Option<crate::asks::Ask>,
    pub focus: FocusHandle,
    pub root: WeakEntity<Root>,
    pub changes: Entity<Changes>,
    pub shown: Option<crate::pick::Shown>,
    pub space: crate::diff::Space,
    pub layout: ely_gpui_component::git::DiffLayout,
    /// Diff reads started, numbering each.
    pub reads: u64,
    pub side: Side,
    pub look: Option<Look>,
    pub subject: Entity<TextInput>,
    pub body: Entity<TextInput>,
    pub amend: bool,
    /// The message typed before amend filled in the last one.
    pub draft: Option<(String, String)>,
    pub search: Entity<TextInput>,
    pub find: Find,
    /// Amend reads asked; switching or unticking voids older ones.
    pub amend_ask: u64,
    /// Amend is ticked; the last message is on its way.
    pub amend_loading: bool,
    /// File history and blame opened, numbering each.
    pub looks: u64,
    /// The conflicted file being merged by hand.
    pub merging: Option<crate::conflict::Merging>,
    pub merge_editor: Entity<ely_gpui_component::editor::CodeEditor>,
    _quit: Subscription,
    _searching: Subscription,
    _returning: Subscription,
}

pub fn text(cx: &App, key: &str, args: &[(&str, &str)]) -> SharedString {
    ely_ui::i18n::text(cx, key, args)
}

/// The window, with the repositories it was started with.
pub fn open(app: AppId, window: &mut Window, cx: &mut App) -> Entity<Root> {
    crate::menu::bind(cx);
    let recent_file = match ely_platform::data_dir(app) {
        Ok(dir) => dir.join("recent.json"),
        Err(error) => ely_platform::startup::fail(app, &format!("{error:#}")),
    };
    let tree = cx.new(|cx| Tree::new(recent_file, window, cx));
    let root = ely_ui::tool_root(app, tree.clone().into(), crate::menu::menus(cx), window, cx);
    tree.update(cx, |tree, cx| {
        tree.root = root.downgrade();
        window.focus(&tree.focus, cx);
        if let Some(error) = tree.unread.take() {
            tree.failed("tree.error.recent", &error, cx);
        }
        if let Some(error) = tree.tools.broken.clone() {
            tree.failed("tree.error.tools_file", &error, cx);
        }
    });
    for path in ely_ui::launch_files(cx) {
        tree.update(cx, |tree, cx| tree.open_path(path, cx));
    }
    let opening = tree.clone();
    ely_ui::on_open(cx, move |path, cx| {
        opening.update(cx, |tree, cx| tree.open_path(path, cx))
    });
    // A clone is not left running behind a closed window.
    let closing = tree.clone();
    ely_gpui_component::shell::guard_close(window, cx, move |_, cx| {
        closing.update(cx, |tree, cx| tree.stop_all(cx));
        true
    });
    root
}

/// `file` as Git names it; a symlink stays itself.
pub(crate) fn inside(root: &Path, file: &Path) -> io::Result<String> {
    let name = file.file_name().ok_or_else(|| io::Error::other("a path without a name"))?;
    let dir = dunce::canonicalize(file.parent().unwrap_or(Path::new(".")))?;
    let rel = dir.join(name);
    let rel = rel.strip_prefix(root).map_err(io::Error::other)?;
    rel.to_str().map(str::to_owned).ok_or_else(|| io::Error::other("a path that is not UTF-8"))
}

impl Tree {
    pub(crate) fn new(recent_file: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let git = Git::find().map_err(|error| error.to_string());
        if let Err(error) = &git {
            log::error!("tree: {error}");
        }
        let (recent, unread) = match ely_files::recent::recent(&recent_file) {
            Ok(list) => (list, None),
            Err(error) => (Vec::new(), Some(error.to_string())),
        };
        let this = cx.entity();
        let changes = cx.new(|cx| Changes::new(&this, cx));
        let subject = text(cx, "tree.commit.subject", &[]);
        let subject = cx.new(|cx| TextInput::new(window, cx).placeholder(subject));
        let body = text(cx, "tree.commit.body", &[]);
        let body = cx.new(|cx| {
            TextInput::new(window, cx)
                .placeholder(body)
                .multi_line(3, 8)
        });
        let merge_editor =
            cx.new(|cx| ely_gpui_component::editor::CodeEditor::new(String::new(), window, cx));
        let search = text(cx, "tree.search", &[]);
        let search = cx.new(|cx| TextInput::new(window, cx).placeholder(search));
        let searching = cx.subscribe(&search, |tree: &mut Tree, _, event: &InputEvent, cx| {
            if *event == InputEvent::Submit {
                tree.search(cx);
            }
        });
        Tree {
            git,
            repos: Vec::new(),
            active: None,
            next: 1,
            recent,
            unread,
            tools: crate::tools::Tools::load(recent_file.with_file_name("tools.json")),
            recent_file,
            cloning: None,
            ask: None,
            focus: cx.focus_handle(),
            root: WeakEntity::new_invalid(),
            changes,
            shown: None,
            space: crate::diff::Space::Exact,
            layout: ely_gpui_component::git::DiffLayout::Unified,
            reads: 0,
            side: Side::Changes,
            look: None,
            subject,
            body,
            amend: false,
            draft: None,
            search,
            find: Find::Message,
            amend_ask: 0,
            amend_loading: false,
            looks: 0,
            merging: None,
            merge_editor,
            _searching: searching,
            // Global settings change outside any repository's watch.
            _returning: cx.observe_window_activation(window, |tree, window, cx| {
                if let Some(id) = tree.active.filter(|_| window.is_window_active()) {
                    tree.read_me(id, cx);
                }
            }),
            _quit: cx.on_app_quit(|tree, cx| {
                tree.stop_all(cx);
                async {}
            }),
        }
    }

    pub fn toast(&self, text: SharedString, severity: Severity, cx: &mut App) {
        log::warn!("tree: {text}");
        if let Some(root) = self.root.upgrade() {
            root.update(cx, |root, cx| root.toast(text, severity, cx));
        }
    }

    pub fn failed(&self, key: &str, error: &str, cx: &mut App) {
        let words = text(cx, key, &[("error", error)]);
        self.toast(words, Severity::Danger, cx);
    }

    /// The Git to run, or why there is none.
    pub fn git(&self, cx: &mut App) -> Option<Git> {
        match &self.git {
            Ok(git) => Some(git.clone()),
            Err(error) => {
                self.failed("tree.error.no_git", error, cx);
                None
            }
        }
    }

    pub fn opened(&self, id: u64) -> Option<&Opened> {
        self.repos.iter().find(|opened| opened.id == id)
    }

    pub fn opened_mut(&mut self, id: u64) -> Option<&mut Opened> {
        self.repos.iter_mut().find(|opened| opened.id == id)
    }

    pub fn active(&self) -> Option<&Opened> {
        self.active.and_then(|id| self.opened(id))
    }

    /// Opens `path`'s repository; a file also shows its history.
    pub fn open_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(git) = self.git(cx) else { return };
        let task = cx.background_spawn(async move {
            let file = path.is_file().then(|| path.clone());
            let dir = file.as_ref().and_then(|file| file.parent()).unwrap_or(&path);
            let opened = repo::open(&git, dir).map_err(|error| (path.clone(), error));
            // The file as Git names it, from the repository's top.
            let inside = match (&opened, file) {
                (Ok(repo), Some(file)) => Some(inside(&repo.root, &file).map_err(|error| (file, error))),
                _ => None,
            };
            (opened, inside)
        });
        cx.spawn(async move |this, cx| {
            let (opened, inside) = task.await;
            this.update(cx, |tree, cx| {
                tree.add(opened, cx);
                match inside {
                    Some(Ok(rel)) => tree.file_log(&rel, cx),
                    Some(Err((file, error))) => {
                        let shown = file.display().to_string();
                        let words = text(cx, "tree.error.open", &[("path", &shown), ("error", &error.to_string())]);
                        tree.toast(words, Severity::Danger, cx);
                    }
                    None => {}
                }
            })
            .ok();
        })
        .detach();
    }

    /// Starts a repository in `dir`, then opens it.
    pub fn init_at(&mut self, dir: PathBuf, cx: &mut Context<Self>) {
        let Some(git) = self.git(cx) else { return };
        let task = cx
            .background_spawn(async move { repo::init(&git, &dir).map_err(|error| (dir, error)) });
        cx.spawn(async move |this, cx| {
            let made = task.await;
            this.update(cx, |tree, cx| tree.add(made, cx)).ok();
        })
        .detach();
    }

    fn add(&mut self, opened: Result<Repo, (PathBuf, io::Error)>, cx: &mut Context<Self>) {
        let repo = match opened {
            Ok(repo) => repo,
            Err((path, error)) => return self.not_opened(&path, &error, cx),
        };
        self.recent = match ely_files::recent::remember(&self.recent_file, &repo.root) {
            Ok(list) => list,
            Err(error) => {
                self.failed("tree.error.recent", &error.to_string(), cx);
                self.recent.clone()
            }
        };
        if let Some(known) = self
            .repos
            .iter()
            .find(|opened| opened.repo.root == repo.root)
        {
            let known = known.id;
            return self.activate(Some(known), cx);
        }
        let id = self.next;
        self.next += 1;
        let watcher = match crate::watch::start(id, &repo, cx) {
            Ok(watcher) => Some(watcher),
            Err(error) => {
                self.failed("tree.error.watch", &error.to_string(), cx);
                None
            }
        };
        self.repos.push(Opened {
            id,
            repo,
            status: None,
            reading: false,
            again: false,
            writes: VecDeque::new(),
            writing: false,
            hist: Hist::default(),
            refs: None,
            refs_asks: 0,
            doing: None,
            doing_asks: 0,
            held: Default::default(),
            me: None,
            me_asks: 0,
            places: Default::default(),
            picked_ref: None,
            picked_place: None,
            net: None,
            _watcher: watcher,
        });
        log::info!("tree: {} repositories open", self.repos.len());
        self.activate(Some(id), cx);
        self.read(id, cx);
        self.read_me(id, cx);
    }

    /// A recent entry that no longer opens leaves the list.
    fn not_opened(&mut self, path: &Path, error: &io::Error, cx: &mut Context<Self>) {
        let shown = path.display().to_string();
        if self.recent.iter().any(|known| known == path) {
            self.recent.retain(|known| known != path);
            if let Err(error) = ely_files::recent::forget(&self.recent_file, path) {
                self.failed("tree.error.recent", &error.to_string(), cx);
            }
            let words = text(
                cx,
                "tree.error.recent_gone",
                &[("path", &shown), ("error", &error.to_string())],
            );
            self.toast(words, Severity::Danger, cx);
        } else {
            let words = text(
                cx,
                "tree.error.open",
                &[("path", &shown), ("error", &error.to_string())],
            );
            self.toast(words, Severity::Danger, cx);
        }
        cx.notify();
    }

    /// Reads the work tree; overlapping asks become one.
    pub fn read(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(git) = self.git.as_ref().ok().cloned() else {
            return;
        };
        let Some(opened) = self.opened_mut(id) else {
            return;
        };
        if opened.reading {
            opened.again = true;
            return;
        }
        opened.reading = true;
        let root = opened.repo.root.clone();
        let task = cx.background_spawn(async move { status::read(&git, &root) });
        cx.spawn(async move |this, cx| {
            let read = task.await.map_err(|error| error.to_string());
            this.update(cx, |tree, cx| tree.read_done(id, read, cx))
                .ok();
        })
        .detach();
    }

    fn read_done(&mut self, id: u64, read: Result<Status, String>, cx: &mut Context<Self>) {
        let Some(opened) = self.opened_mut(id) else {
            return;
        };
        if let Err(error) = &read {
            log::warn!(
                "tree: reading {} failed: {error}",
                opened.repo.root.display()
            );
        }
        opened.status = Some(read);
        opened.reading = false;
        if std::mem::take(&mut opened.again) {
            self.read(id, cx);
        }
        if self.shown.as_ref().is_some_and(|shown| shown.repo == id) {
            self.read_shown(cx);
        }
        self.refresh_log(id, cx);
        // Settings may have moved with the files.
        self.read_me(id, cx);
        self.read_doing(id, cx);
        if self.opened(id).is_some_and(|opened| opened.refs.is_some()) {
            self.read_refs(id, cx);
        }
        if self
            .opened(id)
            .is_some_and(|opened| opened.places.read.is_some())
        {
            self.read_places(id, cx);
        }
        cx.notify();
    }

    pub fn close_active(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.active else { return };
        self.close(id, cx);
    }

    /// Closes a tab; writes already queued finish first.
    pub fn close(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(at) = self.repos.iter().position(|opened| opened.id == id) else {
            return;
        };
        if self.repos[at].writing || !self.repos[at].writes.is_empty() {
            let words = text(cx, "tree.busy", &[]);
            return self.toast(words, Severity::Warning, cx);
        }
        let closed = self.repos.remove(at);
        if self.shown.as_ref().is_some_and(|shown| shown.repo == id) {
            self.shown = None;
        }
        if self.look.as_ref().is_some_and(|look| look.repo() == id) {
            self.look = None;
        }
        log::info!("tree: closed {}, {} repositories open", closed.repo.root.display(), self.repos.len());
        if self.active == Some(id) {
            // The closed box goes with its repository.
            self.active = None;
            let next = self
                .repos
                .get(at.min(self.repos.len().saturating_sub(1)))
                .map(|opened| opened.id);
            self.activate(next, cx);
        }
        cx.notify();
    }
}
