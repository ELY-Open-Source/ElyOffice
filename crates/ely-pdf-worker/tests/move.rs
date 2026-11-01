//! Moving text blocks as drawn, against the real worker (PDF-03).

use std::path::Path;

use ely_pdf_engine::{
    Doc, Worker,
    sample::{Extras, one_page, pdf_with},
};

fn worker() -> Worker {
    Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker")
}

/// A Letter page: two lines, a stamp, a picture.
fn opened(dir: &Path) -> (Worker, Doc) {
    let path = dir.join("doc.pdf");
    let extras = Extras { stamp: Some("STAMP"), picture: true, ..Extras::default() };
    std::fs::write(&path, pdf_with(&[(612.0, 792.0, &["Hello world", "Second line"])], &[], &extras)).expect("writes");
    let mut worker = worker();
    let doc = worker.open(&path).expect("opens").doc;
    (worker, doc)
}

/// Rectangles equal to a hundredth of a point.
fn near(got: [f32; 4], want: [f32; 4]) {
    assert!(got.iter().zip(want).all(|(a, b)| (a - b).abs() < 0.01), "{got:?} is not {want:?}");
}

/// A Helvetica page drawing `content`, opened.
fn drawn(dir: &Path, content: &str) -> (Worker, Doc) {
    let font = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string();
    let path = dir.join("drawn.pdf");
    std::fs::write(&path, one_page("", " /Resources << /Font << /F1 5 0 R >> >>", content, &[font])).expect("writes");
    let mut worker = worker();
    let doc = worker.open(&path).expect("opens").doc;
    (worker, doc)
}

/// Pixels on the page that are not white.
fn inked(worker: &mut Worker, doc: Doc) -> usize {
    let tile = worker.render(doc, 0, (1.0, 0), [0, 0, 612, 792]).expect("draws").bgra;
    tile.as_chunks::<4>().0.iter().filter(|p| p[0] < 250 || p[1] < 250 || p[2] < 250).count()
}

#[test]
fn a_moved_block_leaves_forms_and_pictures_and_undoes() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(dir.path());
    let blocks = worker.text_blocks(doc, 0).expect("blocks");
    let index = blocks.iter().position(|b| b.text.starts_with("Hello")).expect("hello");
    let (was, text) = (blocks[index].rect, blocks[index].text.clone());
    let objects = worker.objects(doc, 0).expect("objects");
    worker.move_text(doc, 0, (index, &text), (40.0, 100.0)).expect("moves");
    doc = worker.now(doc).expect("the state");
    let blocks = worker.text_blocks(doc, 0).expect("blocks");
    let moved: Vec<_> = blocks.iter().filter(|b| b.text == text).collect();
    assert_eq!(moved.len(), 1, "the text, once");
    near(moved[0].rect, [was[0] + 40.0, was[1] + 100.0, was[2] + 40.0, was[3] + 100.0]);
    assert!(moved[0].editable, "still the page's own text");
    assert_eq!(blocks.iter().filter(|b| b.text == "STAMP").count(), 1, "the stamp drawn once");
    assert_eq!(worker.objects(doc, 0).expect("objects").len(), objects.len(), "no object added or lost");
    worker.undo(doc, false).expect("undoes");
    doc = worker.now(doc).expect("the state");
    near(worker.text_blocks(doc, 0).expect("blocks")[index].rect, was);
}

#[test]
fn a_moved_block_keeps_fonts_spacing_colour_and_hidden_text() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let content = "BT /F1 18 Tf 72 700 Td 2 Tc (Hello ) Tj 0 Tc 1 0 0 rg /F2 18 Tf (world) Tj 0 g 3 Tr /F1 18 Tf 0 -22 Td (hidden) Tj ET 0 0 1 rg 300 600 50 50 re f";
    let fonts = [
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Times-Bold >>".to_string(),
    ];
    let path = dir.path().join("mixed.pdf");
    std::fs::write(&path, one_page("", " /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >>", content, &fonts)).expect("writes");
    let mut worker = worker();
    let mut doc = worker.open(&path).expect("opens").doc;
    let blocks = worker.text_blocks(doc, 0).expect("blocks");
    let index = blocks.iter().position(|b| b.text.starts_with("Hello")).expect("hello");
    let before = worker.text(doc, 0).expect("reads");
    let red = |worker: &mut Worker, doc, top: i32| {
        let tile = worker.render(doc, 0, (1.0, 0), [0, top, 612, 60]).expect("draws").bgra;
        tile.as_chunks::<4>().0.iter().filter(|p| p[2] > 180 && p[1] < 80 && p[0] < 80).count()
    };
    let dark = |worker: &mut Worker, doc, top: i32| {
        let tile = worker.render(doc, 0, (1.0, 0), [0, top, 612, 24]).expect("draws").bgra;
        tile.as_chunks::<4>().0.iter().filter(|p| p[0] < 128 && p[1] < 128 && p[2] < 128).count()
    };
    assert!(red(&mut worker, doc, 60) > 0, "a red world");
    assert_eq!(dark(&mut worker, doc, 98), 0, "a hidden line");
    worker.move_text(doc, 0, (index, &blocks[index].text), (30.0, 200.0)).expect("moves");
    doc = worker.now(doc).expect("the state");
    let after = worker.text(doc, 0).expect("reads");
    assert_eq!(after.iter().map(|c| c.c).collect::<String>(), before.iter().map(|c| c.c).collect::<String>());
    for (a, b) in before.iter().zip(&after) {
        let shifted = [a.rect[0] + 30.0, a.rect[1] + 200.0, a.rect[2] + 30.0, a.rect[3] + 200.0];
        assert!(b.rect.iter().zip(shifted).all(|(x, y)| (x - y).abs() < 0.01), "{:?}: {:?} is not {shifted:?}", b.c, b.rect);
    }
    assert_eq!(red(&mut worker, doc, 60), 0, "nothing left at the old place");
    assert!(red(&mut worker, doc, 260) > 0, "world stays red");
    assert_eq!(dark(&mut worker, doc, 298), 0, "the hidden line stays hidden");
    let blue = |worker: &mut Worker, doc, at: [i32; 2]| {
        let tile = worker.render(doc, 0, (1.0, 0), [at[0], at[1], 50, 50]).expect("draws").bgra;
        tile.as_chunks::<4>().0.iter().filter(|p| p[0] > 180 && p[1] < 80 && p[2] < 80).count()
    };
    assert_eq!((blue(&mut worker, doc, [300, 142]), blue(&mut worker, doc, [330, 342])), (2500, 0), "the square stays put, once");
}

#[test]
fn clipping_text_is_no_plain_text_and_does_not_move() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = drawn(dir.path(), "q BT /F1 60 Tf 7 Tr 72 700 Td (Hello) Tj ET 1 0 0 rg 0 0 612 792 re f Q");
    let ink = inked(&mut worker, doc);
    assert!(ink > 0, "red shows through the letters");
    let blocks = worker.text_blocks(doc, 0).expect("blocks");
    let index = blocks.iter().position(|b| b.text == "Hello").expect("hello");
    assert!(!blocks[index].editable, "a mask, not text to edit");
    let before = worker.history(doc).expect("a mark");
    worker.move_text(doc, 0, (index, "Hello"), (40.0, 100.0)).expect_err("refused");
    let doc = worker.now(doc).expect("the state");
    assert_eq!((worker.history(doc).expect("a mark"), inked(&mut worker, doc)), (before, ink), "nothing changed");
}

#[test]
fn a_page_clipped_by_a_painting_path_refuses_the_move() {
    for paint in ["f", "S"] {
        let dir = tempfile::tempdir().expect("a temp dir");
        let content = format!("q 0 0 1 rg 0 0 1 RG 50 650 150 80 re W {paint} BT /F1 18 Tf 72 700 Td (Hello) Tj ET Q");
        let (mut worker, doc) = drawn(dir.path(), &content);
        let ink = inked(&mut worker, doc);
        let blocks = worker.text_blocks(doc, 0).expect("blocks");
        let index = blocks.iter().position(|b| b.text == "Hello").expect("hello");
        let before = worker.history(doc).expect("a mark");
        let said = worker.move_text(doc, 0, (index, "Hello"), (40.0, 200.0)).expect_err("refused");
        assert!(said.to_string().contains("clips with paths that also paint"), "{said}");
        let doc = worker.now(doc).expect("the state");
        assert_eq!((worker.history(doc).expect("a mark"), inked(&mut worker, doc)), (before, ink), "W {paint}: nothing changed");
    }
}

#[test]
fn a_plain_clip_moves_with_its_text() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = drawn(dir.path(), "q 50 650 150 80 re W n BT /F1 18 Tf 72 700 Td (Hello) Tj ET Q");
    let ink = inked(&mut worker, doc);
    let index = worker.text_blocks(doc, 0).expect("blocks").iter().position(|b| b.text == "Hello").expect("hello");
    worker.move_text(doc, 0, (index, "Hello"), (40.0, 200.0)).expect("moves");
    let doc = worker.now(doc).expect("the state");
    assert_eq!(inked(&mut worker, doc), ink, "the same letters, moved");
    let blocks = worker.text_blocks(doc, 0).expect("blocks");
    assert!(blocks.iter().any(|b| b.text == "Hello" && b.rect[1] > 250.0), "{blocks:?}");
}

#[test]
fn stroked_text_keeps_its_width_and_dash_through_a_save() {
    for mode in [1, 2] {
        let dir = tempfile::tempdir().expect("a temp dir");
        let content = format!("q 4 w [6 4] 0 d BT /F1 60 Tf {mode} Tr 72 700 Td (Hello) Tj ET Q");
        let (mut worker, doc) = drawn(dir.path(), &content);
        let tile = |worker: &mut Worker, doc, at: [i32; 2]| worker.render(doc, 0, (1.0, 0), [at[0], at[1], 300, 90]).expect("draws").bgra;
        let was = tile(&mut worker, doc, [50, 30]);
        let index = worker.text_blocks(doc, 0).expect("blocks").iter().position(|b| b.text == "Hello").expect("hello");
        worker.move_text(doc, 0, (index, "Hello"), (40.0, 200.0)).expect("moves");
        let doc = worker.now(doc).expect("the state");
        assert!(tile(&mut worker, doc, [90, 230]) == was, "{mode} Tr: the letters as drawn before");
        let out = dir.path().join("moved.pdf");
        worker.save(doc, &out).expect("saves");
        let again = worker.open(&out).expect("opens").doc;
        assert!(tile(&mut worker, again, [90, 230]) == was, "{mode} Tr: the same after a save");
    }
}
