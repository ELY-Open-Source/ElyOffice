use super::analysis::{cells, skip, steps};
use super::rules::row;
use super::sheet::open_book;
use super::*;
use crate::sheet::{
    self, Scale,
    csv::{Column, Encoding, Import},
};

/// Two sheets: S1 heads a sum, S2 holds a list.
fn book() -> String {
    let first = [row(&["Name", "Total"]), row(&["Sum"])].concat();
    let second: String = [["Item", "Note"], ["Pen", "a;b"], ["Ink", "2"]]
        .iter()
        .map(|cells| row(cells))
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet>
 <table:table table:name="S1">{first}</table:table>
 <table:table table:name="S2">{second}</table:table>
 </office:spreadsheet></office:body>
</office:document>"#
    )
}

fn import(dir: &Path, worker: &mut Worker, bytes: &[u8], how: &Import) -> Doc {
    import_as("in.csv", dir, worker, bytes, how)
}

fn import_as(name: &str, dir: &Path, worker: &mut Worker, bytes: &[u8], how: &Import) -> Doc {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("writes");
    worker
        .import(&crate::file_url(&path), how)
        .expect("imports")
}

fn flat(worker: &mut Worker, doc: &Doc, dir: &Path) -> String {
    let out = dir.join("out.fods");
    doc.save_as(worker, &crate::file_url(&out), "fods")
        .expect("saves");
    std::fs::read_to_string(out).expect("reads")
}

#[test]
fn csv_imports_keep_codes_and_leave_formulas_as_text() {
    if skip() {
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install().expect("LOK"), &dir.path().join("p")).expect("up");
    let text = "Account,Code,Amount,Formula,When\n00123,12345678901234567890,1.5,=1+1,2026-03-04\n";
    let how = Import {
        separator: ',',
        quote: Some('"'),
        encoding: Encoding::Utf8,
        columns: vec![
            Column::Text,
            Column::Text,
            Column::Standard,
            Column::Standard,
            Column::DateYmd,
        ],
    };
    let doc = import(dir.path(), &mut worker, text.as_bytes(), &how);
    let xml = flat(&mut worker, &doc, dir.path());
    assert!(xml.contains("<text:p>00123</text:p>"), "the zero stays");
    assert!(
        xml.contains("<text:p>12345678901234567890</text:p>"),
        "the long code stays text"
    );
    assert!(
        xml.contains(r#"office:value="1.5""#),
        "the amount is a number"
    );
    assert!(!xml.contains("table:formula"), "no text ran as a formula");
    assert!(
        xml.contains("<text:p>=1+1</text:p>"),
        "the formula text stays"
    );
    assert!(
        xml.contains(r#"office:date-value="2026-03-04""#),
        "the date column reads year first"
    );
}

#[test]
fn csv_imports_take_the_encoding_separator_and_quote() {
    if skip() {
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install().expect("LOK"), &dir.path().join("p")).expect("up");
    // 名称<TAB>值 / '苹果<TAB>x'<TAB>3 in GB18030.
    let mut bytes = vec![0xC3, 0xFB, 0xB3, 0xC6, b'\t', 0xD6, 0xB5, b'\n', b'\''];
    bytes.extend([0xC6, 0xBB, 0xB9, 0xFB]);
    bytes.extend(b"\tx'\t3\n");
    let how = Import {
        separator: '\t',
        quote: Some('\''),
        encoding: Encoding::Gb18030,
        columns: Vec::new(),
    };
    let doc = import(dir.path(), &mut worker, &bytes, &how);
    let xml = flat(&mut worker, &doc, dir.path());
    assert!(xml.contains("<text:p>名称</text:p>") && xml.contains("<text:p>值</text:p>"));
    assert!(
        xml.contains("<text:p>苹果<text:tab/>x</text:p>"),
        "the quote holds the tab"
    );
    assert!(xml.contains(r#"office:value="3""#), "the number reads");
}

#[test]
fn a_txt_file_imports_as_a_sheet() {
    if skip() {
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install().expect("LOK"), &dir.path().join("p")).expect("up");
    let how = Import {
        separator: ',',
        quote: Some('"'),
        encoding: Encoding::Utf8,
        columns: vec![Column::Text],
    };
    let doc = import_as(
        "notes.txt",
        dir.path(),
        &mut worker,
        b"Account,Value\n00123,1.5\n",
        &how,
    );
    assert_eq!(
        doc.sheets(&mut worker).expect("sheets").list.len(),
        1,
        "a sheet, not a page"
    );
    assert_eq!(cells(&mut worker, &doc, "A2"), "00123");
}

#[test]
fn text_exports_write_the_shown_sheet_as_values() {
    if skip() {
        return;
    }
    let (dir, mut worker, doc) = open_book(&book());
    worker.run(&doc, "cell.goto", &["B2"]).expect("B2");
    worker.run(&doc, "cell.enter", &["=1+2"]).expect("the sum");
    let out = dir.path().join("out.csv");
    doc.export_text(&mut worker, &crate::file_url(&out), ';')
        .expect("exports S1");
    // Windows writes CSV lines with CRLF, as RFC 4180 does.
    let first = std::fs::read_to_string(&out).expect("reads").replace("\r\n", "\n");
    assert_eq!(first, "Name;Total\nSum;3\n", "values, not formulas");
    doc.show_sheet(&mut worker, 1).expect("S2");
    doc.export_text(&mut worker, &crate::file_url(&out), ';')
        .expect("exports S2");
    let second = std::fs::read_to_string(&out).expect("reads").replace("\r\n", "\n");
    assert_eq!(second, "Item;Note\nPen;\"a;b\"\nInk;2\n", "only S2");
}

fn layout(worker: &mut Worker, doc: &Doc) -> sheet::PrintLayout {
    let json = worker.values(doc, sheet::LAYOUT).expect("the layout");
    sheet::print_layout(&json).expect("parses")
}

#[test]
fn print_layouts_set_areas_titles_page_and_breaks() {
    if skip() {
        return;
    }
    let (dir, mut worker, doc) = open_book(&book());
    let fresh = layout(&mut worker, &doc);
    assert_eq!(fresh.areas, Vec::<String>::new());
    assert_eq!(fresh.scale, Scale::Percent(100));
    assert_eq!(fresh.sheets, ["S1", "S2"], "one page style");
    let before = steps(&mut worker, &doc);
    worker
        .run(&doc, "print.area", &["$A$1:$B$2", "$1:$1", "$A:$A"])
        .expect("sets the area");
    let page = sheet::page_json(true, Scale::Fit(1, 0));
    worker
        .run(&doc, "page.setup", &[&page])
        .expect("sets the page");
    assert_eq!(steps(&mut worker, &doc), before + 2, "two undo steps");
    let set = layout(&mut worker, &doc);
    assert_eq!(set.areas, ["$A$1:$B$2"]);
    assert_eq!((set.rows, set.columns), (Some((1, 1)), Some((1, 1))));
    assert_eq!((set.landscape, set.scale), (true, Scale::Fit(1, 0)));
    let wide = sheet::page_json(true, Scale::Percent(500));
    assert!(
        worker.run(&doc, "page.setup", &[&wide]).is_err(),
        "500% is refused"
    );
    worker.run(&doc, "cell.goto", &["B3"]).expect("B3");
    worker.run(&doc, "break.row", &[]).expect("a row break");
    assert!(layout(&mut worker, &doc).row_break);
    worker.run(&doc, "break.row_remove", &[]).expect("removed");
    assert!(!layout(&mut worker, &doc).row_break);
    let xlsx = dir.path().join("out.xlsx");
    doc.save_as(&mut worker, &crate::file_url(&xlsx), "xlsx")
        .expect("saves");
    let again = worker.load(&crate::file_url(&xlsx)).expect("reloads");
    let kept = layout(&mut worker, &again);
    assert_eq!(kept.areas, ["$A$1:$B$2"], "the area survives XLSX");
    assert_eq!((kept.rows, kept.columns), (Some((1, 1)), Some((1, 1))));
    assert_eq!((kept.landscape, kept.scale), (true, Scale::Fit(1, 0)));
}

fn protection(worker: &mut Worker, doc: &Doc) -> sheet::Protection {
    let json = worker
        .values(doc, sheet::PROTECTION)
        .expect("the protection");
    sheet::protection(&json).expect("parses")
}

#[test]
fn protection_guards_locked_cells_and_keeps_its_password() {
    if skip() {
        return;
    }
    let (_dir, mut worker, doc) = open_book(&book());
    worker.run(&doc, "cell.goto", &["B1"]).expect("B1");
    worker
        .run(&doc, "cells.lock", &["false"])
        .expect("unlocks B1");
    let allow = vec!["selectLocked".to_string(), "selectUnlocked".to_string()];
    let on = sheet::protect_json("pw", &allow);
    worker.run(&doc, "sheet.protect", &[&on]).expect("protects");
    let read = protection(&mut worker, &doc);
    assert!(read.protected && read.password);
    assert_eq!(read.allow, allow);
    worker.run(&doc, "cell.goto", &["A1"]).expect("A1");
    let _ = worker.run(&doc, "cell.enter", &["x"]);
    assert_eq!(cells(&mut worker, &doc, "A1"), "Name", "A1 is locked");
    worker.run(&doc, "cell.goto", &["B1"]).expect("B1");
    worker
        .run(&doc, "cell.enter", &["y"])
        .expect("B1 takes text");
    assert_eq!(cells(&mut worker, &doc, "B1"), "y");
    let wrong = sheet::unprotect_json("nope");
    assert!(worker.run(&doc, "sheet.protect", &[&wrong]).is_err());
    assert!(protection(&mut worker, &doc).protected, "still protected");
    let right = sheet::unprotect_json("pw");
    worker
        .run(&doc, "sheet.protect", &[&right])
        .expect("unprotects");
    assert!(!protection(&mut worker, &doc).protected);
}

/// A cell's formula and value in an OOXML sheet.
fn ooxml_cell(column: &str, formula: &str, value: &str) -> String {
    format!(r#"<c r="{column}1"><f>{formula}</f><v>{value}</v></c>"#)
}

#[test]
fn excel_caches_recompute_unless_the_function_is_unknown() {
    if skip() {
        return;
    }
    let (dir, mut worker, doc) = open_book(&book());
    let xlsx = dir.path().join("saved.xlsx");
    doc.save_as(&mut worker, &crate::file_url(&xlsx), "xlsx")
        .expect("saves");
    let unpacked = dir.path().join("x");
    let unzip = std::process::Command::new("unzip")
        .args(["-q", "-o"])
        .arg(&xlsx)
        .arg("-d")
        .arg(&unpacked)
        .status()
        .expect("unzip runs");
    assert!(unzip.success());
    // Excel caches: an unknown function's and a stale one.
    let sheet = unpacked.join("xl/worksheets/sheet1.xml");
    let xml = std::fs::read_to_string(&sheet).expect("the sheet");
    let at = xml.find("<sheetData>").expect("sheet data") + "<sheetData>".len();
    let both = ooxml_cell("A", "FUTUREFN(1)", "42") + &ooxml_cell("B", "1+1", "7");
    let row = format!(r#"<row r="1">{both}</row>"#);
    let end = xml[at..].find("</row>").expect("row one") + at + "</row>".len();
    std::fs::write(&sheet, format!("{}{row}{}", &xml[..at], &xml[end..])).expect("writes");
    let excel = dir.path().join("excel.xlsx");
    // Python's zipfile, as ooxml.py uses: no zip on Windows.
    let pack = "import os, sys, zipfile
with zipfile.ZipFile(sys.argv[1], 'w', zipfile.ZIP_DEFLATED) as z:
    for d, _, fs in os.walk('.'):
        for f in fs:
            z.write(os.path.join(d, f))";
    let zip = std::process::Command::new("python3")
        .current_dir(&unpacked)
        .args(["-c", pack])
        .arg(&excel)
        .status()
        .expect("zip runs");
    assert!(zip.success());
    let opened = worker.load(&crate::file_url(&excel)).expect("loads");
    // The file's result, which the banner names (D-02).
    assert_eq!(cells(&mut worker, &opened, "A1"), "42", "the file's result");
    let named = sheet::kept(&worker.values(&opened, sheet::KEPT).expect("reads")).expect("parses");
    assert_eq!(named.len(), 1, "{named:?}");
    assert_eq!(
        cells(&mut worker, &opened, "B1"),
        "2",
        "a stale cache recomputes"
    );
    let again = dir.path().join("again.fods");
    opened
        .save_as(&mut worker, &crate::file_url(&again), "fods")
        .expect("saves");
    let kept = std::fs::read_to_string(again).expect("reads");
    assert!(kept.contains("of:=FUTUREFN(1)"), "the formula stays as written");
}

#[test]
fn linked_values_stay_as_saved_and_are_named() {
    if skip() {
        return;
    }
    let (dir, mut worker, doc) = open_book(&book());
    let source = dir.path().join("source.ods");
    doc.save_as(&mut worker, &crate::file_url(&source), "ods")
        .expect("saves the source");
    let reader = worker
        .load(&crate::file_url(&dir.path().join("book.fods")))
        .expect("a second copy");
    let formula = format!("='{}/[source.ods]S1'!A1", dir.path().display());
    worker.run(&reader, "cell.goto", &["D1"]).expect("D1");
    worker
        .run(&reader, "cell.enter", &[&formula])
        .expect("the link");
    assert_eq!(cells(&mut worker, &reader, "D1"), "Name");
    let saved = dir.path().join("reader.xlsx");
    reader
        .save_as(&mut worker, &crate::file_url(&saved), "xlsx")
        .expect("saves");
    std::fs::remove_file(&source).expect("the source goes");
    let offline = worker.load(&crate::file_url(&saved)).expect("reloads");
    assert_eq!(
        cells(&mut worker, &offline, "D1"),
        "Name",
        "the old value stays"
    );
    let links =
        sheet::links(&worker.values(&offline, sheet::LINKS).expect("reads")).expect("parses");
    assert_eq!(links, [crate::file_url(&source)], "the source is named");
    let plain = sheet::links(&worker.values(&doc, sheet::LINKS).expect("reads")).expect("parses");
    assert!(plain.is_empty());
}
