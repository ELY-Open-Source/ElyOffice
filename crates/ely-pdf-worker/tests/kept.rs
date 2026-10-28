//! Edits that keep a block's lines where they were (#17).

use std::path::Path;

use ely_pdf_engine::{Doc, Worker, Writing, Wrote, sample::one_page};

fn worker() -> Worker {
    Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker")
}

/// Rectangles equal to a hundredth of a point.
fn near(got: [f32; 4], want: [f32; 4]) {
    assert!(got.iter().zip(want).all(|(a, b)| (a - b).abs() < 0.01), "{got:?} is not {want:?}");
}

/// A rewrite of block `block` as `text` in `font`.
fn rewrite(worker: &mut Worker, doc: Doc, block: usize, text: &str, font: &str) -> Writing {
    let read = worker.text_blocks(doc, 0).expect("blocks")[block].clone();
    Writing {
        block: Some(block),
        expected: read.text,
        text: text.into(),
        font: font.into(),
        size: read.size,
        rgb: read.rgb,
        bx: [read.rect[0], read.rect[1], read.rect[2] - read.rect[0]],
    }
}

/// Right-hand text between two left lines: one block.
const COLUMNS: &str = "BT /F1 9 Tf 72 700 Td (Left line one) Tj ET BT /F1 9 Tf 72 689 Td (Left line two) Tj ET BT /F1 9 Tf 150 694 Td (Pages 2) Tj ET";

/// A Helvetica page drawing `content`.
fn page(dir: &Path, content: &str) -> (Worker, Doc) {
    let font = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string();
    let path = dir.join("columns.pdf");
    std::fs::write(&path, one_page("", " /Resources << /Font << /F1 5 0 R >> >>", content, &[font])).expect("writes");
    let mut worker = worker();
    let doc = worker.open(&path).expect("opens").doc;
    (worker, doc)
}

/// Rewrites block 0 as `text`; the characters past its box.
fn past_the_box(content: &str, text: &str) -> (Vec<char>, String) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = page(dir.path(), content);
    let writing = rewrite(&mut worker, doc, 0, text, "helvetica");
    let right = writing.bx[0] + writing.bx[2];
    assert!(matches!(worker.put_text(doc, 0, &writing, 0.0).expect("writes"), Wrote::Done(_)));
    doc = worker.now(doc).expect("the state");
    let after = worker.text(doc, 0).expect("reads");
    let past = after.iter().filter(|ch| ch.rect[2] > right + 0.5).map(|ch| ch.c).collect();
    (past, after.iter().map(|ch| ch.c).collect())
}

#[test]
fn an_edit_that_keeps_the_lines_keeps_each_where_it_was() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = page(dir.path(), COLUMNS);
    let before = worker.text(doc, 0).expect("reads");
    let blocks = worker.text_blocks(doc, 0).expect("blocks");
    assert_eq!(blocks[0].text, "Left line one\nLeft line two\nPages 2", "one block");
    let writing = rewrite(&mut worker, doc, 0, "Left line one\nLeft line two\nPages 4", "helvetica");
    let Wrote::Done(_) = worker.put_text(doc, 0, &writing, 0.0).expect("writes") else {
        panic!("an unchanged layout needs no preview");
    };
    doc = worker.now(doc).expect("the state");
    let after = worker.text(doc, 0).expect("reads");
    let at = |chars: &[ely_pdf_engine::Char], c: char| chars.iter().find(|ch| ch.c == c).expect("the char").rect;
    near(at(&after, '4'), at(&before, '2'));
    near(at(&after, 'P'), at(&before, 'P'));
    let firsts = |chars: &[ely_pdf_engine::Char]| chars.iter().filter(|ch| ch.c == 'L').map(|ch| ch.rect).collect::<Vec<_>>();
    assert_eq!(firsts(&after).len(), 2);
    for (now, was) in firsts(&after).into_iter().zip(firsts(&before)) {
        near(now, was);
    }
}

#[test]
fn a_longer_line_in_a_kept_layout_still_wraps_in_its_box() {
    // Same lines, size and width; one line now overflows.
    let (past, words) = past_the_box(COLUMNS, "Left line one\nLeft line two\nPages 2 and many more words now");
    assert!(past.is_empty(), "{past:?} past the box");
    assert!(words.contains("now"), "{words}");
}

#[test]
fn an_indented_first_line_wraps_inside_the_box() {
    // First line indented past the second: still one block.
    let content = "BT /F1 9 Tf 560 700 Td (Left line one) Tj ET BT /F1 9 Tf 500 689 Td (Left line two extra text) Tj ET";
    let (past, words) = past_the_box(content, "Left line one and more words now\nLeft line two extra text");
    assert!(past.is_empty(), "{past:?} past the box");
    assert!(words.contains("now") && words.contains("extra"), "{words}");
}
