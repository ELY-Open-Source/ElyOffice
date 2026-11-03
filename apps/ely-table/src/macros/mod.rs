//! Explicit, experimental R macros over the selected range.

mod contain;
mod files;
#[cfg(test)]
mod integration;
mod protocol;
mod rscript;
mod runner;

use crate::{book::Book, dialog::Dialog};
use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::{Input, TextInput},
    overlays::Dialog as Card,
};
use ely_office::{in_window, prelude::*};
use gpui::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, TaskExt as _, Window, actions, div,
};
use std::{
    io,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

const EXAMPLE: &str = include_str!("example.R");
actions!(ely_table, [RunRMacro]);

pub struct Editor {
    pub tab: u64,
    pub code: Entity<TextInput>,
    pub output: String,
    /// Set while R runs; storing true stops it.
    pub running: Option<Arc<AtomicBool>>,
    /// The file request whose result this editor takes.
    pub asked: Option<u64>,
}

impl Editor {
    fn runs(&self, run: &Arc<AtomicBool>) -> bool {
        self.running.as_ref().is_some_and(|own| Arc::ptr_eq(own, run))
    }
}

/// A closed editor stops its R.
impl Drop for Editor {
    fn drop(&mut self) {
        if let Some(stop) = &self.running {
            stop.store(true, Ordering::Relaxed);
        }
    }
}

pub fn menu(cx: &App) -> gpui::Menu {
    gpui::Menu::new(Book::text(cx, "table.r.menu", &[])).items([gpui::MenuItem::action(
        Book::text(cx, "table.r.title", &[]),
        RunRMacro,
    )])
}

/// `error` in the interface language when ElyTable words it.
fn said(error: &io::Error, cx: &App) -> String {
    if let Some(key) = protocol::refusal(error) {
        return Book::text(cx, key, &[]).to_string();
    }
    let inner = error.get_ref();
    if let Some(exited) = inner.and_then(|e| e.downcast_ref::<runner::Exited>()) {
        return Book::text(cx, "table.r.exited", &[("status", &exited.to_string())]).to_string();
    }
    match inner.and_then(|e| e.downcast_ref::<protocol::Unreadable>()) {
        Some(why) => Book::text(cx, "table.r.selection", &[("error", &why.0)]).to_string(),
        None => error.to_string(),
    }
}

impl Book {
    pub fn on_r_macro(&mut self, _: &RunRMacro, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active() else {
            return self.warn_key("command.blocked.no_document", cx);
        };
        let code = self
            .r_code
            .get_or_insert_with(|| {
                let label = Self::text(cx, "table.r.code", &[]);
                cx.new(|cx| {
                    let mut field = TextInput::new(window, cx).multi_line(10, 16).label(label);
                    field.set_text(EXAMPLE, cx);
                    field
                })
            })
            .clone();
        let output = String::new();
        self.open_dialog(Dialog::RMacro(Box::new(Editor { tab, code, output, running: None, asked: None })));
        cx.notify();
    }

    /// Adds `text` to the open editor's output.
    pub(super) fn r_note(&mut self, text: &str, cx: &mut Context<Self>) {
        if let Some(Dialog::RMacro(editor)) = &mut self.dialog {
            editor.output.push_str(text);
            cx.notify();
        }
    }

    /// As `r_note`, only while `run` is the editor's.
    fn r_said(&mut self, run: &Arc<AtomicBool>, text: &str, ended: bool, cx: &mut Context<Self>) {
        let Some(Dialog::RMacro(editor)) = &mut self.dialog else {
            return;
        };
        if editor.runs(run) {
            editor.output.push_str(text);
            if ended {
                editor.running = None;
            }
            cx.notify();
        }
    }

    fn stop_r(&mut self) {
        if let Some(Dialog::RMacro(editor)) = &self.dialog
            && let Some(stop) = &editor.running
        {
            log::info!("table: R macro asked to stop");
            stop.store(true, Ordering::Relaxed);
        }
    }

    fn execute_r(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::RMacro(editor)) = &mut self.dialog else {
            return;
        };
        if editor.running.is_some() {
            return;
        }
        editor.output.clear();
        let (tab, script) = (editor.tab, editor.code.read(cx).text().to_string());
        let rscript = match rscript::locate() {
            Ok(path) => path,
            Err(error) => return self.r_note(&said(&error, cx), cx),
        };
        if !self.settled(tab, cx) {
            return self.r_note(&Self::text(cx, "table.r.busy", &[]), cx);
        }
        let run = Arc::new(AtomicBool::new(false));
        if let Some(Dialog::RMacro(editor)) = &mut self.dialog {
            editor.running = Some(Arc::clone(&run));
        }
        cx.notify();
        log::info!("table: R macro reads tab {tab}");
        self.read_in(
            tab,
            |worker, doc| {
                let at = worker.now(doc)?;
                let snapshot = protocol::Snapshot::parse(&worker.values(doc, ely_lok::sheet::MACRO)?)?;
                Ok((at, snapshot))
            },
            move |book, read, _, cx| match read {
                Ok(read) => book.run_r(tab, rscript, read, script, run, cx),
                Err(error) => book.r_said(&run, &said(&error, cx), true, cx),
            },
            cx,
        );
    }

    /// R runs on its own thread, never the executor's.
    fn run_r(
        &mut self,
        tab: u64,
        rscript: PathBuf,
        (at, snapshot): (ely_lok::Doc, protocol::Snapshot),
        script: String,
        run: Arc<AtomicBool>,
        cx: &mut Context<Self>,
    ) {
        log::info!("table: R macro reads {} cells", snapshot.rows * snapshot.columns);
        let (done, result) = futures::channel::oneshot::channel();
        let stop = Arc::clone(&run);
        std::thread::spawn(move || done.send(runner::run(&rscript, &snapshot, &script, &stop)));
        cx.spawn(async move |this, cx| {
            let outcome = result.await;
            in_window(&this, cx, |book, _, cx| match outcome {
                Ok(outcome) => book.ran_r(tab, at, outcome, run, cx),
                Err(error) => book.r_said(&run, &format!("\n{error}"), true, cx),
            })
        })
        .detach_and_log_err(cx);
    }

    fn ran_r(
        &mut self,
        tab: u64,
        at: ely_lok::Doc,
        outcome: runner::Outcome,
        run: Arc<AtomicBool>,
        cx: &mut Context<Self>,
    ) {
        let open = matches!(&self.dialog, Some(Dialog::RMacro(editor)) if editor.runs(&run));
        if !open || self.tab(tab).is_none() {
            return log::info!("table: R macro closed; its batch is dropped");
        }
        self.r_said(&run, &outcome.log, false, cx);
        let batch = match outcome.batch {
            Ok(batch) => batch,
            Err(error) => return self.r_said(&run, &format!("\n{}", said(&error, cx)), true, cx),
        };
        // Queued input lands first, so the revision check sees it.
        if !self.settled(tab, cx) {
            let outcome = runner::Outcome { log: String::new(), batch: Ok(batch) };
            return self.when_settled(tab, move |book, _, cx| book.ran_r(tab, at, outcome, run, cx));
        }
        log::info!("table: R macro applies {} bytes to tab {tab}", batch.len());
        let stop = Arc::clone(&run);
        self.read_in(
            tab,
            move |worker, _| apply(worker, &at, &batch, &stop),
            move |book, result, _, cx| {
                let text = match result {
                    Ok(()) => Self::text(cx, "table.r.done", &[]),
                    Err(error) if ely_lok::is_stale(&error) => Self::text(cx, "table.r.stale", &[]),
                    Err(error) if protocol::refusal(&error).is_some() => said(&error, cx).into(),
                    Err(error) => {
                        log::warn!("table: R macro batch refused: {error}");
                        Self::text(cx, "table.r.refused", &[("error", &error.to_string())])
                    }
                };
                book.r_said(&run, &format!("\n{text}"), true, cx);
            },
            cx,
        );
    }
}

/// A stop or close before the worker takes the batch.
fn apply(worker: &mut ely_lok::Worker, at: &ely_lok::Doc, batch: &str, stop: &AtomicBool) -> io::Result<()> {
    if stop.load(Ordering::Relaxed) {
        log::info!("table: R macro stopped before its batch applied");
        return Err(protocol::refused("table.r.stopped"));
    }
    // Any writer since R's read makes `at` stale.
    match worker.run(at, "macro.apply", &[batch]) {
        // Read under this lock: no other batch interleaves.
        Err(error) if !ely_lok::is_stale(&error) => match worker.values(at, ely_lok::sheet::MACRO_REFUSAL) {
            Ok(reason) => Err(protocol::kernel_refusal(&reason, error)),
            Err(lost) => Err(protocol::unsure(&error, &format!("no reason: {lost}"))),
        },
        done => done,
    }
}

pub fn dialog(
    this: Entity<Book>,
    editor: &Editor,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let content = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(Book::text(cx, "table.r.permissions", &[]))
        .child(Input::new(&editor.code))
        .child(div().id("r-output").max_h_40().overflow_y_scroll().child(editor.output.clone()));
    let (open, save) = (this.clone(), this.clone());
    let running = editor.running.is_some();
    let open_label = Book::text(cx, "table.r.open", &[]);
    let save_label = Book::text(cx, "table.r.save", &[]);
    let card = Card::new("table-r-macro", Book::text(cx, "table.r.title", &[]), dismiss)
        .child(content)
        .action(move |_| {
            Button::new("r-open", open_label)
                .disabled(running)
                .on_click(move |_, _, cx| open.update(cx, |book, cx| book.r_file(false, cx)))
        })
        .action(move |_| {
            Button::new("r-save", save_label)
                .disabled(running)
                .on_click(move |_, _, cx| save.update(cx, |book, cx| book.r_file(true, cx)))
        });
    if running {
        let label = Book::text(cx, "table.r.stop", &[]);
        return card.action(move |_| {
            Button::new("r-stop", label).on_click(move |_, _, cx| this.update(cx, |book, _| book.stop_r()))
        });
    }
    let label = Book::text(cx, "table.r.run", &[]);
    card.action(move |_| {
        Button::new("r-run", label)
            .variant(ButtonVariant::Primary)
            .on_click(move |_, _, cx| this.update(cx, |book, cx| book.execute_r(cx)))
    })
}
