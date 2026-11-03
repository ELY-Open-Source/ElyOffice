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
fn a_queued_edit_drops_the_batch_it_would_follow() {
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
    let (tab, at, edited) = cx.update(|app| {
        let book = book.read(app);
        let tab = book.active_tab().expect("a tab");
        let at = book.core.worker.lock().expect("the worker lock").now(&tab.doc).expect("revision");
        (tab.id, at, tab.view.read(app).edited())
    });
    let run = Arc::new(AtomicBool::new(false));
    update(&mut cx, &book, |book, cx| {
        if let Some(Dialog::RMacro(editor)) = &mut book.dialog {
            editor.running = Some(Arc::clone(&run));
        }
        // Queued, not landed: the batch could take the worker first.
        book.run_active("cell.enter", &["typed"], cx);
        let outcome = runner::Outcome { log: String::new(), batch: Ok("{}".into()) };
        book.ran_r(tab, (at, edited), outcome, Arc::clone(&run), cx);
    });
    // Nothing ran yet: the batch never went to the worker.
    let (stale, output, running) = cx.update(|app| {
        let Some(Dialog::RMacro(editor)) = &book.read(app).dialog else {
            panic!("the R editor is open");
        };
        let stale = Book::text(app, "table.r.stale", &[]).to_string();
        (stale, editor.output.clone(), editor.running.is_some())
    });
    assert!(!running && output.contains(&stale), "{output}");
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
    assert_eq!(apply(&mut worker, &batch), Some("table.r.protected"));
    worker.run(&doc, "sheet.protect", &[&ely_lok::sheet::unprotect_json("")]).expect("unprotected");
    worker.run(&doc, "cell.goto", &["A2:B2"]).expect("a row");
    worker.run(&doc, "cells.merge", &["false"]).expect("merged");
    worker.run(&doc, "cell.goto", &["A1:B2"]).expect("selection");
    let batch = untouched(&mut worker, &doc);
    assert_eq!(apply(&mut worker, &batch), Some("table.r.merged"));
}

