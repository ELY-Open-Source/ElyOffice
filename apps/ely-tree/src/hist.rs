//! History in the window: pages, search, a picked commit (TREE-06).

use std::rc::Rc;

use ely_gpui_component::git::{Commit, GraphRow, lanes};
use gpui::{AppContext as _, Context, SharedString};

use crate::{
    commits::{self, Logged, PAGE, Query},
    detail::{self, Detail},
    shown::{self, View},
    stash,
    tree::Tree,
};

/// One repository's history as read so far.
#[derive(Default)]
pub struct Hist {
    pub logged: Vec<Logged>,
    pub rows: Rc<Vec<Commit>>,
    pub graph: Rc<Vec<GraphRow>>,
    pub loading: bool,
    /// The last page came back short.
    pub done: bool,
    /// Read at least once.
    pub read: bool,
    pub query: Option<Query>,
    pub error: Option<String>,
    /// `show-ref --head` when last read.
    refs: Vec<u8>,
    /// Refs may have moved during a read; check after it.
    stale: bool,
    serial: u64,
    pub pick: Picked,
}

/// A commit in the middle: its files, one file's diff.
#[derive(Default)]
pub struct Picked {
    pub commit: Option<String>,
    pub detail: Option<Result<Detail, String>>,
    pub file: Option<(usize, Option<Result<View, String>>)>,
}

/// Whose picked commit: history's or a stash's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Whose {
    History,
    Stash,
    Reflog,
}

/// A time in this machine's zone, to the minute.
pub fn when(at: i64) -> SharedString {
    match jiff::Timestamp::from_second(at) {
        Ok(at) => at
            .to_zoned(jiff::tz::TimeZone::system())
            .strftime("%Y-%m-%d %H:%M")
            .to_string()
            .into(),
        Err(error) => {
            log::warn!("tree: time {at} is out of range: {error}");
            at.to_string().into()
        }
    }
}

/// The rows a commit list draws.
pub fn rows(logged: &[Logged]) -> Vec<Commit> {
    logged
        .iter()
        .map(|commit| Commit {
            id: commit.id.clone().into(),
            parents: commit.parents.iter().map(|id| id.clone().into()).collect(),
            subject: commit.subject.clone().into(),
            author: commit.author.clone().into(),
            when: when(commit.at),
            refs: commit.refs.iter().map(|name| name.clone().into()).collect(),
        })
        .collect()
}

fn graph(logged: &[Logged]) -> Vec<GraphRow> {
    let parents: Vec<Vec<&str>> = logged
        .iter()
        .map(|commit| commit.parents.iter().map(String::as_str).collect())
        .collect();
    lanes(
        logged
            .iter()
            .zip(&parents)
            .map(|(commit, parents)| (commit.id.as_str(), parents.as_slice())),
    )
}

impl Tree {
    pub fn set_side(&mut self, side: crate::tree::Side, cx: &mut Context<Self>) {
        self.look = None;
        self.side = side;
        self.ensure_side(cx);
        cx.notify();
    }

    /// A panel's lists are read once it first shows.
    pub fn ensure_side(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.active else { return };
        let Some(opened) = self.opened(id) else {
            return;
        };
        match self.side {
            crate::tree::Side::History if !opened.hist.read && !opened.hist.loading => {
                self.load_log(id, true, cx)
            }
            crate::tree::Side::Branches if opened.refs.is_none() => self.read_refs(id, cx),
            crate::tree::Side::Places if opened.places.read.is_none() => self.read_places(id, cx),
            _ => {}
        }
    }

    /// The next page, once the list nears its end.
    pub fn more_log(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.active {
            self.load_log(id, false, cx);
        }
    }

    fn hist_mut(&mut self, id: u64) -> Option<&mut Hist> {
        self.opened_mut(id).map(|opened| &mut opened.hist)
    }

    /// Reads history from the top, or the next page.
    pub fn load_log(&mut self, id: u64, fresh: bool, cx: &mut Context<Self>) {
        let Some(git) = self.git.as_ref().ok().cloned() else {
            return;
        };
        let Some(root) = self.opened(id).map(|opened| opened.repo.root.clone()) else {
            return;
        };
        let Some(hist) = self.hist_mut(id) else {
            return;
        };
        if !fresh && (hist.loading || hist.done) {
            return;
        }
        hist.serial += 1;
        hist.loading = true;
        let (serial, skip, query) = (
            hist.serial,
            if fresh { 0 } else { hist.logged.len() },
            hist.query.clone(),
        );
        let task = cx.background_spawn(async move {
            let refs = commits::refs_now(&git, &root)?;
            Ok::<_, std::io::Error>((refs, commits::read(&git, &root, skip, query.as_ref())?))
        });
        cx.spawn(async move |this, cx| {
            let read = task.await;
            this.update(cx, |tree, cx| tree.log_read(id, serial, fresh, read, cx))
                .ok();
        })
        .detach();
    }

    fn log_read(
        &mut self,
        id: u64,
        serial: u64,
        fresh: bool,
        read: std::io::Result<(Vec<u8>, Vec<Logged>)>,
        cx: &mut Context<Self>,
    ) {
        let Some(hist) = self.hist_mut(id).filter(|hist| hist.serial == serial) else {
            return;
        };
        hist.loading = false;
        hist.read = true;
        // A page read under moved refs is dropped.
        if !fresh && matches!(&read, Ok((refs, _)) if *refs != hist.refs) {
            log::info!("tree: refs moved between history pages; rereading from the top");
            hist.stale = false;
            self.load_log(id, true, cx);
            return;
        }
        match read {
            Ok((refs, page)) => {
                if fresh {
                    hist.logged.clear();
                    hist.refs = refs;
                }
                hist.done = page.len() < PAGE;
                hist.logged.extend(page);
                hist.error = None;
                hist.rows = Rc::new(rows(&hist.logged));
                hist.graph = Rc::new(if hist.query.is_none() {
                    graph(&hist.logged)
                } else {
                    Vec::new()
                });
                log::info!("tree: history shows {} commits, searched {}", hist.logged.len(), hist.query.is_some());
            }
            Err(error) => {
                log::warn!("tree: reading history failed: {error}");
                hist.error = Some(error.to_string());
            }
        }
        if std::mem::take(&mut hist.stale) {
            self.refresh_log(id, cx);
        }
        cx.notify();
    }

    /// After a status read: reread history if refs moved.
    pub fn refresh_log(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(git) = self.git.as_ref().ok().cloned() else {
            return;
        };
        let Some(hist) = self.hist_mut(id) else {
            return;
        };
        if hist.loading {
            hist.stale = true;
            return;
        }
        if !hist.read {
            return;
        }
        let was = hist.refs.clone();
        let Some(root) = self.opened(id).map(|opened| opened.repo.root.clone()) else {
            return;
        };
        let task = cx.background_spawn(async move { commits::refs_now(&git, &root) });
        cx.spawn(async move |this, cx| {
            let now = task.await;
            this.update(cx, |tree, cx| match now {
                Ok(now) if now != was => tree.load_log(id, true, cx),
                // A shown error goes once a read succeeds again.
                Ok(_)
                    if tree
                        .opened(id)
                        .is_some_and(|opened| opened.hist.error.is_some()) =>
                {
                    tree.load_log(id, true, cx)
                }
                Ok(_) => {}
                // Shown in the panel, in Git's words.
                Err(error) => {
                    log::warn!("tree: reading refs failed: {error}");
                    if let Some(hist) = tree.hist_mut(id) {
                        hist.error = Some(error.to_string());
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();
    }

    /// Runs the search field's query; empty shows all history.
    pub fn search(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.active else { return };
        let text = self.search.read(cx).text().trim().to_string();
        let query = (!text.is_empty()).then_some(Query {
            find: self.find,
            text,
        });
        let Some(hist) = self.hist_mut(id) else {
            return;
        };
        hist.query = query;
        self.load_log(id, true, cx);
    }

    fn picked_mut(&mut self, id: u64, whose: Whose) -> Option<&mut Picked> {
        let opened = self.opened_mut(id)?;
        Some(match whose {
            Whose::History => &mut opened.hist.pick,
            Whose::Stash | Whose::Reflog => &mut opened.places.pick,
        })
    }

    /// Shows a commit's details in the middle.
    pub fn pick_commit(&mut self, commit: &str, cx: &mut Context<Self>) {
        self.pick_in(Whose::History, commit, cx);
    }

    /// Reads a commit, or a stash with its untracked files.
    pub fn pick_in(&mut self, whose: Whose, commit: &str, cx: &mut Context<Self>) {
        self.look = None;
        let Some(git) = self.git.as_ref().ok().cloned() else {
            return;
        };
        let Some(id) = self.active else { return };
        let Some(root) = self.opened(id).map(|opened| opened.repo.root.clone()) else {
            return;
        };
        let Some(picked) = self.picked_mut(id, whose) else {
            return;
        };
        *picked = Picked {
            commit: Some(commit.to_string()),
            ..Picked::default()
        };
        let commit = commit.to_string();
        let task = cx.background_spawn({
            let commit = commit.clone();
            async move {
                match whose {
                    Whose::History | Whose::Reflog => detail::read(&git, &root, &commit),
                    Whose::Stash => stash::detail(&git, &root, &commit),
                }
            }
        });
        cx.spawn(async move |this, cx| {
            let read = task.await.map_err(|error| error.to_string());
            this.update(cx, |tree, cx| {
                let Some(picked) = tree
                    .picked_mut(id, whose)
                    .filter(|picked| picked.commit.as_deref() == Some(commit.as_str()))
                else {
                    return;
                };
                picked.detail = Some(read);
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Shows one file of the picked commit.
    pub fn pick_touched(&mut self, whose: Whose, at: usize, cx: &mut Context<Self>) {
        let Some(git) = self.git.as_ref().ok().cloned() else {
            return;
        };
        let Some(id) = self.active else { return };
        let Some(root) = self.opened(id).map(|opened| opened.repo.root.clone()) else {
            return;
        };
        let Some(picked) = self.picked_mut(id, whose) else {
            return;
        };
        let Some(Ok(detail)) = &picked.detail else {
            return;
        };
        let Some(touched) = detail.files.get(at).cloned() else {
            return;
        };
        // A stash's untracked files sit in their own commit.
        let (holder, parents) = match &touched.commit {
            Some(holder) => (holder.clone(), Vec::new()),
            None => (detail.id.clone(), detail.parents.clone()),
        };
        let commit = detail.id.clone();
        picked.file = Some((at, None));
        let task =
            cx.background_spawn(
                async move { detail::file(&git, &root, &holder, &parents, &touched) },
            );
        cx.spawn(async move |this, cx| {
            let found = task.await;
            this.update(cx, |tree, cx| {
                // Only for the file and commit still shown.
                let Some(picked) = tree
                    .picked_mut(id, whose)
                    .filter(|picked| picked.commit.as_deref() == Some(commit.as_str()))
                else {
                    return;
                };
                let Some((now, view)) = &mut picked.file else {
                    return;
                };
                if *now == at {
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
}
