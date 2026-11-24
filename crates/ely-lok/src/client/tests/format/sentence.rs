//! Sentence case starts a sentence after every full stop.

use super::super::input::{open, paragraphs, saved};
use super::super::sheet::open_book;
use super::super::*;

const KEY_RIGHT: i32 = 1027;
const KEY_HOME: i32 = 1028;
const SHIFT: i32 = 0x1000;

/// A text document holding `body`.
fn text(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text"><office:body><office:text>{body}</office:text></office:body></office:document>"#
    )
}

/// A spreadsheet whose A1 holds `cell`, a paragraph each.
fn book(cell: &[&str]) -> String {
    let cell: String = cell.iter().map(|p| format!("<text:p>{p}</text:p>")).collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet"><office:body><office:spreadsheet><table:table table:name="S"><table:table-row><table:table-cell>{cell}</table:table-cell></table:table-row></table:table></office:spreadsheet></office:body></office:document>"#
    )
}

/// Paragraphs, and what sentence case makes of them.
const CASES: [(&str, &str); 9] = [
    ("hELLO wORLD", "Hello world"),
    ("hELLO wORLD. aNOTHER oNE", "Hello world. Another one"),
    ("hELLO wORLD. ANOTHER oNE", "Hello world. Another one"),
    ("hELLO! aNOTHER oNE? tHIRD", "Hello! Another one? Third"),
    ("oNE. tWO. tHREE", "One. Two. Three"),
    ("sEE e.g. fOO", "See e.g. Foo"),
    ("tHE U.S. aRMY", "The u.s. Army"),
    ("中文句子。hELLO wORLD", "中文句子。Hello world"),
    ("dIE sTRAßE. aNOTHER oNE", "Die straße. Another one"),
];

#[test]
fn every_full_stop_starts_a_sentence() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let body: String = CASES.iter().map(|(from, _)| format!("<text:p>{from}</text:p>")).collect();
    let (dir, mut worker, doc) = open(&text(&body));
    worker.run(&doc, "edit.select_all", &[]).expect("select all");
    worker.run(&doc, "format.case_sentence", &[]).expect("sentence case");
    let cased: Vec<&str> = CASES.iter().map(|(_, to)| *to).collect();
    assert_eq!(paragraphs(&saved(&dir, &mut worker, &doc, "cased.fodt")), cased);
    worker.run(&doc, "edit.undo", &[]).expect("one undo");
    let back: Vec<&str> = CASES.iter().map(|(from, _)| *from).collect();
    assert_eq!(paragraphs(&saved(&dir, &mut worker, &doc, "undone.fodt")), back, "one undo restores all");
}

#[test]
fn a_selection_inside_sentences_changes_only_itself() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(&text("<text:p>hELLO wORLD. aNOTHER oNE. tHIRD oNE</text:p>"));
    click(&mut worker, &doc, 150, 150);
    worker.post_key(&doc, 0, KEY_HOME).expect("home");
    for _ in 0..6 {
        worker.post_key(&doc, 0, KEY_RIGHT).expect("right");
    }
    for _ in 0..14 {
        worker.post_key(&doc, 0, SHIFT | KEY_RIGHT).expect("shift right");
    }
    let (deadline, mut seq) = (std::time::Instant::now() + HANG, Events::default().seq);
    let chosen = loop {
        let chosen = doc.selection_text(&mut worker).expect("the text");
        if chosen == "wORLD. aNOTHER" || std::time::Instant::now() > deadline {
            break chosen;
        }
        seq = worker.poll_events(&doc, seq, 250).expect("the poll returns").seq;
    };
    assert_eq!(chosen, "wORLD. aNOTHER", "the keys select inside both sentences");
    worker.run(&doc, "format.case_sentence", &[]).expect("sentence case");
    let xml = saved(&dir, &mut worker, &doc, "part.fodt");
    assert_eq!(paragraphs(&xml), ["hELLO World. Another oNE. tHIRD oNE"]);
}

#[test]
fn a_cell_starts_a_sentence_after_every_full_stop() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    // The first paragraph outruns the last, which once clipped it.
    let (_dir, mut worker, doc) = open_book(&book(&["hELLO wORLD. aNOTHER oNE", "sHORT. oK"]));
    worker.run(&doc, "cell.goto", &["A1"]).expect("A1");
    worker.run(&doc, "format.case_sentence", &[]).expect("sentence case");
    worker.run(&doc, "cell.goto", &["A1"]).expect("A1");
    let cell = doc.selection_text(&mut worker).expect("the text");
    let lines: Vec<&str> = cell.trim_end().trim_matches('"').lines().collect();
    assert_eq!(lines, ["Hello world. Another one", "Short. Ok"]);
}
