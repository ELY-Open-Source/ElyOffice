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
    assert_eq!(refused(&mut worker, &doc, &one_write(&before, "text", json!("x"))), "locked");
    let bold = json!({"snapshot": before, "writes": [], "bold": true, "autofit": false});
    assert_eq!(refused(&mut worker, &doc, &bold), "protected");
    let open = crate::sheet::unprotect_json("");
    worker.run(&doc, "sheet.protect", &[&open]).expect("unprotected");
    worker.run(&doc, "cell.goto", &["A2:B2"]).expect("a row");
    worker.run(&doc, "cells.merge", &["false"]).expect("merged");
    worker.run(&doc, "cell.goto", &["A1:B2"]).expect("selection");
    let merged = snapshot(&mut worker, &doc);
    assert_eq!(refused(&mut worker, &doc, &one_write(&merged, "text", json!("x"))), "merged");
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
    // Fits as text, not once JSON escapes each backslash.
    let slashes = "\\".repeat(32767);
    for row in 1..=127 {
        worker.run(&doc, "cell.set", &[&format!("S!D{row}"), &slashes]).expect("long text");
    }
    worker.run(&doc, "cell.goto", &["D1:D127"]).expect("selection");
    let reply = worker.values(&doc, crate::sheet::MACRO).expect("reply");
    assert!(reply.len() < 1024, "{} bytes came back", reply.len());
    assert_eq!(serde_json::from_str::<Value>(&reply).expect("JSON")["refused"], "too_big");
}

#[test]
fn a_refusal_mid_batch_undoes_its_writes_and_leaves_no_redo() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = super::sheet::open_book(super::input::FODS);
    worker.run(&doc, "cell.goto", &["B2"]).expect("B2");
    worker.run(&doc, "cells.lock", &["false"]).expect("B2 unlocked");
    worker.run(&doc, "cell.goto", &["A1:B2"]).expect("selection");
    let before = snapshot(&mut worker, &doc);
    let allow = ["selectLocked".to_string(), "selectUnlocked".to_string()];
    worker.run(&doc, "sheet.protect", &[&crate::sheet::protect_json("", &allow)]).expect("protected");
    let undo = worker.values(&doc, ".uno:Undo").expect("undo history");
    // B2 is written, then Calc refuses the locked A1.
    let writes = json!([{"index": 3, "kind": "text", "value": "landed"}, {"index": 0, "kind": "text", "value": "refused"}]);
    let batch = json!({"snapshot": before, "writes": writes, "bold": null, "autofit": false});
    assert_eq!(refused(&mut worker, &doc, &batch), "locked");
    // Calc's Undo selects what it restored.
    worker.run(&doc, "cell.goto", &["A1:B2"]).expect("selection");
    assert_eq!(snapshot(&mut worker, &doc), before, "B2 is undone");
    assert_eq!(worker.values(&doc, ".uno:Undo").expect("undo history"), undo);
    let redo = crate::history::steps(&worker.values(&doc, ".uno:Redo").expect("redo history")).expect("steps");
    assert!(redo.is_empty(), "the refused prefix waits in Redo: {redo:?}");
}

/// Saved “open read-only”: Ely still loads it editable.
const LOAD_READONLY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:config="urn:oasis:names:tc:opendocument:xmlns:config:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:settings><config:config-item-set config:name="ooo:configuration-settings">
  <config:config-item config:name="LoadReadonly" config:type="boolean">true</config:config-item>
 </config:config-item-set></office:settings>
 <office:body><office:spreadsheet><table:table table:name="S"><table:table-row><table:table-cell><text:p>Hello</text:p></table:table-cell></table:table-row></table:table></office:spreadsheet></office:body>
</office:document>"#;

/// Ely reaches no read-only document; R edits as typing does.
#[test]
fn r_batches_edit_what_typing_edits() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("directory");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("worker");
    let marked = dir.path().join("marked.fods");
    std::fs::write(&marked, LOAD_READONLY).expect("fixture");
    let locked = dir.path().join("locked.fods");
    std::fs::write(&locked, super::input::FODS).expect("fixture");
    let mut mode = std::fs::metadata(&locked).expect("mode").permissions();
    mode.set_readonly(true);
    std::fs::set_permissions(&locked, mode.clone()).expect("read-only on disk");
    for source in [&marked, &locked] {
        let doc = worker.load(&crate::file_url(source)).expect("document");
        worker.run(&doc, "cell.set", &["S!B1", "typed"]).expect("typing edits");
        worker.run(&doc, "cell.goto", &["A1:B1"]).expect("selection");
        let before = snapshot(&mut worker, &doc);
        let at = worker.now(&doc).expect("revision");
        let batch = one_write(&before, "text", json!("from R")).to_string();
        worker.run(&at, "macro.apply", &[&batch]).expect("R edits too");
        assert_eq!(snapshot(&mut worker, &doc)["cells"][0]["value"], "from R");
        worker.close_doc(doc).expect("closed");
    }
    #[allow(clippy::permissions_set_readonly_false)]
    mode.set_readonly(false);
    std::fs::set_permissions(&locked, mode).expect("writable again");
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

#[test]
fn a_theme_change_while_r_runs_keeps_the_batch() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = super::sheet::open_book(super::input::FODS);
    worker.run(&doc, "cell.goto", &["A1:B2"]).expect("selection");
    let before = snapshot(&mut worker, &doc);
    let at = worker.now(&doc).expect("revision");
    worker.run(&doc, "view.theme", &["COLOR_SCHEME_ELY_DARK"]).expect("the scheme");
    let batch = one_write(&before, "text", json!("from R")).to_string();
    worker.run(&at, "macro.apply", &[&batch]).expect("still current");
    assert_eq!(snapshot(&mut worker, &doc)["cells"][0]["value"], "from R");
}

/// B1:C1 holds a two-cell array formula.
const ARRAY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S"><table:table-row><table:table-cell><text:p>Head</text:p></table:table-cell><table:table-cell table:formula="of:={1;2}" table:number-matrix-columns-spanned="2" table:number-matrix-rows-spanned="1" office:value-type="float" office:value="1"><text:p>1</text:p></table:table-cell><table:table-cell office:value-type="float" office:value="2"><text:p>2</text:p></table:table-cell></table:table-row></table:table></office:spreadsheet></office:body>
</office:document>"#;

#[test]
fn format_only_batches_cover_part_of_an_array() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = super::sheet::open_book(ARRAY);
    // A1:B1 cuts the array C1 still belongs to.
    worker.run(&doc, "cell.goto", &["A1:B1"]).expect("selection");
    for (bold, autofit) in [(json!(true), false), (json!(null), true)] {
        let before = snapshot(&mut worker, &doc);
        let batch = json!({"snapshot": before, "writes": [], "bold": bold, "autofit": autofit});
        let at = worker.now(&doc).expect("revision");
        worker.run(&at, "macro.apply", &[&batch.to_string()]).expect("formatting is allowed");
    }
    let saved = dir.path().join("array.fods");
    doc.save_as(&mut worker, &crate::file_url(&saved), "fods").expect("saved");
    let xml = std::fs::read_to_string(&saved).expect("XML");
    assert!(xml.contains("fo:font-weight=\"bold\""), "the header is bold");
}

#[test]
fn a_selection_of_exactly_4_mib_reads_and_one_byte_more_does_not() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = super::sheet::open_book(super::input::FODS);
    let long = "a".repeat(32767);
    for row in 1..=127 {
        worker.run(&doc, "cell.set", &[&format!("S!E{row}"), &long]).expect("long text");
    }
    // Each text cell is 30 bytes plus its text.
    let frame = r#"{ "range": "0:4:0:4:127", "rows": 128, "columns": 1, "cells": [ ]}"#.len();
    let cell = |text: usize| 30 + text;
    let last = 4 * 1024 * 1024 - frame - 127 * cell(32767) - cell(0) - 2 * 127;
    for (text, fits) in [(last, true), (last + 1, false)] {
        worker.run(&doc, "cell.set", &["S!E128", &"a".repeat(text)]).expect("last cell");
        worker.run(&doc, "cell.goto", &["E1:E128"]).expect("selection");
        let reply = worker.values(&doc, crate::sheet::MACRO).expect("reply");
        let json: Value = serde_json::from_str(&reply).expect("JSON");
        match fits {
            true => assert_eq!((reply.len(), json["rows"].as_u64()), (4 * 1024 * 1024, Some(128))),
            false => assert_eq!(json["refused"], "too_big", "{} bytes", reply.len()),
        }
    }
}
