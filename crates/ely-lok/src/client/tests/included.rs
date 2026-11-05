//! Linked sections DOCX writes as INCLUDETEXT, or names unkept (0091).

use std::path::{Path, PathBuf};

use super::analysis::skip;
use super::linked::{linking, text};
use super::*;
use crate::links;

#[test]
fn plain_sections_inside_a_linked_section_write_no_word_breaks() {
    if skip() {
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let install = install().expect("the pinned LOK build");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let include = dir.path().join("include.docx");
    let plain = r#"<text:section text:name="P"><text:p>Inner</text:p></text:section>"#;
    for (case, cached) in [("included-plain-open", plain.to_string()), ("included-plain-mid", format!("<text:p>Head</text:p>{plain}<text:p>Tail</text:p>"))] {
        let mut source = dir.path().join(format!("{case}.fodt"));
        linking(&source, &include, &cached);
        // Saved twice: the second save reads the first.
        for round in 0..2 {
            let doc = worker.load(&crate::file_url(&source)).expect("loads");
            source = dir.path().join(format!("{case}-{round}.docx"));
            doc.save_as(&mut worker, &crate::file_url(&source), "docx").expect("saves");
            worker.close_doc(doc).expect("closes");
            super::independent::read_with("ooxml_word.py", &source, case);
        }
        let again = worker.load(&crate::file_url(&source)).expect("reloads");
        let named = links::sources(&worker.values(&again, links::LIST).expect("reads")).expect("parses");
        assert_eq!(named, [crate::file_url(&include)], "{case}: the link survives both saves");
        worker.close_doc(again).expect("closes");
    }
}

/// A body of one table cell holding `inner`; its path.
fn in_cell(dir: &Path, inner: &str) -> (PathBuf, String) {
    let (main, href) = (dir.join("main.fodt"), crate::file_url(&dir.join("include.docx")));
    let cell = |inner: &str| format!("<table:table><table:table-column/><table:table-row><table:table-cell>{inner}</table:table-cell></table:table-row></table:table>");
    let section = format!(r#"<text:section text:name="S"><text:section-source xlink:href="{href}" xlink:type="simple"/>{}</text:section>"#, cell("<text:p>Inner</text:p>"));
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:xlink="http://www.w3.org/1999/xlink"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>Before</text:p>{}<text:p>After</text:p></office:text></office:body>
</office:document>"#,
        cell(&inner.replace("{section}", &section))
    );
    std::fs::write(&main, xml).expect("writes the document");
    (main, href)
}

#[test]
fn a_linked_section_at_a_cell_edge_by_a_table_is_named_unkept() {
    if skip() {
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let install = install().expect("the pinned LOK build");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let (main, href) = in_cell(dir.path(), "{section}");
    let doc = worker.load(&crate::file_url(&main)).expect("loads");
    // Named before any save, so Ely can say so.
    let unkept = links::unkept(&worker.values(&doc, links::LIST).expect("reads")).expect("parses");
    assert_eq!(unkept, [href]);
    let out = dir.path().join("out.docx");
    doc.save_as(&mut worker, &crate::file_url(&out), "docx").expect("saves");
    let again = worker.load(&crate::file_url(&out)).expect("reloads");
    assert_eq!(text(&dir, &mut worker, &again), ["Before", "Inner", "", "After"]);
    let named = links::sources(&worker.values(&again, links::LIST).expect("reads")).expect("parses");
    assert!(named.is_empty(), "the link goes: {named:?}");
}

#[test]
fn a_linked_section_inside_a_cell_away_from_its_edges_stays_includetext() {
    if skip() {
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let install = install().expect("the pinned LOK build");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let (main, href) = in_cell(dir.path(), "<text:p>Lead</text:p>{section}<text:p>Trail</text:p>");
    let doc = worker.load(&crate::file_url(&main)).expect("loads");
    let unkept = links::unkept(&worker.values(&doc, links::LIST).expect("reads")).expect("parses");
    assert!(unkept.is_empty(), "named unkept: {unkept:?}");
    let out = dir.path().join("out.docx");
    doc.save_as(&mut worker, &crate::file_url(&out), "docx").expect("saves");
    let again = worker.load(&crate::file_url(&out)).expect("reloads");
    let named = links::sources(&worker.values(&again, links::LIST).expect("reads")).expect("parses");
    assert_eq!(named, [href], "the link survives in the cell");
}
