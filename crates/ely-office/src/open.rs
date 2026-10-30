//! Opening user files and copies over one worker (FILE-03).

use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use ely_files::{Open, Session};
use gpui::{AppContext as _, Context, Entity, TaskExt as _, Window};

use crate::engine::{Engine, Load};
use crate::files::{self, file_name};
use crate::{Host, Tab, Tabs, View, in_window, text};

/// How host `H`'s engine reads text files.
type ImportOf<H> = <<H as Host>::Engine as Engine>::Import;

/// What a load starts from.
pub enum Source<I> {
    /// A user file under its registry id, with its password.
    File(PathBuf, usize, String),
    /// A template copy; the result is untitled.
    Copy(PathBuf),
    /// A copy that needed a password, with the one given.
    Locked(PathBuf, String),
    /// Checked text bytes, where they go, how to read.
    Import(PathBuf, Arc<Vec<u8>>, I),
}

pub trait Opening: Host {
    /// An untitled copy of a local template.
    fn new_from_file(&mut self, template: &Path, cx: &mut Context<Self>) {
        let name = file_name(template);
        let copy = files::work_dir(&self.core().work, self.core().next).and_then(|dir| {
            let copy = dir.join(name.as_ref());
            std::fs::copy(template, &copy)?;
            Ok(copy)
        });
        match copy {
            Ok(copy) => self.load(Source::Copy(copy), cx),
            Err(error) => self.failed_open(&name, &error, cx),
        }
    }

    /// A new document from checked text bytes named `name`.
    fn import_text(
        &mut self,
        name: &str,
        bytes: Arc<Vec<u8>>,
        how: ImportOf<Self>,
        cx: &mut Context<Self>,
    ) {
        match files::work_dir(&self.core().work, self.core().next) {
            Ok(dir) => self.load(Source::Import(dir.join(name), bytes, how), cx),
            Err(error) => self.failed_open(name, &error, cx),
        }
    }

    /// An untitled document from `contents` named `name`.
    fn new_from_text(&mut self, name: &str, contents: &str, cx: &mut Context<Self>) {
        let copy = files::work_dir(&self.core().work, self.core().next).and_then(|dir| {
            let copy = dir.join(name);
            std::fs::write(&copy, contents)?;
            Ok(copy)
        });
        match copy {
            Ok(copy) => self.load(Source::Copy(copy), cx),
            Err(error) => self.failed_open(name, &error, cx),
        }
    }

    /// Opens a user file; an open one gets focus (FILE-03).
    fn open_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.open_with(path, String::new(), cx);
    }

    /// Opens a user file with its password.
    fn open_with(&mut self, path: PathBuf, password: String, cx: &mut Context<Self>) {
        match self.core_mut().sessions.open(&path, false) {
            Ok(Open::Focus(registry)) => {
                let core = self.core_mut();
                core.active = core
                    .tabs
                    .iter()
                    .find(|tab| tab.registry == Some(registry))
                    .map(|tab| tab.id);
                cx.notify();
            }
            Ok(Open::Fresh(registry) | Open::View(registry)) => {
                self.load(Source::File(path, registry, password), cx)
            }
            Err(error) => self.failed_open(&path.display().to_string(), &error, cx),
        }
    }

    fn failed_open(&self, name: &str, error: &io::Error, cx: &mut gpui::App) {
        log::error!("{}: {name} did not open: {error}", Self::KIND.app.slug());
        let error = error.to_string();
        let text = text(
            cx,
            "office.error.open",
            &[("name", name), ("error", &error)],
        );
        self.toast(text, ely_gpui_component::primitives::Severity::Danger, cx);
    }

    /// Loads off the main thread; the tab takes focus.
    fn load(&mut self, source: Source<ImportOf<Self>>, cx: &mut Context<Self>) {
        let id = self.core().next;
        self.core_mut().next += 1;
        self.load_into(id, source, cx);
    }

    /// Loads into tab id `id`, taken earlier from `next`.
    fn load_into(&mut self, id: u64, source: Source<ImportOf<Self>>, cx: &mut Context<Self>) {
        let (path, password, text) = match &source {
            Source::File(path, _, password) => (path.clone(), password.clone(), None),
            Source::Copy(path) => (path.clone(), String::new(), None),
            Source::Locked(path, password) => (path.clone(), password.clone(), None),
            Source::Import(path, bytes, how) => (
                path.clone(),
                String::new(),
                Some((Arc::clone(bytes), how.clone())),
            ),
        };
        let written = path.clone();
        self.core_mut().loading.push((id, file_name(&path)));
        cx.notify();
        crate::Stopping::watch_slow(self, id, cx);
        let worker = Arc::clone(&self.core().worker);
        cx.spawn(async move |this, cx| {
            let loaded = cx
                .background_spawn(async move {
                    if let Some((bytes, _)) = &text {
                        std::fs::write(&written, bytes.as_slice())?;
                    }
                    let mut worker = worker.lock().expect("the worker lock");
                    let from = match &text {
                        None => Load::File(&written, &password),
                        Some((_, how)) => Load::Import(&written, how),
                    };
                    Self::Engine::load(&mut worker, from)
                })
                .await;
            in_window(&this, cx, |host, window, cx| {
                host.loaded(id, source, loaded, window, cx)
            })
        })
        .detach_and_log_err(cx);
    }

    fn loaded(
        &mut self,
        id: u64,
        source: Source<ImportOf<Self>>,
        loaded: io::Result<<Self::Engine as Engine>::Loaded>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.core_mut()
            .loading
            .retain(|(loading, _)| *loading != id);
        if self.core().slow == Some(id) {
            self.core_mut().slow = None;
        }
        let imported = matches!(source, Source::Import(..));
        let (path, registry, tried) = match source {
            Source::File(path, registry, password) => (path, Some(registry), !password.is_empty()),
            Source::Copy(path) | Source::Import(path, ..) => (path, None, false),
            Source::Locked(path, _) => (path, None, true),
        };
        let opened = loaded.and_then(|loaded| {
            let risk = match files::macro_risk(&Self::KIND, &path)? {
                true => Some(crate::Risk::Macros),
                false => files::section_risk(&Self::KIND, &path)?.or_else(|| Self::Engine::risk(&loaded)),
            };
            let (work, session) = match registry {
                Some(_) => (
                    files::work_dir(&self.core().work, id)?,
                    Session::load(path.clone())?,
                ),
                None => {
                    let dir = path.parent().expect("a copy sits in its work dir");
                    (dir.to_path_buf(), Session::new_file())
                }
            };
            let (doc, view, mark) = Self::Engine::open(&self.core().worker, loaded, window, cx)?;
            Ok((doc, work, session, risk, view, mark))
        });
        let (doc, work, session, risk, view, mark) = match opened {
            Ok(opened) => opened,
            Err(error) => {
                if let Some(registry) = registry {
                    self.core_mut().sessions.close(registry);
                }
                cx.notify();
                // A locked copy asks too, and stays a copy.
                if error.kind() == io::ErrorKind::PermissionDenied && !imported {
                    return crate::Dialogs::ask_password(self, path, tried, registry.is_none(), window, cx);
                }
                // A stopped or crashed engine fails the load first.
                crate::Stopping::check_engine(self, cx);
                return self.failed_open(&path.display().to_string(), &error, cx);
            }
        };
        let name = match registry {
            Some(_) => file_name(&path),
            None if imported => file_name(&path),
            None => {
                self.core_mut().untitled += 1;
                let n = self.core().untitled.to_string();
                text(cx, "office.untitled", &[("n", &n)])
            }
        };
        self.watch(id, &view, cx);
        window.focus(&Self::Engine::focus(view.read(cx)), cx);
        // Tabs keep the order their loads began.
        let at = self.core().tabs.partition_point(|tab| tab.id < id);
        self.core_mut().tabs.insert(at, Tab {
            id,
            view,
            doc,
            session,
            registry,
            name,
            modified: false,
            risk,
            mark,
            work,
            exporting: false,
            reopen: false,
            extra: Default::default(),
        });
        self.core_mut().active = Some(id);
        // Points from now on; a crash copy comes back unsaved.
        let tab = self.tab(id).expect("just opened");
        let named = tab.name.clone();
        log::info!(
            "{}: opened {named} as tab {id}, risk {:?}, {} tabs open",
            Self::KIND.app.slug(),
            tab.risk,
            self.core().tabs.len()
        );
        crate::Recovering::keep_tab(self, id, &named, tried, cx);
        crate::Recovering::reopened(self, id, &path, cx);
        cx.notify();
    }

    /// Engine errors become toasts; kernel edits mark the session.
    fn watch(&mut self, id: u64, view: &Entity<View<Self>>, cx: &mut Context<Self>) {
        let seen = std::cell::Cell::new(Self::Engine::edited(view.read(cx)));
        let reported = move |host: &mut Self,
                             view: &Entity<View<Self>>,
                             modified: bool,
                             cx: &mut Context<Self>| {
            let edited = Self::Engine::edited(view.read(cx));
            let moved = seen.replace(edited) != edited;
            if let Some(tab) = host.tab_mut(id) {
                match (modified, tab.modified) {
                    (true, false) => tab.session.edit(),
                    // An edit landing mid-save outdates its ticket.
                    (true, true) if moved => tab.session.edit(),
                    (false, true) => tab.session.reverted(),
                    _ => {}
                }
                tab.modified = modified;
            }
            host.observed(id, view, cx);
            if Self::Engine::settled(view.read(cx)) {
                let (ready, rest): (Vec<_>, Vec<_>) = host
                    .core_mut()
                    .waiting
                    .drain(..)
                    .partition(|(tab, _)| *tab == id);
                host.core_mut().waiting = rest;
                for (_, then) in ready {
                    cx.spawn(async move |this, cx| in_window(&this, cx, then))
                        .detach_and_log_err(cx);
                }
            }
            cx.notify();
        };
        Self::Engine::watch(view, cx, reported, move |host, error, cx| {
            // A released tab's view may finish its queue late.
            if host.tab(id).is_none() {
                log::info!("office: tab {id} failed after it closed");
                return;
            }
            host.engine_failed(error, cx);
            crate::Stopping::check_engine(host, cx);
        });
    }
}

impl<H: Host> Opening for H {}
