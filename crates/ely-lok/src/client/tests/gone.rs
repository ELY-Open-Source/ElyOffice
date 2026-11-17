//! A sheet gone from a linked file keeps saved values.

use super::analysis::{cells, skip};
use super::linked::{book, rates, sheets, update};
use super::sheet::open_book;

/// A source whose S1.A1 holds `value`, beside sheet `other`.
fn two_sheets(value: u32, other: &str) -> String {
    let cell = |v: u32| format!(r#"<table:table-row><table:table-cell office:value-type="float" office:value="{v}"/></table:table-row>"#);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S1">{}</table:table><table:table table:name="{other}">{}</table:table>
 </office:spreadsheet></office:body>
</office:document>"#,
        cell(value),
        cell(7)
    )
}

#[test]
fn a_formula_whose_sheet_is_gone_keeps_its_value() {
    if skip() {
        return;
    }
    // A blank operand caches no cell; INDIRECT hides its sheet.
    let indirect = |at: &str| format!("INDIRECT(\"'{{}}/[source.fods]S1'!{at}\")");
    let cases = [
        ("'{}/[source.fods]S1'!A1".to_owned(), "1", true),
        ("'{}/[source.fods]S1'!A2+5".to_owned(), "5", true),
        (indirect("A1"), "1", false),
        (format!("{}+5", indirect("B5")), "5", false),
        // XLSX keeps no INDIRECT result: the link awaits a check.
        (indirect("A1"), "Err:540", true),
        (format!("{}+5", indirect("B5")), "Err:540", true),
    ];
    for (cell, value, kept) in cases {
        let (dir, mut worker, reader) = open_book(&book(0));
        let source = dir.path().join("source.fods");
        rates(&source, 1);
        let formula = format!("={}", cell.replace("{}", &dir.path().display().to_string()));
        worker.run(&reader, "cell.goto", &["H1"]).expect("H1");
        worker.run(&reader, "cell.enter", &[&formula]).expect("the link");
        let saved = dir.path().join("reader.xlsx");
        reader.save_as(&mut worker, &crate::file_url(&saved), "xlsx").expect("saves");
        // S1 renamed: the file reads, the sheet does not.
        let nine = r#"<table:table-cell office:value-type="float" office:value="9"/>"#;
        std::fs::write(&source, sheets("", nine)).expect("renames the sheet");
        let book = if kept { worker.load(&crate::file_url(&saved)).expect("reloads") } else { reader };
        assert_eq!(update(&mut worker, &book), [crate::file_url(&source)], "{cell} reopened {kept}: the source is named");
        assert_eq!(cells(&mut worker, &book, "H1"), value, "{cell} reopened {kept}: the saved value stays");
    }
}

#[test]
fn a_local_indirect_leaves_other_sheets_unread() {
    if skip() {
        return;
    }
    let (dir, mut worker, reader) = open_book(&book(0));
    let source = dir.path().join("source.fods");
    std::fs::write(&source, two_sheets(1, "Unused")).expect("writes the source");
    let link = format!("='{}/[source.fods]S1'!A1", dir.path().display());
    for (cell, formula) in [("H1", link.as_str()), ("H2", "=INDIRECT(\"A1\")")] {
        worker.run(&reader, "cell.goto", &[cell]).expect("the cell");
        worker.run(&reader, "cell.enter", &[formula]).expect("the formula");
    }
    let saved = dir.path().join("reader.xlsx");
    reader.save_as(&mut worker, &crate::file_url(&saved), "xlsx").expect("saves");
    // Only a sheet no formula reads is renamed.
    std::fs::write(&source, two_sheets(2, "Other")).expect("renames the other sheet");
    let book = worker.load(&crate::file_url(&saved)).expect("reloads");
    assert_eq!(update(&mut worker, &book), Vec::<String>::new(), "the update succeeds");
    assert_eq!(cells(&mut worker, &book, "H1"), "2", "the read sheet's new value");
}
