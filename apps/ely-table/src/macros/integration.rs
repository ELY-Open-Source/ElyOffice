//! R macros against the real kernel.

use super::{
    RunRMacro,
    protocol::{Snapshot, refusal},
    rscript, runner,
};
use crate::{
    book::Book,
    dialog::Dialog,
    tests::{book, settle, update},
};
use ely_lok::{Doc, Worker};
use ely_office::prelude::*;
use gpui::{AppContext as _, TestAppContext};
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};

fn snapshot(worker: &mut Worker, doc: &Doc) -> String {
    worker
        .values(doc, ely_lok::sheet::MACRO)
        .expect("typed selection")
}

fn automatic_styles(xml: &str) -> &str {
    xml.split("<office:automatic-styles>")
        .nth(1)
        .expect("automatic styles")
        .split("</office:automatic-styles>")
        .next()
        .expect("style contents")
}

#[test]
fn real_r_demo_values_format_errors_and_one_undo() {
    let Some(install) = ely_lok::test_install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let Ok(r) = rscript::locate() else {
        eprintln!("SKIP: Rscript is not installed");
        return;
    };
    let dir = tempfile::tempdir().expect("directory");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("worker");
    let source = dir.path().join("demo.fods");
    std::fs::write(
        &source,
        include_str!("../../../../tests/fixtures/table/r-macro-demo.fods"),
    )
    .expect("fixture");
    let doc = worker.load(&ely_lok::file_url(&source)).expect("document");
    doc.visible_area(&mut worker, (0, 0, 15000, 10000), (256, 3840))
        .expect("viewport");
    worker
        .run(&doc, "cell.goto", &["A1:E3"])
        .expect("selection");
    let before = snapshot(&mut worker, &doc);
    let geometry = worker
        .values(&doc, ely_lok::sheet::GEOMETRY)
        .expect("column widths");
    let script = super::EXAMPLE;
    let stop = AtomicBool::new(false);
    let copy = Snapshot::parse(&before).expect("snapshot");
    let failed = runner::run(&r, &copy, &format!("{script}\nstop(\"falló\")"), &stop);
    assert!(failed.batch.is_err());
    assert!(failed.log.contains("falló"));
    assert_eq!(snapshot(&mut worker, &doc), before);
    let at = worker.now(&doc).expect("revision before R");
    let outcome = runner::run(&r, &copy, script, &stop);
    let batch = outcome.batch.expect("successful R batch");
    worker
        .run(&at, "macro.apply", &[&batch])
        .expect("batch applied");
    let after: Value = serde_json::from_str(&snapshot(&mut worker, &doc)).expect("JSON");
    let original: Value = serde_json::from_str(&before).expect("original JSON");
    assert_eq!(after["cells"][0]["value"], "Nombre");
    assert_eq!(after["cells"][5]["value"], "José \"Pepe\"");
    assert_eq!(after["cells"][6]["value"], "00123");
    assert_eq!(after["cells"][9], original["cells"][9], "formula kept");
    assert_eq!(after["cells"][14], original["cells"][14], "formula kept");
    assert_ne!(
        worker
            .values(&doc, ely_lok::sheet::GEOMETRY)
            .expect("adjusted widths"),
        geometry
    );
    let formatted = dir.path().join("formatted.fods");
    doc.save_as(&mut worker, &ely_lok::file_url(&formatted), "fods")
        .expect("formatted document");
    let xml = std::fs::read_to_string(&formatted).expect("formatted XML");
    assert!(automatic_styles(&xml).contains("fo:font-weight=\"bold\""));
    worker.run(&doc, "edit.undo", &[]).expect("one undo");
    worker
        .run(&doc, "cell.goto", &["A1:E3"])
        .expect("selection restored");
    assert_eq!(snapshot(&mut worker, &doc), before);
    assert_eq!(
        worker
            .values(&doc, ely_lok::sheet::GEOMETRY)
            .expect("restored widths"),
        geometry
    );
    let undone = dir.path().join("undone.fods");
    doc.save_as(&mut worker, &ely_lok::file_url(&undone), "fods")
        .expect("undone document");
    let xml = std::fs::read_to_string(&undone).expect("undone XML");
    assert!(!automatic_styles(&xml).contains("fo:font-weight=\"bold\""));
}

#[test]
fn a_stopped_batch_never_reaches_the_kernel() {
    let Some(install) = ely_lok::test_install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("directory");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("worker");
    let source = dir.path().join("demo.fods");
    std::fs::write(&source, include_str!("../../../../tests/fixtures/table/r-macro-demo.fods")).expect("fixture");
    let doc = worker.load(&ely_lok::file_url(&source)).expect("document");
    let at = worker.now(&doc).expect("revision");
    let stopped = super::apply(&mut worker, &at, "{}", &AtomicBool::new(true)).expect_err("stopped");
    assert_eq!(refusal(&stopped), Some("table.r.stopped"));
    // Unstopped, the same batch reaches the kernel, which refuses it.
    let sent = super::apply(&mut worker, &at, "{}", &AtomicBool::new(false)).expect_err("kernel refuses {}");
    assert_eq!(refusal(&sent), Some("table.r.invalid"), "{sent}");
}

#[test]
fn a_cell_set_while_r_runs_makes_the_batch_stale() {
    let dir = tempfile::tempdir().expect("directory");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    cx.update(ely_gpui_component::init_for_tests);
    update(&mut cx, &book, |book, cx| book.new_blank(cx));
    settle(&cx, |app| {
        book.read(app).active_tab().is_some_and(|tab| {
            tab.view.read(app).settled() && !tab.extra.sheets.list.is_empty()
        })
    });
    cx.update_window(window.into(), |_, window, app| {
        book.update(app, |book, cx| book.on_r_macro(&RunRMacro, window, cx))
    })
    .expect("the window lives");
    // R reads; then `ely-table cell set` lands.
    let (tab, doc, at, batch) = cx.update(|app| {
        let book = book.read(app);
        let tab = book.active_tab().expect("a tab");
        let b1 = format!("{}!B1", tab.extra.sheets.list[0].name);
        let mut worker = book.core.worker.lock().expect("the worker lock");
        worker.run(&tab.doc, "cell.goto", &["A1"]).expect("selection");
        let at = worker.now(&tab.doc).expect("revision");
        let batch = untouched(&mut worker, &tab.doc);
        worker.run(&tab.doc, "cell.set", &[&b1, "from the command line"]).expect("cell set");
        (tab.id, tab.doc.clone(), at, batch)
    });
    let run = Arc::new(AtomicBool::new(false));
    update(&mut cx, &book, |book, cx| {
        if let Some(Dialog::RMacro(editor)) = &mut book.dialog {
            editor.running = Some(Arc::clone(&run));
        }
        let outcome = runner::Outcome { log: String::new(), batch: Ok(batch) };
        book.ran_r(tab, at, outcome, Arc::clone(&run), cx);
    });
    settle(&cx, |app| {
        matches!(&book.read(app).dialog, Some(Dialog::RMacro(editor)) if editor.running.is_none())
    });
    let (stale, output) = cx.update(|app| {
        let Some(Dialog::RMacro(editor)) = &book.read(app).dialog else {
            panic!("the R editor is open");
        };
        (Book::text(app, "table.r.stale", &[]).to_string(), editor.output.clone())
    });
    assert!(output.contains(&stale), "{output}");
    let first = cx.update(|app| {
        let mut worker = book.read(app).core.worker.lock().expect("the worker lock");
        worker.run(&doc, "cell.goto", &["A1"]).expect("selection");
        snapshot(&mut worker, &doc)
    });
    assert!(!first.contains("from R"), "the stale batch never landed: {first}");
}

#[test]
fn a_window_edit_queued_while_r_runs_lands_first() {
    let dir = tempfile::tempdir().expect("directory");
    let mut cx = TestAppContext::single();
    let Some((window, book)) = book(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    cx.update(ely_gpui_component::init_for_tests);
    update(&mut cx, &book, |book, cx| book.new_blank(cx));
    settle(&cx, |app| {
        book.read(app).active_tab().is_some_and(|tab| tab.view.read(app).settled())
    });
    cx.update_window(window.into(), |_, window, app| {
        book.update(app, |book, cx| book.on_r_macro(&RunRMacro, window, cx))
    })
    .expect("the window lives");
    let (tab, doc, at, batch) = cx.update(|app| {
        let book = book.read(app);
        let tab = book.active_tab().expect("a tab");
        let mut worker = book.core.worker.lock().expect("the worker lock");
        worker.run(&tab.doc, "cell.goto", &["A1"]).expect("selection");
        let at = worker.now(&tab.doc).expect("revision");
        (tab.id, tab.doc.clone(), at, untouched(&mut worker, &tab.doc))
    });
    let run = Arc::new(AtomicBool::new(false));
    update(&mut cx, &book, |book, cx| {
        if let Some(Dialog::RMacro(editor)) = &mut book.dialog {
            editor.running = Some(Arc::clone(&run));
        }
        // Queued, not landed: the batch could take the worker first.
        book.run_active("cell.enter", &["typed"], cx);
        let outcome = runner::Outcome { log: String::new(), batch: Ok(batch) };
        book.ran_r(tab, at, outcome, Arc::clone(&run), cx);
        // Not sent: it waits behind the queued input.
        assert!(book.core.waiting.iter().any(|(id, _)| *id == tab), "the batch went first");
    });
    settle(&cx, |app| {
        matches!(&book.read(app).dialog, Some(Dialog::RMacro(editor)) if editor.running.is_none())
    });
    let output = cx.update(|app| match &book.read(app).dialog {
        Some(Dialog::RMacro(editor)) => editor.output.clone(),
        _ => panic!("the R editor is open"),
    });
    let stale = cx.update(|app| Book::text(app, "table.r.stale", &[]).to_string());
    assert!(output.contains(&stale), "{output}");
    let first = cx.update(|app| {
        let mut worker = book.read(app).core.worker.lock().expect("the worker lock");
        worker.run(&doc, "cell.goto", &["A1"]).expect("selection");
        snapshot(&mut worker, &doc)
    });
    assert!(first.contains("typed") && !first.contains("from R"), "{first}");
}

/// The batch R would send unchanged for the selection now.
fn untouched(worker: &mut Worker, doc: &Doc) -> String {
    let snapshot: Value = serde_json::from_str(&snapshot(worker, doc)).expect("JSON");
    let first = json!({"index": 0, "kind": "text", "value": "from R"});
    json!({"snapshot": snapshot, "writes": [first], "bold": null, "autofit": false}).to_string()
}

#[test]
fn each_kernel_refusal_names_its_reason() {
    let Some(install) = ely_lok::test_install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("directory");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("worker");
    let source = dir.path().join("demo.fods");
    std::fs::write(&source, include_str!("../../../../tests/fixtures/table/r-macro-demo.fods")).expect("fixture");
    let doc = worker.load(&ely_lok::file_url(&source)).expect("document");
    let apply = |worker: &mut Worker, batch: &str| {
        let at = worker.now(&doc).expect("revision");
        refusal(&super::apply(worker, &at, batch, &AtomicBool::new(false)).expect_err("refused"))
    };
    worker.run(&doc, "cell.goto", &["A1:A10001"]).expect("a tall selection");
    let read = Snapshot::parse(&snapshot(&mut worker, &doc)).expect_err("too many cells");
    assert_eq!(refusal(&read), Some("table.r.cells"));
    worker.run(&doc, "cell.goto", &["A1:B2"]).expect("selection");
    let batch = untouched(&mut worker, &doc);
    worker.run(&doc, "cell.goto", &["A1:B3"]).expect("another selection");
    assert_eq!(apply(&mut worker, &batch), Some("table.r.stale"));
    worker.run(&doc, "cell.goto", &["A1:B2"]).expect("selection");
    let lock = ely_lok::sheet::protect_json("", &[]);
    worker.run(&doc, "sheet.protect", &[&lock]).expect("protected");
    assert_eq!(apply(&mut worker, &batch), Some("table.r.locked"));
    let mut bold: Value = serde_json::from_str(&batch).expect("JSON");
    bold["writes"] = json!([]);
    bold["bold"] = json!(true);
    assert_eq!(apply(&mut worker, &bold.to_string()), Some("table.r.protected"));
    worker.run(&doc, "sheet.protect", &[&ely_lok::sheet::unprotect_json("")]).expect("unprotected");
    worker.run(&doc, "cell.goto", &["A2:B2"]).expect("a row");
    worker.run(&doc, "cells.merge", &["false"]).expect("merged");
    worker.run(&doc, "cell.goto", &["A1:B2"]).expect("selection");
    let batch = untouched(&mut worker, &doc);
    assert_eq!(apply(&mut worker, &batch), Some("table.r.merged"));
}

/// The demo workbook in a fresh worker, A1:B2 selected.
fn demo(dir: &std::path::Path) -> Option<(Worker, Doc)> {
    let install = ely_lok::test_install()?;
    let mut worker = Worker::spawn(&install, &dir.join("profile")).expect("worker");
    let source = dir.join("demo.fods");
    std::fs::write(&source, include_str!("../../../../tests/fixtures/table/r-macro-demo.fods")).expect("fixture");
    let doc = worker.load(&ely_lok::file_url(&source)).expect("document");
    worker.run(&doc, "cell.goto", &["A1:B2"]).expect("selection");
    Some((worker, doc))
}

#[test]
fn a_cell_set_waiting_for_the_worker_lands_after_the_batch() {
    let dir = tempfile::tempdir().expect("directory");
    let Some((mut worker, doc)) = demo(dir.path()) else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let (at, batch) = (worker.now(&doc).expect("revision"), untouched(&mut worker, &doc));
    let shared = std::sync::Mutex::new(worker);
    let asked = AtomicBool::new(false);
    std::thread::scope(|scope| {
        let mut held = shared.lock().expect("the worker lock");
        let cli = scope.spawn(|| {
            asked.store(true, std::sync::atomic::Ordering::SeqCst);
            let mut worker = shared.lock().expect("the worker lock");
            worker.run(&doc, "cell.set", &["Demo!D9", "from the command line"]).expect("cell set");
        });
        while !asked.load(std::sync::atomic::Ordering::SeqCst) {
            std::thread::yield_now();
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(!cli.is_finished(), "the cell set waits for the worker");
        // Check and write share the lock; the set waits.
        super::apply(&mut held, &at, &batch, &AtomicBool::new(false)).expect("R's batch is current");
    });
    let mut worker = shared.into_inner().expect("the worker");
    let undo = worker.values(&doc, ".uno:Undo").expect("undo history");
    let steps = ely_lok::history::steps(&undo).expect("steps");
    assert!(steps.len() >= 2 && steps[1].0.starts_with("R macro"), "the set followed the batch: {steps:?}");
    worker.run(&doc, "cell.goto", &["A1"]).expect("A1");
    assert!(snapshot(&mut worker, &doc).contains("from R"));
}

#[test]
fn a_lost_worker_says_changes_may_have_landed() {
    let dir = tempfile::tempdir().expect("directory");
    let Some((mut worker, doc)) = demo(dir.path()) else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let (at, batch) = (worker.now(&doc).expect("revision"), untouched(&mut worker, &doc));
    worker.stopper().stop().expect("the worker ends");
    let lost = super::apply(&mut worker, &at, &batch, &AtomicBool::new(false)).expect_err("no reply");
    assert_eq!(refusal(&lost), Some("table.r.unsure"), "{lost}");
}
