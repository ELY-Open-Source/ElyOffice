//! LibreOfficeKit as the shell's engine (§7).

use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use ely_lok::{Doc, LokError, LokView, Reported, Worker, sheet::csv::Import};
use ely_platform::AppId;
use gpui::{App, Context, Entity, FocusHandle, Window};

use crate::engine::{Engine, Load};
use crate::{Host, Tabs, text};

/// The LOK engine.
pub struct Lok;

/// A load: the document, its layout, its history.
pub struct Loaded {
    doc: Doc,
    laid: ely_lok::Layout,
    mark: String,
}

impl Engine for Lok {
    type Worker = Worker;
    type Doc = Doc;
    type View = LokView;
    type Loaded = Loaded;
    type Import = Import;

    fn load(worker: &mut Worker, from: Load<Import>) -> io::Result<Loaded> {
        let doc = match from {
            Load::File(path, password) => worker.unlock(&ely_lok::file_url(path), password)?,
            Load::Import(path, how) => worker.import(&ely_lok::file_url(path), how)?,
        };
        // Layout and history too: the window never waits.
        let laid = ely_lok::layout(worker, &doc)?;
        let mark = worker.values(&doc, ".uno:Undo")?;
        Ok(Loaded { doc, laid, mark })
    }

    fn open(
        worker: &Arc<Mutex<Worker>>,
        loaded: Loaded,
        _: &mut Window,
        cx: &mut App,
    ) -> io::Result<(Doc, Entity<LokView>, String)> {
        let Loaded { doc, laid, mark } = loaded;
        let view = LokView::open(Arc::clone(worker), doc.clone(), laid, cx);
        let (zoom, deck) = (ely_ui::settings::current(cx).doc_zoom, doc.is_deck());
        // Decided once: a later default leaves it.
        view.update(cx, |view, cx| {
            if deck {
                view.turn_parts();
            }
            match zoom {
                ely_platform::settings::DocZoom::Percent(percent) => view.set_zoom(percent as f32 / 100.0, cx),
                ely_platform::settings::DocZoom::Auto if deck => view.fit_page(cx),
                ely_platform::settings::DocZoom::Auto => {}
            }
        });
        log::info!("office: opens at {zoom:?}, deck {deck}");
        Ok((doc, view, mark))
    }

    fn history(worker: &mut Worker, doc: &Doc) -> io::Result<String> {
        worker.values(doc, ".uno:Undo")
    }

    fn write(worker: &mut Worker, doc: &Doc, path: &Path, format: &str) -> io::Result<()> {
        doc.save_owned(worker, &ely_lok::file_url(path), format)
    }

    fn snapshot(worker: &mut Worker, doc: &Doc, path: &Path, format: &str) -> io::Result<()> {
        doc.save_as(worker, &ely_lok::file_url(path), format)
    }

    fn close(worker: &mut Worker, doc: Doc) -> io::Result<()> {
        worker.close_doc(doc)
    }

    fn retire(view: &Entity<LokView>, cx: &mut App) -> ely_gpui_component::primitives::Retired {
        view.update(cx, |view, _| view.retire_all())
    }

    fn ended(worker: &mut Worker) -> Option<String> {
        worker.ended()
    }

    fn restart(app: AppId, worker: &mut Worker) -> io::Result<()> {
        let author = worker.author().map(str::to_string);
        let author = author.ok_or_else(|| io::Error::other("the ended worker had no author"))?;
        // The ended worker stays until the new one is ready.
        let mut fresh = spawn(app)?;
        fresh.ready()?;
        fresh.set_author(&author)?;
        *worker = fresh;
        Ok(())
    }

    fn settled(view: &LokView) -> bool {
        view.settled()
    }

    fn edited(view: &LokView) -> u64 {
        view.edited()
    }

    fn edit_pending(view: &LokView) -> bool {
        view.edit_pending()
    }

    fn focus(view: &LokView) -> FocusHandle {
        view.focus_handle().clone()
    }

    /// The kernel reports its own modified status.
    fn saved(_: &Entity<LokView>, _: &str, _: &mut App) {}

    fn watch<H: Host<Engine = Self>>(
        view: &Entity<LokView>,
        cx: &mut Context<H>,
        reported: impl Fn(&mut H, &Entity<LokView>, bool, &mut Context<H>) + 'static,
        failed: impl Fn(&mut H, &str, &mut Context<H>) + 'static,
    ) {
        cx.subscribe(view, move |host, _, error: &LokError, cx| match error.0.starts_with(ely_lok::STALE) {
            true => failed(host, &text(cx, "office.error.changed", &[]), cx),
            false => failed(host, &error.0, cx),
        })
        .detach();
        // The status shows the zoom.
        cx.subscribe(view, |_, _, _: &ely_lok::Zoomed, cx| cx.notify()).detach();
        cx.subscribe(view, move |host, view, _: &Reported, cx| {
            let modified = view.read(cx).state(".uno:ModifiedStatus") == Some("true");
            reported(host, &view, modified, cx)
        })
        .detach();
    }
}

/// The worker and the scratch root of `app`.
pub fn start(app: AppId, cx: &mut App) -> io::Result<(Arc<Mutex<Worker>>, PathBuf)> {
    let data = ely_platform::data_dir(app).map_err(io::Error::other)?;
    let work = crate::fresh_work(&data)?;
    let worker = Arc::new(Mutex::new(spawn(app)?));
    // The window opens now; the kernel finishes starting behind it.
    let starting = Arc::clone(&worker);
    // Settings as read at launch; defaults if unreadable.
    let wanted = Arc::new(Mutex::new(ely_ui::settings::current(cx).author.clone()));
    let first = Arc::clone(&wanted);
    let started = cx.background_executor().spawn(async move {
        let mut worker = starting.lock().expect("the worker lock");
        worker.ready()?;
        sign(&mut worker, &first)
    });
    cx.spawn(async move |_| {
        if let Err(error) = started.await {
            ely_platform::startup::fail(app, &format!("the document engine did not start: {error}"));
        }
    })
    .detach();
    follow_author(&worker, wanted, cx);
    Ok((worker, work))
}

/// Sends the newest wanted name, read under the worker lock.
fn sign(worker: &mut Worker, wanted: &Mutex<String>) -> io::Result<()> {
    let author = wanted.lock().expect("the author lock").clone();
    worker.set_author(&ely_platform::user::named(&author)?)
}

/// A worker process on the app's profile, not yet checked.
fn spawn(app: AppId) -> io::Result<Worker> {
    let data = ely_platform::data_dir(app).map_err(io::Error::other)?;
    let install = ely_lok::install_dir()?;
    let worker = Worker::launch(&install, &data.join("lok-profile"))?;
    log::info!("{}: worker from {}", app.slug(), install.display());
    Ok(worker)
}

/// A new author name signs from the next comment on.
fn follow_author(worker: &Arc<Mutex<Worker>>, wanted: Arc<Mutex<String>>, cx: &mut App) {
    let worker = Arc::clone(worker);
    cx.observe_global::<ely_ui::settings::Current>(move |cx| {
        let author = ely_ui::settings::current(cx).author.clone();
        if *wanted.lock().expect("the author lock") == author {
            return;
        }
        *wanted.lock().expect("the author lock") = author;
        let (worker, wanted) = (Arc::clone(&worker), Arc::clone(&wanted));
        cx.background_executor()
            .spawn(async move {
                // Any order of these tasks ends on the newest name.
                let set = sign(&mut worker.lock().expect("the worker lock"), &wanted);
                if let Err(error) = set {
                    log::error!("lok: the new author name is not set: {error}");
                }
            })
            .detach();
    })
    .detach();
}

/// Mapped UNO commands on LOK tabs.
pub trait Commands: Host<Engine = Lok> {
    /// Runs a mapped command on the active tab.
    fn run_active(&mut self, id: &str, values: &[&str], cx: &mut Context<Self>) {
        self.run_in(self.core().active, id, values, cx);
    }

    /// Runs a mapped command on tab `tab`.
    fn run_in(&mut self, tab: Option<u64>, id: &str, values: &[&str], cx: &mut Context<Self>) {
        let Some(tab) = tab.and_then(|tab| self.tab(tab)) else {
            let text = text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, ely_gpui_component::primitives::Severity::Warning, cx);
        };
        log::info!("{}: runs {id}", Self::KIND.app.slug());
        // Numbers hold only in the state they were read.
        let addressed = ely_lok::commands::find(id).is_ok_and(|command| command.addressed(values));
        let sent = match (addressed, self.read_at(tab.id, cx)) {
            (true, Some(at)) => tab.view.update(cx, |view, _| view.run_at(&at, id, values)),
            _ => tab.view.update(cx, |view, _| view.run(id, values)),
        };
        if let Err(error) = sent {
            self.engine_failed(&error.to_string(), cx);
        }
    }

    /// Runs `commands` made from tab `tab`'s last read, as one.
    fn run_all_in(&mut self, tab: u64, commands: &[(&str, &[&str])], cx: &mut Context<Self>) {
        let (Some(open), Some(at)) = (self.tab(tab), self.read_at(tab, cx)) else {
            return log::warn!("{}: tab {tab} has no read to run from", Self::KIND.app.slug());
        };
        log::info!("{}: runs {} commands from one read", Self::KIND.app.slug(), commands.len());
        if let Err(error) = open.view.update(cx, |view, _| view.run_all_at(&at, commands)) {
            self.engine_failed(&error.to_string(), cx);
        }
    }

    /// Runs a numbered command against the read state `at`.
    fn run_on(&mut self, tab: u64, at: &Doc, id: &str, values: &[&str], cx: &mut Context<Self>) {
        let Some(tab) = self.tab(tab) else {
            return;
        };
        log::info!("{}: runs {id} at a read state", Self::KIND.app.slug());
        if let Err(error) = tab.view.update(cx, |view, _| view.run_at(at, id, values)) {
            self.engine_failed(&error.to_string(), cx);
        }
    }
}

impl<H: Host<Engine = Lok>> Commands for H {}

/// A worker that shuts down when the test's thread ends.
#[cfg(feature = "test-support")]
pub fn test_worker(install: &Path, profile: &Path) -> Arc<Mutex<Worker>> {
    let worker = Worker::spawn(install, profile).expect("the worker");
    let worker = Arc::new(Mutex::new(worker));
    let ending = worker.clone();
    ely_ui::at_thread_end(move || match ending.lock() {
        Ok(mut worker) => match worker.shutdown() {
            Ok(()) => {}
            Err(error) => log::warn!("lok: test worker shutdown failed: {error}"),
        },
        Err(_) => log::warn!("lok: a test panicked holding the worker"),
    });
    worker
}
