//! The real example changes values and formatting together.

use super::{protocol::Snapshot, runner};
use ely_lok::{Doc, Worker};
use serde_json::Value;

fn snapshot(worker: &mut Worker, doc: &Doc) -> String {
    worker
        .values(doc, ".uno:ElyMacroSelection")
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
    if !runner::available() {
        eprintln!("SKIP: Rscript is not installed");
        return;
    }
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
    let script = include_str!("../../../../examples/macros/limpiar-textos.R");
    let failed = runner::run(
        Snapshot::parse(&before).expect("snapshot"),
        format!("{script}\nstop(\"falló\")"),
    );
    assert!(failed.batch.is_err());
    assert!(failed.log.contains("falló"));
    assert_eq!(snapshot(&mut worker, &doc), before);
    let at = worker.now(&doc).expect("revision before R");
    let outcome = runner::run(Snapshot::parse(&before).expect("snapshot"), script.into());
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
