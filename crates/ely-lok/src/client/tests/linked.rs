//! LOK-11: links keep saved values until the user asks.

use std::path::Path;

use super::analysis::{cells, skip};
use super::input::{paragraphs, saved};
use super::sheet::open_book;
use super::*;
use crate::links;

/// A one-sheet book whose S1.A1 holds `value`.
fn book(value: u32) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S1"><table:table-row>
 <table:table-cell office:value-type="float" office:value="{value}"/></table:table-row></table:table>
 <table:named-expressions><table:named-range table:name="Rates" table:base-cell-address="$S1.$A$1" table:cell-range-address="$S1.$A$1"/></table:named-expressions>
 </office:spreadsheet></office:body>
</office:document>"#
    )
}

pub(super) fn rates(path: &Path, value: u32) {
    std::fs::write(path, book(value)).expect("writes the source");
}

pub(super) fn update(worker: &mut Worker, doc: &Doc) -> Vec<String> {
    links::failed(&worker.values(doc, links::UPDATE).expect("updates")).expect("parses")
}

#[test]
fn a_book_reads_its_source_only_when_asked() {
    if skip() {
        return;
    }
    let (dir, mut worker, reader) = open_book(&book(0));
    let source = dir.path().join("source.fods");
    rates(&source, 1);
    let formula = format!("='{}/[source.fods]S1'!A1", dir.path().display());
    worker.run(&reader, "cell.goto", &["H1"]).expect("H1");
    worker.run(&reader, "cell.enter", &[&formula]).expect("the link");
    assert_eq!(cells(&mut worker, &reader, "H1"), "1");
    let saved = dir.path().join("reader.xlsx");
    reader.save_as(&mut worker, &crate::file_url(&saved), "xlsx").expect("saves");
    rates(&source, 2);
    // The linking book stays open with its own copy.
    let book = worker.load(&crate::file_url(&saved)).expect("reloads");
    assert_eq!(cells(&mut worker, &book, "H1"), "1", "opening shows the saved value");
    let named = links::sources(&worker.values(&book, links::LIST).expect("reads")).expect("parses");
    assert_eq!(named, [crate::file_url(&source)]);
    assert_eq!(update(&mut worker, &book), Vec::<String>::new());
    assert_eq!(cells(&mut worker, &book, "H1"), "2", "an update reads the source");
    rates(&source, 3);
    assert_eq!(cells(&mut worker, &book, "H1"), "2", "a later save waits for the user");
    assert_eq!(update(&mut worker, &book), Vec::<String>::new());
    assert_eq!(cells(&mut worker, &book, "H1"), "3", "a second update reads it again");
    std::fs::remove_file(&source).expect("the source goes");
    assert_eq!(update(&mut worker, &book), [crate::file_url(&source)], "the failed source is named");
    assert_eq!(cells(&mut worker, &book, "H1"), "3", "a missing source keeps the value");
}

/// A book whose `S` is `sheet` and holds `cells`.
fn sheets(sheet: &str, cells: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:xlink="http://www.w3.org/1999/xlink"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S">{sheet}<table:table-row>{cells}</table:table-row></table:table>
 </office:spreadsheet></office:body>
</office:document>"#
    )
}

/// An unreadable linked source: kept, named, no dialog.
fn missing_stays(book: &str, missing: &str, cell: &str) {
    let (_dir, mut worker, doc) = open_book(book);
    let quiet = std::time::Duration::from_secs(3);
    let before = cells(&mut worker, &doc, "A1");
    let base = poll_until(&mut worker, &doc, &Events::default(), quiet, |_| false);
    let named = links::sources(&worker.values(&doc, links::LIST).expect("reads")).expect("parses");
    assert_eq!(named, [missing], "{book}");
    assert_eq!(update(&mut worker, &doc), [missing], "the missing source is named");
    assert_eq!(cells(&mut worker, &doc, "A1"), before, "the saved value stays");
    assert_eq!(before, cell);
    let after = poll_until(&mut worker, &doc, &base, quiet, |seen| !seen.blocked.is_empty());
    assert!(after.blocked.is_empty(), "no dialog: {:?}", after.blocked);
}

#[test]
fn a_linked_sheet_keeps_its_cells_when_its_file_is_missing() {
    if skip() {
        return;
    }
    let missing = crate::file_url(&std::env::temp_dir().join("ely-nope").join("missing.ods"));
    let sheet = format!(r#"<table:table-source xlink:type="simple" xlink:href="{missing}" table:filter-name="calc8" table:mode="copy-all"/>"#);
    let cell = r#"<table:table-cell office:value-type="float" office:value="5"/>"#;
    missing_stays(&sheets(&sheet, cell), &missing, "5");
}

#[test]
fn a_linked_area_keeps_its_cells_when_its_file_is_missing() {
    if skip() {
        return;
    }
    let missing = crate::file_url(&std::env::temp_dir().join("ely-nope").join("missing.ods"));
    let cell = format!(
        r#"<table:table-cell office:value-type="float" office:value="7"><table:cell-range-source table:name="S1.A1:A1" xlink:type="simple" xlink:href="{missing}" table:filter-name="calc8" table:last-column-spanned="1" table:last-row-spanned="1"/></table:table-cell>"#
    );
    missing_stays(&sheets("", &cell), &missing, "7");
}

/// A fresh flat book whose S1.A1 is 1.
fn present() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let source = dir.path().join("source.fods");
    rates(&source, 1);
    (dir, crate::file_url(&source))
}

#[test]
fn a_linked_sheet_not_in_its_file_keeps_its_cells() {
    if skip() {
        return;
    }
    let (_source, url) = present();
    let sheet = format!(
        r#"<table:table-source xlink:type="simple" xlink:href="{url}" table:filter-name="OpenDocument Spreadsheet Flat XML" table:table-name="Gone" table:mode="copy-all"/>"#
    );
    let cell = r#"<table:table-cell office:value-type="float" office:value="5"/>"#;
    missing_stays(&sheets(&sheet, cell), &url, "5");
}

#[test]
fn a_linked_range_not_in_its_file_keeps_its_cells() {
    if skip() {
        return;
    }
    let (_source, url) = present();
    let cell = format!(
        r#"<table:table-cell office:value-type="float" office:value="7"><table:cell-range-source table:name="NoSuchRange" xlink:type="simple" xlink:href="{url}" table:filter-name="OpenDocument Spreadsheet Flat XML" table:last-column-spanned="1" table:last-row-spanned="1"/></table:table-cell>"#
    );
    missing_stays(&sheets("", &cell), &url, "7");
}

#[test]
fn a_linked_sheet_in_its_file_reads_it() {
    if skip() {
        return;
    }
    let (_source, url) = present();
    let sheet = format!(
        r#"<table:table-source xlink:type="simple" xlink:href="{url}" table:filter-name="OpenDocument Spreadsheet Flat XML" table:table-name="S1" table:mode="copy-all"/>"#
    );
    let cell = r#"<table:table-cell office:value-type="float" office:value="5"/>"#;
    let (_dir, mut worker, doc) = open_book(&sheets(&sheet, cell));
    assert_eq!(update(&mut worker, &doc), Vec::<String>::new());
    assert_eq!(cells(&mut worker, &doc, "A1"), "1", "the sheet reads its file");
}

#[test]
fn a_dde_link_to_no_server_is_named_without_a_dialog() {
    if skip() {
        return;
    }
    let (dir, mut worker, made) = open_book(&book(0));
    worker.run(&made, "cell.goto", &["B1"]).expect("B1");
    worker.run(&made, "cell.enter", &["=DDE(\"elynosuchapp\";\"nofile\";\"A1\")"]).expect("the link");
    let saved = dir.path().join("dde.ods");
    made.save_as(&mut worker, &crate::file_url(&saved), "ods").expect("saves");
    worker.close_doc(made).expect("closes");
    let doc = worker.load(&crate::file_url(&saved)).expect("reloads");
    let quiet = std::time::Duration::from_secs(3);
    let base = poll_until(&mut worker, &doc, &Events::default(), quiet, |_| false);
    let named = links::sources(&worker.values(&doc, links::LIST).expect("reads")).expect("parses");
    assert_eq!(named, ["elynosuchapp|nofile"]);
    assert_eq!(update(&mut worker, &doc), ["elynosuchapp|nofile"], "the failed server is named");
    let after = poll_until(&mut worker, &doc, &base, quiet, |seen| !seen.blocked.is_empty());
    assert!(after.blocked.is_empty(), "no dialog: {:?}", after.blocked);
}

#[test]
fn an_empty_csv_area_keeps_its_cells() {
    if skip() {
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let csv = dir.path().join("empty.csv");
    std::fs::write(&csv, "").expect("the file");
    let url = crate::file_url(&csv);
    let cell = format!(
        r#"<table:table-cell office:value-type="float" office:value="7"><table:cell-range-source table:name="CSV_all" xlink:type="simple" xlink:href="{url}" table:filter-name="Text - txt - csv (StarCalc)" table:last-column-spanned="1" table:last-row-spanned="1"/></table:table-cell>"#
    );
    missing_stays(&sheets("", &cell), &url, "7");
}

/// An area link to `area`, read every second.
fn every_second(source: &Path, area: &str) -> String {
    let url = crate::file_url(source);
    let cell = format!(
        r#"<table:table-cell office:value-type="float" office:value="7"><table:cell-range-source table:name="{area}" xlink:type="simple" xlink:href="{url}" table:filter-name="OpenDocument Spreadsheet Flat XML" table:last-column-spanned="1" table:last-row-spanned="1" table:refresh-delay="PT1S"/></table:table-cell>"#
    );
    sheets("", &cell)
}

/// Lets the kernel's timers run for five seconds.
fn later(worker: &mut Worker, doc: &Doc) {
    poll_until(worker, doc, &Events::default(), std::time::Duration::from_secs(5), |_| false);
}

#[test]
fn a_refresh_interval_waits_for_the_user() {
    if skip() {
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let source = dir.path().join("source.fods");
    rates(&source, 1);
    let (_dir, mut worker, doc) = open_book(&every_second(&source, "Rates"));
    later(&mut worker, &doc);
    assert_eq!(cells(&mut worker, &doc, "A1"), "7", "no timer reads before the user asks");
    assert_eq!(update(&mut worker, &doc), Vec::<String>::new());
    assert_eq!(cells(&mut worker, &doc, "A1"), "1");
    rates(&source, 2);
    later(&mut worker, &doc);
    assert_eq!(cells(&mut worker, &doc, "A1"), "1", "a later save waits for the user");
}

#[test]
fn a_refresh_interval_keeps_an_unread_source_unread() {
    if skip() {
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let source = dir.path().join("source.fods");
    rates(&source, 1);
    let (_dir, mut worker, doc) = open_book(&every_second(&source, "S1.A1"));
    assert_eq!(update(&mut worker, &doc), [crate::file_url(&source)]);
    later(&mut worker, &doc);
    assert_eq!(cells(&mut worker, &doc, "A1"), "7", "no timer overwrites the saved value");
}

/// A document whose section links `include`, showing `cached`.
fn linking(path: &Path, include: &Path, cached: &str) {
    let href = crate::file_url(include);
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:xlink="http://www.w3.org/1999/xlink"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>Before</text:p>
 <text:section text:name="S"><text:section-source xlink:href="{href}" xlink:type="simple"/>
 <text:p>{cached}</text:p></text:section><text:p>After</text:p></office:text></office:body>
</office:document>"#
    );
    std::fs::write(path, xml).expect("writes the document");
}

/// A text document of one paragraph.
fn included(path: &Path, said: &str) {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>{said}</text:p></office:text></office:body>
</office:document>"#
    );
    std::fs::write(path, xml).expect("writes the source");
}

fn text(dir: &tempfile::TempDir, worker: &mut Worker, doc: &Doc) -> Vec<String> {
    paragraphs(&saved(dir, worker, doc, "read.fodt"))
}

#[test]
fn a_linked_section_reads_its_file_only_when_asked() {
    if skip() {
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let install = install().expect("the pinned LOK build");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let (main, include) = (dir.path().join("main.fodt"), dir.path().join("include.fodt"));
    linking(&main, &include, "Cached");
    included(&include, "One");
    let doc = worker.load(&crate::file_url(&main)).expect("loads");
    assert_eq!(text(&dir, &mut worker, &doc), ["Before", "Cached", "After"], "opening shows the saved text");
    let named = links::sources(&worker.values(&doc, links::LIST).expect("reads")).expect("parses");
    assert_eq!(named, [crate::file_url(&include)]);
    assert_eq!(update(&mut worker, &doc), Vec::<String>::new());
    assert_eq!(text(&dir, &mut worker, &doc), ["Before", "One", "After"], "an update reads the file");
    included(&include, "Two");
    assert_eq!(text(&dir, &mut worker, &doc), ["Before", "One", "After"], "a later save waits for the user");
    assert_eq!(update(&mut worker, &doc), Vec::<String>::new());
    assert_eq!(text(&dir, &mut worker, &doc), ["Before", "Two", "After"]);
    std::fs::remove_file(&include).expect("the file goes");
    assert_eq!(update(&mut worker, &doc), [crate::file_url(&include)], "the failed file is named");
    assert_eq!(text(&dir, &mut worker, &doc), ["Before", "Two", "After"], "a missing file keeps the text");
}


#[test]
fn includetext_is_a_linked_section_and_saves_as_one() {
    if skip() {
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let install = install().expect("the pinned LOK build");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office/includetext.docx");
    let (main, flat, include) = (dir.path().join("main.docx"), dir.path().join("inc.fodt"), dir.path().join("include.docx"));
    std::fs::copy(fixture, &main).expect("copies");
    // Word's file, as Writer writes DOCX.
    included(&flat, "Included after");
    let made = worker.load(&crate::file_url(&flat)).expect("loads");
    made.save_as(&mut worker, &crate::file_url(&include), "docx").expect("the source");
    worker.close_doc(made).expect("closes");
    let doc = worker.load(&crate::file_url(&main)).expect("loads");
    let loaded = poll_until(&mut worker, &doc, &Events::default(), HANG, super::docx::modified);
    assert!(!super::docx::modified(&loaded), "the section made at load marked it modified");
    let shown = text(&dir, &mut worker, &doc);
    assert_eq!(shown[..2], ["Before", "Included before"], "opening shows the saved text: {shown:?}");
    assert!(shown.iter().any(|p| p == "Line one") && shown.iter().any(|p| p == "Line two"), "{shown:?}");
    let named = links::sources(&worker.values(&doc, links::LIST).expect("reads")).expect("parses");
    assert_eq!(named, [crate::file_url(&include)]);
    let cached = dir.path().join("cached.docx");
    doc.save_as(&mut worker, &crate::file_url(&cached), "docx").expect("saves");
    super::independent::read_with("ooxml_word.py", &cached, "included-cached");
    assert_eq!(update(&mut worker, &doc), Vec::<String>::new());
    let shown = text(&dir, &mut worker, &doc);
    assert_eq!(shown[..2], ["Before", "Included after"], "the update reads the DOCX: {shown:?}");
    assert_eq!(shown.iter().filter(|p| *p == "Included after").count(), 2, "both sections read: {shown:?}");
    let out = dir.path().join("out.docx");
    doc.save_as(&mut worker, &crate::file_url(&out), "docx").expect("saves");
    super::independent::read_with("ooxml_word.py", &out, "included");
    let again = worker.load(&crate::file_url(&out)).expect("reloads");
    let named = links::sources(&worker.values(&again, links::LIST).expect("reads")).expect("parses");
    assert_eq!(named, [crate::file_url(&include)], "Ely's own file links again");
}

/// The INCLUDETEXT fixture beside an `include.docx` of `body`.
pub(super) fn includes(dir: &tempfile::TempDir, worker: &mut Worker, body: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office/includetext.docx");
    let (main, flat, include) = (dir.path().join("main.docx"), dir.path().join("inc.fodt"), dir.path().join("include.docx"));
    std::fs::copy(fixture, &main).expect("copies");
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text>{body}</office:text></office:body>
</office:document>"#
    );
    std::fs::write(&flat, xml).expect("writes the source");
    let made = worker.load(&crate::file_url(&flat)).expect("loads");
    made.save_as(worker, &crate::file_url(&include), "docx").expect("the source");
    worker.close_doc(made).expect("closes");
    (main, include)
}

#[test]
fn a_source_opening_with_a_table_stays_includetext() {
    if skip() {
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let install = install().expect("the pinned LOK build");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let table = r#"<table:table table:name="T"><table:table-column/><table:table-row><table:table-cell><text:p>Cell</text:p></table:table-cell></table:table-row></table:table><text:p>Tail</text:p>"#;
    let (main, include) = includes(&dir, &mut worker, table);
    let doc = worker.load(&crate::file_url(&main)).expect("loads");
    assert_eq!(update(&mut worker, &doc), Vec::<String>::new());
    let out = dir.path().join("out.docx");
    doc.save_as(&mut worker, &crate::file_url(&out), "docx").expect("saves");
    super::independent::read_with("ooxml_word.py", &out, "included-table");
    let again = worker.load(&crate::file_url(&out)).expect("reloads");
    let named = links::sources(&worker.values(&again, links::LIST).expect("reads")).expect("parses");
    assert_eq!(named, [crate::file_url(&include)], "the links survive the save");
}

#[test]
fn a_linked_section_saved_as_doc_keeps_its_paragraphs() {
    if skip() {
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let install = install().expect("the pinned LOK build");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let (mut source, _) = includes(&dir, &mut worker, "<text:p>Included after</text:p>");
    let mut seen = Vec::new();
    for round in 0..4 {
        let doc = worker.load(&crate::file_url(&source)).expect("loads");
        seen.push(text(&dir, &mut worker, &doc));
        source = dir.path().join(format!("round{round}.doc"));
        doc.save_as(&mut worker, &crate::file_url(&source), "doc").expect("saves");
        worker.close_doc(doc).expect("closes");
    }
    assert!(seen.windows(2).skip(1).all(|pair| pair[0] == pair[1]), "each .doc save reads back the same: {seen:?}");
}
