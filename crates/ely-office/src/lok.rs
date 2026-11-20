//! LibreOfficeKit as the shell's engine (§7).

use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    time::Instant,
};

use ely_lok::{Doc, LokError, LokView, Reported, Worker, sheet::csv::Import};
use ely_gpui_component::primitives::Severity;
use ely_platform::AppId;
use gpui::{App, Context, Entity, FocusHandle, Keystroke, TaskExt as _, Window};

use crate::engine::{Engine, Load, Stop};
use crate::{Host, Tabs, text};

mod early;

/// The LOK engine.
pub struct Lok;

/// A load: the document and its layout.
pub struct Loaded {
    doc: Doc,
    laid: ely_lok::Layout,
    feed: ely_lok::Feed,
    waiter: ely_lok::Waiter,
    links: Vec<String>,
    /// Word links the kernel keeps as pictures.
    pictured: bool,
}

impl Engine for Lok {
    type Worker = Worker;
    type Doc = Doc;
    type View = LokView;
    type Loaded = Loaded;
    type Import = Import;
    // One kernel: a load behind blocks the front's keys.
    const PARKS: bool = true;

    fn load(worker: &mut Worker, from: Load<Import>) -> io::Result<Loaded> {
        if let Some(early) = early::taken(worker, &from)? {
            return early;
        }
        let doc = match from {
            Load::File(path, password) => worker.unlock(&ely_lok::file_url(path), password)?,
            Load::Import(path, how) => worker.import(&ely_lok::file_url(path), how)?,
        };
        // Layout too: the window never waits.
        let laid = ely_lok::layout(worker, &doc)?;
        // Impress names no links.
        let (links, pictured) = match (doc.is_deck(), doc.is_sheet()) {
            (true, _) => (Vec::new(), false),
            (false, sheet) => {
                let listed = worker.values(&doc, ely_lok::links::LIST)?;
                let pictured = !sheet && !ely_lok::links::pictured(&listed)?.is_empty();
                (ely_lok::links::sources(&listed)?, pictured)
            }
        };
        Ok(Loaded { doc, laid, feed: worker.feed(), waiter: worker.waiter(), links, pictured })
    }

    fn open(
        worker: &Arc<Mutex<Worker>>,
        loaded: Loaded,
        _: &mut Window,
        cx: &mut App,
    ) -> io::Result<(Doc, Entity<LokView>)> {
        let Loaded { doc, laid, feed, waiter, .. } = loaded;
        let view = LokView::open(Arc::clone(worker), doc.clone(), laid, (feed, waiter), cx);
        let (zoom, deck) = (ely_ui::settings::current(cx).doc_zoom, doc.is_deck());
        let kind = match (doc.is_sheet(), deck) {
            (true, _) => "office.view.sheet",
            (_, true) => "office.view.deck",
            _ => "office.view.text",
        };
        let name = text(cx, kind, &[]);
        // Decided once: a later default leaves it.
        view.update(cx, |view, cx| {
            view.set_name(name);
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
        Ok((doc, view))
    }

    fn discard(worker: &mut Worker, loaded: Loaded) -> io::Result<()> {
        worker.close_doc(loaded.doc)
    }

    fn skipped(worker: &mut Worker, from: Load<Import>) -> io::Result<()> {
        early::skipped(worker, &from)
    }

    fn risk(loaded: &Loaded) -> Option<crate::Risk> {
        loaded.pictured.then(|| {
            log::warn!("office: a Word link stays only a picture");
            crate::Risk::Links
        })
    }

    fn unsound(worker: &mut Worker, doc: &Doc, format: &str) -> io::Result<Option<crate::Risk>> {
        Self::lost(worker, doc, format)
    }

    fn lost(worker: &mut Worker, doc: &Doc, format: &str) -> io::Result<Option<crate::Risk>> {
        if format != "docx" || doc.is_sheet() || doc.is_deck() {
            return Ok(None);
        }
        let unkept = ely_lok::links::unkept(&worker.values(doc, ely_lok::links::LIST)?)?;
        Ok((!unkept.is_empty()).then(|| {
            log::warn!("office: {} linked sections DOCX keeps only as text", unkept.len());
            crate::Risk::Unkept
        }))
    }

    fn links(loaded: &Loaded) -> Vec<String> {
        loaded.links.clone()
    }

    fn update_links(worker: &mut Worker, doc: &Doc) -> io::Result<Vec<String>> {
        ely_lok::links::failed(&worker.values(doc, ely_lok::links::UPDATE)?)
    }

    fn history(worker: &mut Worker, doc: &Doc) -> io::Result<String> {
        worker.values(doc, ".uno:Undo")
    }

    /// Only a save before a close enters a cell edit.
    fn write(worker: &mut Worker, doc: &Doc, path: &Path, format: &str, closing: bool) -> io::Result<()> {
        match closing {
            true => doc.save_closing(worker, &ely_lok::file_url(path), format),
            false => doc.save_owned(worker, &ely_lok::file_url(path), format),
        }
    }

    fn snapshot(worker: &mut Worker, doc: &Doc, path: &Path, format: &str) -> io::Result<()> {
        doc.save_as(worker, &ely_lok::file_url(path), format)
    }

    /// Waiting input stops it: `ErrorKind::Interrupted`.
    fn point(worker: &mut Worker, doc: &Doc, path: &Path, format: &str) -> io::Result<()> {
        doc.snapshot(worker, &ely_lok::file_url(path), format)
    }

    fn close(worker: &mut Worker, doc: Doc) -> io::Result<()> {
        worker.close_doc(doc)
    }

    fn retire(view: &Entity<LokView>, cx: &mut App) -> ely_gpui_component::primitives::Retired {
        view.update(cx, |view, _| view.retire_all())
    }

    fn hide(view: &Entity<LokView>, cx: &mut App) {
        view.update(cx, |view, _| view.hide());
    }

    fn replay(view: &Entity<LokView>, keys: Vec<Keystroke>, window: &mut Window, cx: &mut App) {
        view.update(cx, |view, cx| view.replay(keys, window, cx));
    }

    fn ended(worker: &mut Worker) -> Option<String> {
        worker.ended()
    }

    fn stopper(worker: &Worker) -> Option<Stop> {
        let stopper = worker.stopper();
        Some(Arc::new(move || stopper.stop()))
    }

    fn restart(_: AppId, worker: &mut Worker) -> io::Result<()> {
        let author = worker.author().map(str::to_string);
        let author = author.ok_or_else(|| io::Error::other("the ended worker had no author"))?;
        // The ended worker stays until the new one is ready.
        let mut fresh = worker.relaunch()?;
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
        view.edit_pending() || view.unconfirmed()
    }

    fn typed_at(view: &LokView) -> Option<std::time::Instant> {
        view.typed_at()
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
    ) -> Vec<gpui::Subscription> {
        vec![
            cx.subscribe(view, move |host, _, error: &LokError, cx| match error.0.starts_with(ely_lok::STALE) {
                true => failed(host, &text(cx, "office.error.changed", &[]), cx),
                false => failed(host, &error.0, cx),
            }),
            // Log the title only; the body may name user files.
            cx.subscribe(view, |host, _, closed: &ely_lok::Closed, cx| {
                log::warn!("{}: the engine opened a dialog Ely closed: {}", H::KIND.app.slug(), closed.title);
                let title = closed.title.as_str();
                let shown = match closed.body.as_str() {
                    "" => text(cx, "office.error.dialog", &[("title", title)]),
                    body => text(cx, "office.error.dialog_said", &[("title", title), ("body", body)]),
                };
                host.toast_unlogged(shown, Severity::Danger, cx);
            }),
            cx.subscribe(view, move |host, view, linked: &ely_lok::Linked, cx| follow(host, view, &linked.0, cx)),
            // Dropped files that are no pictures open as files do.
            cx.subscribe(view, |host, _, dropped: &ely_lok::Dropped, cx| ely_ui::open_dropped(H::KIND.app, &dropped.0, host.core().window, cx)),
            // The next start finds an engine waiting.
            cx.subscribe(view, |_, _, _: &ely_lok::Drawn, _| crate::resident::successor(H::KIND.app)),
            // The status shows the zoom.
            cx.subscribe(view, |_, _, _: &ely_lok::Zoomed, cx| cx.notify()),
            cx.subscribe(view, move |host, view, _: &Reported, cx| {
                let modified = view.read(cx).state(".uno:ModifiedStatus") == Some("true");
                reported(host, &view, modified, cx)
            }),
        ]
    }
}

/// Follows a link: a document place, or web and mail.
fn follow<H: Host<Engine = Lok>>(host: &mut H, view: Entity<LokView>, link: &str, cx: &mut Context<H>) {
    let slug = H::KIND.app.slug();
    if let Some(mark) = link.strip_prefix('#') {
        log::info!("{slug}: jumping to a place in the document");
        if let Err(error) = view.update(cx, |view, _| view.run("link.jump", &[mark])) {
            host.engine_failed(&error.to_string(), cx);
        }
    } else {
        match ely_ui::outward(link) {
            Ok(address) => {
                log::info!("{slug}: opening a web or mail link");
                cx.open_url(&address);
            }
            Err(notice) => host.toast(text(cx, notice, &[]), Severity::Danger, cx),
        }
    }
}

/// The worker and its stop.
type Taken = (Arc<Mutex<Worker>>, Option<Stop>);

/// The engine `take_engine` hands to `start`.
static TAKEN: Mutex<Option<Taken>> = Mutex::new(None);

/// Takes the engine; it checks, then loads the first file.
pub fn take_engine(app: AppId, files: &[PathBuf]) {
    let began = Instant::now();
    let launched = spawn(app).unwrap_or_else(|error| {
        ely_platform::startup::fail(app, &format!("the document engine did not start: {error}"))
    });
    log::info!("{}: engine taken in {} ms", app.slug(), began.elapsed().as_millis());
    let stop = Lok::stopper(&launched);
    let worker = Arc::new(Mutex::new(launched));
    let (locked, held_now) = mpsc::channel();
    let shared = Arc::clone(&worker);
    let first = files.first().cloned();
    let thread = std::thread::Builder::new().name("engine-take".into()).spawn(move || {
        // Locked before any load: the author signs first.
        let mut held = worker.lock().expect("the worker lock");
        if locked.send(()).is_err() {
            return log::error!("{}: the take stopped waiting for the lock", app.slug());
        }
        let ready = held.ready().and_then(|()| sign(&mut held, &Mutex::new(early::author())));
        match ready {
            Ok(()) => log::info!("{}: engine ready {} ms after the take began", app.slug(), began.elapsed().as_millis()),
            Err(error) => ely_platform::startup::fail(app, &format!("the document engine did not start: {error}")),
        }
        // The window opens meanwhile; its load takes this.
        if let Some(path) = first.filter(|path| crate::files::check_package(path).is_ok()) {
            early::load(&mut held, path);
            log::info!("{}: launch file loaded early, {} ms after the take began", app.slug(), began.elapsed().as_millis());
        }
    });
    if let Err(error) = thread {
        ely_platform::startup::fail(app, &format!("the engine thread did not start: {error}"));
    }
    if held_now.recv().is_err() {
        ely_platform::startup::fail(app, "the engine thread ended before it held the worker");
    }
    match TAKEN.lock() {
        Ok(mut slot) => *slot = Some((shared, stop)),
        Err(_) => ely_platform::startup::fail(app, "the engine slot is poisoned"),
    }
}

/// The worker, its stop and the scratch root of `app`.
pub fn start(app: AppId, cx: &mut App) -> io::Result<(Arc<Mutex<Worker>>, Option<Stop>, PathBuf)> {
    let data = ely_platform::data_dir(app).map_err(io::Error::other)?;
    let work = crate::fresh_work(&data)?;
    let taken = TAKEN.lock().map_err(|_| io::Error::other("the engine slot is poisoned"))?.take();
    let taken = taken.ok_or_else(|| io::Error::other("no engine was taken before the window"))?;
    let (worker, stop) = taken;
    // The take signed the launch settings' name.
    let wanted = Arc::new(Mutex::new(ely_ui::settings::current(cx).author.clone()));
    follow_author(&worker, wanted, cx);
    Ok((worker, stop, work))
}

/// Sends the newest wanted name, read under the worker lock.
fn sign(worker: &mut Worker, wanted: &Mutex<String>) -> io::Result<()> {
    let author = wanted.lock().expect("the author lock").clone();
    worker.set_author(&ely_platform::user::named(&author)?)
}

/// The idle engine, else a new one; checked later.
fn spawn(app: AppId) -> io::Result<Worker> {
    let (install, profile, probe) = crate::resident::place(app)?;
    crate::resident::used(app)?;
    let worker = Worker::take(&install, &profile, app.slug(), probe)?;
    #[cfg(target_os = "linux")]
    crate::resident::unshadow(app);
    log::info!("{}: worker from {}, start check {probe:?}", app.slug(), install.display());
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
        self.run_in(self.core().active(), id, values, cx);
    }

    /// Runs a mapped command on tab `tab`.
    fn run_in(&mut self, tab: Option<u64>, id: &str, values: &[&str], cx: &mut Context<Self>) {
        let Some(tab) = tab.and_then(|tab| self.tab(tab)) else {
            let text = text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
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

    /// Writes tab `id` in its input order, as an edit.
    fn write_in<T: Send + 'static>(
        &mut self,
        id: u64,
        write: impl FnOnce(&mut Worker, &Doc) -> io::Result<T> + Send + 'static,
        then: impl FnOnce(&mut Self, io::Result<T>, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        let slug = Self::KIND.app.slug();
        let Some(tab) = self.tab(id) else {
            return log::warn!("{slug}: tab {id} is gone; nothing written");
        };
        let (done, outcome) = futures::channel::oneshot::channel();
        let busy = std::rc::Rc::clone(&tab.busy);
        tab.view.update(cx, |view, _| {
            view.write(move |worker, doc| {
                if done.send(write(worker, doc)).is_err() {
                    log::warn!("{slug}: tab {id}'s write ended unheard");
                }
            })
        });
        cx.spawn(async move |this, cx| {
            let _busy = busy;
            // A view gone with the write queued drops it.
            let written = outcome.await.unwrap_or_else(|_| Err(io::Error::other(format!("tab {id} closed before its write"))));
            if let Err(error) = &written {
                log::warn!("{slug}: tab {id}'s write failed: {error}");
            }
            crate::in_window(&this, cx, |host, window, cx| then(host, written, window, cx))
        })
        .detach_and_log_err(cx);
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

/// Loads `path` as a launch does; its document.
#[cfg(feature = "test-support")]
pub fn test_load_early(worker: &Arc<Mutex<Worker>>, path: &Path) -> Doc {
    early::load(&mut worker.lock().expect("the worker"), path.to_path_buf());
    early::doc().expect("the early load")
}

/// The early load's document, while one waits.
#[cfg(feature = "test-support")]
pub fn test_early_doc() -> Option<Doc> {
    early::doc()
}

/// A worker stopped, then its profile freed, at thread end.
#[cfg(feature = "test-support")]
pub fn test_worker(install: &Path) -> (Arc<Mutex<Worker>>, Option<Stop>) {
    let profile = tempfile::tempdir().expect("a profile dir");
    let worker = Worker::spawn(install, profile.path()).expect("the worker");
    let stop = Lok::stopper(&worker);
    let worker = Arc::new(Mutex::new(worker));
    let ending = worker.clone();
    ely_ui::at_thread_end(move || {
        let mut worker = ending.lock().unwrap_or_else(|poisoned| {
            log::warn!("lok: a test panicked holding the worker");
            poisoned.into_inner()
        });
        if let Err(error) = worker.shutdown() {
            log::warn!("lok: test worker shutdown failed: {error}");
        }
        drop(worker);
        drop(profile);
    });
    (worker, stop)
}
