//! The real example changes values and formatting together.

use super::{protocol::Snapshot, rscript, runner};
use ely_lok::{Doc, Worker};
use serde_json::Value;
use std::sync::atomic::AtomicBool;

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
