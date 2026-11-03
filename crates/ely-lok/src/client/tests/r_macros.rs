//! A batch changes values and format in one undo step.

use super::{sheet::open_sheet, *};
use serde_json::{Value, json};

fn snapshot(worker: &mut Worker, doc: &Doc) -> Value {
    serde_json::from_str(
        &worker
            .values(doc, crate::sheet::MACRO)
            .expect("typed selection"),
    )
    .expect("JSON")
}

#[test]
fn r_batches_validate_conflicts_preserve_formulas_and_undo_together() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    // Latin text avoids unrelated stale CJK row heights.
    let (_dir, mut worker, doc) =
        super::sheet::open_book(&super::input::FODS.replace("Hello 世界", "Hello"));
    worker
        .run(&doc, "cell.set", &["S!A1", "'  á \"00123\"  "])
        .expect("text");
    worker
        .run(&doc, "cell.set", &["S!B1", "=1+1"])
        .expect("formula");
    worker
        .run(&doc, "cell.goto", &["A1:C1"])
        .expect("selection");
    let before = snapshot(&mut worker, &doc);
    assert_eq!(before["cells"][0]["kind"], "text");
    assert_eq!(before["cells"][1]["formula"], "=1+1");
    assert_eq!(before["cells"][1]["value"], "2");
    assert_eq!(before["cells"][2]["kind"], "empty");
    let geometry = worker
        .values(&doc, crate::sheet::GEOMETRY)
        .expect("column widths");
    let history = crate::history::steps(&worker.values(&doc, ".uno:Undo").expect("undo history"))
        .expect("steps");
    let batch = |writes: Value| {
        json!({"snapshot": before, "writes": writes, "bold": true, "autofit": true}).to_string()
    };
    let valid = json!([{"index":0,"kind":"text","value":"00123"}]);
    let mut invalid = valid.clone();
    invalid
        .as_array_mut()
        .expect("writes")
        .push(json!({"index":1,"kind":"number","value":"7"}));
    let at = worker.now(&doc).expect("revision");
    assert!(worker.run(&at, "macro.apply", &[&batch(invalid)]).is_err());
    assert_eq!(snapshot(&mut worker, &doc), before, "no partial values");
    assert_eq!(
        worker.values(&doc, crate::sheet::GEOMETRY).expect("widths"),
        geometry,
        "no partial format"
    );
    let at = worker.now(&doc).expect("revision");
    worker
        .run(&doc, "cell.set", &["S!D5", "changed while R ran"])
        .expect("concurrent edit");
    assert!(crate::is_stale(
        &worker
            .run(&at, "macro.apply", &[&batch(valid.clone())])
            .expect_err("stale batch")
    ));
    assert_eq!(snapshot(&mut worker, &doc), before);
    worker
        .run(&doc, "edit.undo", &[])
        .expect("concurrent edit undone");
    worker
        .run(&doc, "cell.goto", &["A1:C1"])
        .expect("selection restored");
    let at = worker.now(&doc).expect("revision");
    worker
        .run(&at, "macro.apply", &[&batch(valid)])
        .expect("batch applied");
    let after = snapshot(&mut worker, &doc);
    assert_eq!(after["cells"][0]["value"], "00123");
    assert_eq!(after["cells"][0]["kind"], "text");
    assert_eq!(after["cells"][1], before["cells"][1], "formula kept");
    worker.run(&doc, "edit.undo", &[]).expect("one undo");
    worker
        .run(&doc, "cell.goto", &["A1:C1"])
        .expect("selection restored");
    assert_eq!(snapshot(&mut worker, &doc), before);
    assert_eq!(
        worker.values(&doc, crate::sheet::GEOMETRY).expect("widths"),
        geometry
    );
    assert_eq!(
        crate::history::steps(&worker.values(&doc, ".uno:Undo").expect("history")).expect("steps"),
        history
    );
}

#[test]
fn r_snapshot_preserves_mixed_cell_types() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = super::sheet::open_book(include_str!(
        "../../../../../tests/fixtures/table/r-macro-demo.fods"
    ));
    worker
        .run(&doc, "cell.goto", &["A1:E3"])
        .expect("selection");
    let data = snapshot(&mut worker, &doc);
    let cells = &data["cells"];
    assert_eq!(cells[5]["value"], " José \"Pepe\" ");
    assert_eq!(cells[6]["kind"], "text");
    assert_eq!(cells[6]["value"], "00123");
    assert_eq!(cells[7]["value"], "3.5");
    assert_eq!(cells[8]["kind"], "boolean");
    assert_eq!(cells[8]["value"], true);
    assert_eq!(cells[9]["value"], "7");
    assert!(
        cells[9]["formula"]
            .as_str()
            .expect("formula")
            .starts_with('=')
    );
    assert_eq!(cells[10]["kind"], "empty");
    assert_eq!(cells[13]["value"], false);
}

#[test]
fn r_macro_refuses_disabled_undo_before_any_changes() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("directory");
    let profile = dir.path().join("profile");
    std::fs::create_dir_all(profile.join("user")).expect("profile");
    std::fs::write(profile.join("user/registrymodifications.xcu"), r#"<?xml version="1.0" encoding="UTF-8"?>
<oor:items xmlns:oor="http://openoffice.org/2001/registry">
<item oor:path="/org.openoffice.Office.Common/Undo"><prop oor:name="Steps" oor:op="fuse"><value>0</value></prop></item>
</oor:items>"#).expect("disable undo");
    let mut worker = Worker::spawn(&install, &profile).expect("worker");
    let source = dir.path().join("demo.fods");
    std::fs::write(
        &source,
        include_str!("../../../../../tests/fixtures/table/r-macro-demo.fods"),
    )
    .expect("fixture");
    let doc = worker.load(&crate::file_url(&source)).expect("document");
    worker
        .run(&doc, "cell.goto", &["A1:E3"])
        .expect("selection");
    let before = snapshot(&mut worker, &doc);
    let geometry = worker.values(&doc, crate::sheet::GEOMETRY).expect("widths");
    let batch = json!({"snapshot":before,"writes":[{"index":0,"kind":"text","value":"must not appear"}],"bold":true,"autofit":true}).to_string();
    let at = worker.now(&doc).expect("revision");
    assert!(worker.run(&at, "macro.apply", &[&batch]).is_err());
    assert_eq!(reason(&mut worker, &doc)["refused"], "undo_off");
    assert_eq!(snapshot(&mut worker, &doc), before);
    assert_eq!(
        worker.values(&doc, crate::sheet::GEOMETRY).expect("widths"),
        geometry
    );
}

#[test]
fn r_batches_keep_precise_numbers_and_scientific_notation() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_sheet();
    worker
        .run(&doc, "cell.goto", &["A1:D1"])
        .expect("selection");
    let before = snapshot(&mut worker, &doc);
    let numbers = ["1.0000000000000002", "0.12345678901234566", "1e-7", "1e20"];
    let writes: Vec<_> = numbers
        .iter()
        .enumerate()
        .map(|(index, value)| json!({"index":index,"kind":"number","value":value}))
        .collect();
    let batch = json!({"snapshot":before,"writes":writes,"bold":null,"autofit":false}).to_string();
    let at = worker.now(&doc).expect("revision");
    worker
        .run(&at, "macro.apply", &[&batch])
        .expect("numeric writes");
    let after = snapshot(&mut worker, &doc);
    for (index, expected) in numbers.iter().enumerate() {
        let actual = after["cells"][index]["value"]
            .as_str()
            .expect("decimal")
            .parse::<f64>()
            .expect("number");
        assert_eq!(
            actual.to_bits(),
            expected.parse::<f64>().expect("expected").to_bits()
        );
    }
    let batch = json!({"snapshot":after,"writes":[],"bold":true,"autofit":false}).to_string();
    let at = worker.now(&doc).expect("revision");
    worker
        .run(&at, "macro.apply", &[&batch])
        .expect("stable numeric snapshot");
    worker.run(&doc, "edit.undo", &[]).expect("undo format");
    worker.run(&doc, "edit.undo", &[]).expect("undo values");
    worker
        .run(&doc, "cell.goto", &["A1:D1"])
        .expect("selection restored");
    assert_eq!(snapshot(&mut worker, &doc), before);
}

/// The kernel's reason for the last refused batch.
fn reason(worker: &mut Worker, doc: &Doc) -> Value {
    serde_json::from_str(&worker.values(doc, crate::sheet::MACRO_REFUSAL).expect("the reason"))
        .expect("JSON")
}

/// Refused `batch`: the kernel's code for it.
fn refused(worker: &mut Worker, doc: &Doc, batch: &Value) -> Value {
    let at = worker.now(doc).expect("revision");
    assert!(worker.run(&at, "macro.apply", &[&batch.to_string()]).is_err());
    reason(worker, doc)["refused"].clone()
}

fn one_write(before: &Value, kind: &str, value: Value) -> Value {
    json!({"snapshot": before, "writes": [{"index": 0, "kind": kind, "value": value}], "bold": null, "autofit": false})
}

#[test]
fn r_refusals_say_why_once() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = super::sheet::open_book(super::input::FODS);
    worker.run(&doc, "cell.goto", &["A1:A10001"]).expect("a tall selection");
    assert_eq!(snapshot(&mut worker, &doc)["refused"], "cells");
    worker.run(&doc, "cell.goto", &["A1:B2"]).expect("selection");
    let before = snapshot(&mut worker, &doc);
    // Calc holds no empty text; ElyTable sends a clear.
    assert_eq!(refused(&mut worker, &doc, &one_write(&before, "text", json!(""))), "invalid");
    assert_eq!(reason(&mut worker, &doc)["refused"], "", "a reason is read once");
    let moved = json!({"snapshot": before, "writes": [], "bold": true, "autofit": false});
    worker.run(&doc, "cell.goto", &["A1:B3"]).expect("another selection");
    assert_eq!(refused(&mut worker, &doc, &moved), "changed");
    worker.run(&doc, "cell.goto", &["A1:B2"]).expect("selection");
    let locked = crate::sheet::protect_json("", &[]);
    worker.run(&doc, "sheet.protect", &[&locked]).expect("protected");
    assert_eq!(refused(&mut worker, &doc, &one_write(&before, "text", json!("x"))), "protected");
    let open = crate::sheet::unprotect_json("");
    worker.run(&doc, "sheet.protect", &[&open]).expect("unprotected");
    worker.run(&doc, "cell.goto", &["A2:B2"]).expect("a row");
    worker.run(&doc, "cells.merge", &["false"]).expect("merged");
    worker.run(&doc, "cell.goto", &["A1:B2"]).expect("selection");
    let merged = snapshot(&mut worker, &doc);
    assert_eq!(refused(&mut worker, &doc, &one_write(&merged, "text", json!("x"))), "merged");
}

#[test]
fn r_batches_refuse_a_document_read_only_now() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("directory");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("worker");
    let source = dir.path().join("locked.fods");
    std::fs::write(&source, super::input::FODS).expect("fixture");
    let mut mode = std::fs::metadata(&source).expect("mode").permissions();
    mode.set_readonly(true);
    std::fs::set_permissions(&source, mode.clone()).expect("read-only");
    let doc = worker.load(&crate::file_url(&source)).expect("document");
    worker.run(&doc, "cell.goto", &["A1:B2"]).expect("selection");
    let before = snapshot(&mut worker, &doc);
    let code = refused(&mut worker, &doc, &one_write(&before, "text", json!("x")));
    #[allow(clippy::permissions_set_readonly_false)]
    mode.set_readonly(false);
    std::fs::set_permissions(&source, mode).expect("writable again");
    assert_eq!(code, "read_only");
    assert_eq!(snapshot(&mut worker, &doc), before);
}

#[test]
fn r_selection_past_4_mib_is_refused_before_it_is_copied() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = super::sheet::open_book(super::input::FODS);
    // Each cell is legal; together they pass 4 MiB.
    let long = "a".repeat(32767);
    for row in 1..=130 {
        worker.run(&doc, "cell.set", &[&format!("S!C{row}"), &long]).expect("long text");
    }
    worker.run(&doc, "cell.goto", &["C1:C130"]).expect("selection");
    let reply = worker.values(&doc, crate::sheet::MACRO).expect("reply");
    assert!(reply.len() < 1024, "{} bytes came back", reply.len());
    let reply: Value = serde_json::from_str(&reply).expect("JSON");
    assert_eq!(reply["refused"], "too_big");
}

#[test]
fn r_batches_keep_subnormals_and_bold_every_script() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = super::sheet::open_book(super::input::FODS);
    worker.run(&doc, "cell.set", &["S!C1", "שלום"]).expect("complex script");
    worker.run(&doc, "cell.goto", &["A1:C2"]).expect("selection");
    let before = snapshot(&mut worker, &doc);
    let numbers = ["5e-324", "1e-310"];
    let writes: Vec<_> = numbers
        .iter()
        .enumerate()
        .map(|(at, value)| json!({"index": 3 + at, "kind": "number", "value": value}))
        .collect();
    let batch = json!({"snapshot": before, "writes": writes, "bold": true, "autofit": false});
    let at = worker.now(&doc).expect("revision");
    worker.run(&at, "macro.apply", &[&batch.to_string()]).expect("subnormals apply");
    let after = snapshot(&mut worker, &doc);
    for (at, expected) in numbers.iter().enumerate() {
        let value: f64 = after["cells"][3 + at]["value"].as_str().expect("decimal").parse().expect("number");
        assert_eq!(value.to_bits(), expected.parse::<f64>().expect("expected").to_bits());
    }
    let saved = dir.path().join("bold.fods");
    doc.save_as(&mut worker, &crate::file_url(&saved), "fods").expect("saved");
    let xml = std::fs::read_to_string(&saved).expect("XML");
    for weight in ["fo:font-weight=\"bold\"", "style:font-weight-asian=\"bold\"", "style:font-weight-complex=\"bold\""] {
        assert!(xml.contains(weight), "{weight} is missing");
    }
}
