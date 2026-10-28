//! Page edits, undo and saving against the real worker (PDF-02).

use std::path::Path;

use ely_pdf_engine::{
    Doc, Worker,
    sample::{Extras, pdf, pdf_with},
};

fn worker() -> Worker {
    Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker")
}

fn open(worker: &mut Worker, dir: &Path, name: &str, bytes: &[u8]) -> Doc {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("writes");
    worker.open(&path).expect("opens").doc
}

/// Each page's text.
fn texts(worker: &mut Worker, doc: Doc) -> Vec<String> {
    let count = worker.pages(doc).expect("counts");
    (0..count)
        .map(|page| worker.text(doc, page).expect("reads").iter().map(|c| c.c).collect())
        .collect()
}

/// Bookmark titles with their pages.
fn marks(worker: &mut Worker, doc: Doc) -> Vec<(String, Option<usize>)> {
    worker.outline(doc).expect("the outline").into_iter().map(|m| (m.title, m.page)).collect()
}

const LETTER: (f32, f32) = (612.0, 792.0);

fn three() -> Vec<u8> {
    let pages: &[(f32, f32, &[&str])] = &[(612.0, 792.0, &["one"]), (612.0, 792.0, &["two"]), (612.0, 792.0, &["three"])];
    let extras = Extras { links: &[(0, 1), (2, 1), (2, 0)], fields: &[1, 2], labels: &[(0, "r"), (1, "D")], ..Extras::default() };
    pdf_with(pages, &[("One", 0, &[]), ("Two", 1, &[]), ("Three", 2, &[])], &extras)
}

#[test]
fn a_delete_drops_what_pointed_there_and_undoes() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = worker();
    let mut doc = open(&mut worker, dir.path(), "three.pdf", &three());
    let before = worker.history(doc).expect("a mark");
    let after = worker.delete_pages(doc, &[1]).expect("deletes");
    doc = worker.now(doc).expect("the state");
    assert_ne!(before, after);
    assert_eq!(texts(&mut worker, doc), ["one", "three"]);
    let pointing = marks(&mut worker, doc);
    assert_eq!(pointing, [("One".into(), Some(0)), ("Two".into(), None), ("Three".into(), Some(1))]);
    let out = dir.path().join("saved.pdf");
    worker.save(doc, &out).expect("saves");
    let saved = String::from_utf8_lossy(&std::fs::read(&out).expect("reads")).into_owned();
    assert!(saved.contains("(three)") && !saved.contains("(two)"), "the deleted page is gone from the file");
    assert_eq!(saved.matches("/Link").count(), 1, "only the link to page one stays");
    assert!(saved.contains("field1") && !saved.contains("field0"), "the field on the deleted page goes");
    let labels = saved.split("/Nums").nth(1).expect("labels").replace(char::is_whitespace, "");
    assert!(labels.starts_with("[0<</S/r>>1<</S/D>>]"), "{labels}");
    assert!(!dir.path().join("saved.pdf.raw").exists(), "the scratch copy goes");
    assert_eq!(worker.undo(doc, false).expect("undoes"), before);
    doc = worker.now(doc).expect("the state");
    assert_eq!(texts(&mut worker, doc), ["one", "two", "three"]);
    assert_eq!(marks(&mut worker, doc)[1], ("Two".into(), Some(1)));
    assert_eq!(worker.undo(doc, true).expect("redoes"), after);
    doc = worker.now(doc).expect("the state");
    assert_eq!(worker.pages(doc).expect("counts"), 2);
}

#[test]
fn moves_turns_and_crops_are_steps() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = worker();
    let mut doc = open(&mut worker, dir.path(), "three.pdf", &three());
    worker.move_pages(doc, &[2], 0).expect("moves");
    doc = worker.now(doc).expect("the state");
    assert_eq!(texts(&mut worker, doc), ["three", "one", "two"]);
    assert_eq!(marks(&mut worker, doc)[2], ("Three".into(), Some(0)), "bookmarks follow their page");
    worker.move_pages(doc, &[0, 1], 3).expect("moves to the end");
    doc = worker.now(doc).expect("the state");
    assert_eq!(texts(&mut worker, doc), ["two", "three", "one"]);
    worker.rotate_pages(doc, &[1], 90).expect("turns");
    doc = worker.now(doc).expect("the state");
    assert_eq!(worker.bounds(doc, 1, 1).expect("bounds"), [[0.0, 0.0, 792.0, 612.0]]);
    // Turned a quarter, the text's left edge is on top.
    let top = |worker: &mut Worker, doc: Doc| worker.text(doc, 1).expect("reads")[0].rect[1];
    let was = top(&mut worker, doc);
    worker.crop_pages(doc, &[1], [50.0, 0.0, 0.0, 0.0]).expect("crops");
    doc = worker.now(doc).expect("the state");
    assert_eq!(worker.bounds(doc, 1, 1).expect("bounds"), [[0.0, 0.0, 792.0, 562.0]]);
    assert!((top(&mut worker, doc) - (was - 50.0)).abs() < 0.5, "the seen top was cut");
    worker.crop_pages(doc, &[1], [0.0; 4]).expect("uncrops");
    doc = worker.now(doc).expect("the state");
    assert_eq!(worker.bounds(doc, 1, 1).expect("bounds"), [[0.0, 0.0, 792.0, 612.0]]);
    for _ in 0..5 {
        worker.undo(doc, false).expect("undoes");
        doc = worker.now(doc).expect("the state");
    }
    assert_eq!(texts(&mut worker, doc), ["one", "two", "three"]);
    assert_eq!(worker.bounds(doc, 0, 3).expect("bounds"), [[0.0, 0.0, LETTER.0, LETTER.1]; 3]);
    assert!(worker.undo(doc, false).is_err(), "nothing left to undo");
}

#[test]
fn pages_come_in_blank_or_from_a_file() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = worker();
    let mut doc = open(&mut worker, dir.path(), "three.pdf", &three());
    worker.insert_blank(doc, 1, (300.0, 400.0)).expect("inserts a blank page");
    doc = worker.now(doc).expect("the state");
    assert_eq!(worker.bounds(doc, 1, 1).expect("bounds"), [[0.0, 0.0, 300.0, 400.0]]);
    let other = dir.path().join("other.pdf");
    std::fs::write(&other, pdf(&[(612.0, 792.0, &["x"]), (612.0, 792.0, &["y"])], &[])).expect("writes");
    let (count, _) = worker.insert_file(doc, 4, &other).expect("inserts a file");
    doc = worker.now(doc).expect("the state");
    assert_eq!(count, 2);
    assert_eq!(texts(&mut worker, doc), ["one", "", "two", "three", "x", "y"]);
    worker.undo(doc, false).expect("undoes");
    doc = worker.now(doc).expect("the state");
    assert_eq!(worker.pages(doc).expect("counts"), 4);
}

#[test]
fn an_extract_writes_some_pages_and_leaves_the_document() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = worker();
    let mut doc = open(&mut worker, dir.path(), "three.pdf", &three());
    worker.rotate_pages(doc, &[0], 90).expect("turns");
    doc = worker.now(doc).expect("the state");
    let mark = worker.history(doc).expect("a mark");
    let out = dir.path().join("part.pdf");
    worker.extract(doc, &[0, 2], &out).expect("extracts");
    assert_eq!(worker.history(doc).expect("a mark"), mark);
    assert_eq!(texts(&mut worker, doc), ["one", "two", "three"]);
    assert!(worker.undo(doc, true).is_err(), "the extract is not a redo");
    let part = worker.open(&out).expect("opens the part").doc;
    assert_eq!(texts(&mut worker, part), ["one", "three"]);
    assert_eq!(worker.bounds(part, 0, 1).expect("bounds"), [[0.0, 0.0, 792.0, 612.0]], "with its edits");
    worker.undo(doc, false).expect("the turn undoes");
}

#[test]
fn bad_edits_are_refused_and_change_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = worker();
    let pages: &[(f32, f32, &[&str])] = &[(612.0, 792.0, &["big"]), (100.0, 100.0, &["small"])];
    let doc = open(&mut worker, dir.path(), "two.pdf", &pdf(pages, &[]));
    let mark = worker.history(doc).expect("a mark");
    assert!(worker.delete_pages(doc, &[0, 1]).is_err(), "a PDF keeps a page");
    assert!(worker.delete_pages(doc, &[1, 0]).is_err(), "pages in order");
    assert!(worker.rotate_pages(doc, &[0], 45).is_err(), "quarter turns only");
    // Fine for the big page, too much for the small.
    assert!(worker.crop_pages(doc, &[0, 1], [200.0, 0.0, 0.0, 0.0]).is_err());
    assert_eq!(worker.bounds(doc, 0, 2).expect("bounds"), [[0.0, 0.0, 612.0, 792.0], [0.0, 0.0, 100.0, 100.0]], "rolled back");
    assert_eq!(worker.history(doc).expect("a mark"), mark);
    assert!(worker.undo(doc, false).is_err(), "no step was made");
}

#[test]
fn an_extract_after_an_undo_keeps_the_redo() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = worker();
    let mut doc = open(&mut worker, dir.path(), "three.pdf", &three());
    let turned = worker.rotate_pages(doc, &[0], 90).expect("turns");
    doc = worker.now(doc).expect("the state");
    worker.save(doc, &dir.path().join("saved.pdf")).expect("saves");
    worker.undo(doc, false).expect("undoes");
    doc = worker.now(doc).expect("the state");
    worker.extract(doc, &[0], &dir.path().join("part.pdf")).expect("extracts");
    assert_eq!(worker.undo(doc, true).expect("redoes"), turned);
    doc = worker.now(doc).expect("the state");
    assert_eq!(texts(&mut worker, doc), ["one", "two", "three"]);
    assert_eq!(worker.bounds(doc, 0, 1).expect("bounds"), [[0.0, 0.0, 792.0, 612.0]], "the turn is back");
}

#[test]
fn inserted_pages_keep_their_links_and_fields() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = worker();
    let mut doc = open(&mut worker, dir.path(), "three.pdf", &three());
    let other = dir.path().join("other.pdf");
    let pages: &[(f32, f32, &[&str])] = &[(612.0, 792.0, &["x"]), (612.0, 792.0, &["y"])];
    let extras = Extras { links: &[(0, 1)], fields: &[0], ..Extras::default() };
    std::fs::write(&other, pdf_with(pages, &[], &extras)).expect("writes");
    worker.insert_file(doc, 3, &other).expect("inserts");
    doc = worker.now(doc).expect("the state");
    let saved = |worker: &mut Worker, doc: Doc, name: &str| {
        let out = dir.path().join(name);
        worker.save(doc, &out).expect("saves");
        String::from_utf8_lossy(&std::fs::read(&out).expect("reads")).into_owned()
    };
    let merged = saved(&mut worker, doc, "merged.pdf");
    assert_eq!(merged.matches("/Link").count(), 4, "three links of its own, one inserted");
    assert!(merged.contains("(field0 2)") && merged.contains("(value0)"), "a clashing field is renamed, its value kept");
    // The inserted link points at y: deleting y drops it.
    worker.delete_pages(doc, &[4]).expect("deletes y");
    doc = worker.now(doc).expect("the state");
    assert_eq!(saved(&mut worker, doc, "less.pdf").matches("/Link").count(), 3);
}

#[test]
fn a_field_without_its_page_named_still_goes_with_the_page() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = worker();
    let pages: &[(f32, f32, &[&str])] = &[(612.0, 792.0, &["one"]), (612.0, 792.0, &["two"])];
    let extras = Extras { fields: &[1], loose: true, ..Extras::default() };
    let mut doc = open(&mut worker, dir.path(), "loose.pdf", &pdf_with(pages, &[], &extras));
    worker.delete_pages(doc, &[1]).expect("deletes");
    doc = worker.now(doc).expect("the state");
    let out = dir.path().join("out.pdf");
    worker.save(doc, &out).expect("saves");
    let saved = String::from_utf8_lossy(&std::fs::read(&out).expect("reads")).into_owned();
    assert!(!saved.contains("value0"), "the field and its value are gone");
}

#[test]
fn crop_margins_are_points_whatever_the_user_unit() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = worker();
    let extras = Extras { unit: Some(2.0), ..Extras::default() };
    let mut doc = open(&mut worker, dir.path(), "big.pdf", &pdf_with(&[(612.0, 792.0, &["x"])], &[], &extras));
    assert_eq!(worker.bounds(doc, 0, 1).expect("bounds"), [[0.0, 0.0, 1224.0, 1584.0]]);
    worker.crop_pages(doc, &[0], [72.0, 0.0, 0.0, 0.0]).expect("crops");
    doc = worker.now(doc).expect("the state");
    assert_eq!(worker.bounds(doc, 0, 1).expect("bounds"), [[0.0, 0.0, 1224.0, 1512.0]]);
}

/// Runs `step` on its own thread, ten seconds at most.
fn bounded<T: Send + 'static>(step: impl FnOnce() -> T + Send + 'static) -> T {
    let (done, answer) = std::sync::mpsc::channel();
    std::thread::spawn(move || done.send(step()).expect("the test waits"));
    answer.recv_timeout(std::time::Duration::from_secs(10)).expect("done in time")
}

#[test]
fn long_clashing_field_names_still_get_a_number() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let long = "a".repeat(255);
    let page: &[(f32, f32, &[&str])] = &[(612.0, 792.0, &["x"])];
    let extras = Extras { fields: &[0], prefix: Some(&long), ..Extras::default() };
    let (mine, other) = (dir.path().join("mine.pdf"), dir.path().join("other.pdf"));
    std::fs::write(&mine, pdf_with(page, &[], &extras)).expect("writes");
    std::fs::write(&other, pdf_with(page, &[], &extras)).expect("writes");
    let out = dir.path().join("out.pdf");
    let saved = out.clone();
    bounded(move || {
        let mut worker = worker();
        let mut doc = worker.open(&mine).expect("opens").doc;
        worker.insert_file(doc, 1, &other).expect("inserts");
        doc = worker.now(doc).expect("the state");
        worker.save(doc, &saved).expect("saves");
    });
    let saved = String::from_utf8_lossy(&std::fs::read(&out).expect("reads")).into_owned();
    assert!(saved.contains(&format!("({long}0 2)")), "the second is numbered");
}

/// Red pixels of a page at one pixel a point.
fn red(worker: &mut Worker, doc: Doc, page: usize) -> usize {
    let pixels = worker.render(doc, page, (1.0, 0), [0, 0, 612, 792]).expect("renders").bgra;
    pixels.chunks_exact(4).filter(|p| p[2] > 180 && p[1] < 80 && p[0] < 80).count()
}

#[test]
fn inserted_fields_keep_the_look_of_their_form() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = worker();
    let page: &[(f32, f32, &[&str])] = &[(612.0, 792.0, &["x"])];
    let other = dir.path().join("other.pdf");
    std::fs::write(&other, pdf_with(page, &[], &Extras { fields: &[0], look: Some(("Custom", "Custom")), ..Extras::default() })).expect("writes");
    let source = worker.open(&other).expect("opens").doc;
    let shown = red(&mut worker, source, 0);
    assert!(shown > 100, "the source draws its value red: {shown}");
    // Its own form gives the font name another font.
    let mine = Extras { fields: &[0], prefix: Some("mine"), look: Some(("Custom", "Custom")), ..Extras::default() };
    let mut doc = open(&mut worker, dir.path(), "mine.pdf", &pdf_with(page, &[], &mine));
    worker.insert_file(doc, 1, &other).expect("inserts");
    doc = worker.now(doc).expect("the state");
    let out = dir.path().join("out.pdf");
    worker.save(doc, &out).expect("saves");
    let merged = worker.open(&out).expect("opens").doc;
    assert_eq!(red(&mut worker, merged, 1), shown, "the same red, the same size");
    let saved = String::from_utf8_lossy(&std::fs::read(&out).expect("reads")).into_owned();
    assert!(saved.contains("(/Custom_2 24 Tf 1 0 0 rg)"), "the clashing font is renamed where it is used");
}

#[test]
fn an_escaped_font_name_and_the_form_alignment_come_along() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = worker();
    let page: &[(f32, f32, &[&str])] = &[(612.0, 792.0, &["x"])];
    let other = dir.path().join("other.pdf");
    let theirs = Extras { fields: &[0], look: Some(("Custom", "C#75stom")), align: Some(2), ..Extras::default() };
    std::fs::write(&other, pdf_with(page, &[], &theirs)).expect("writes");
    let mine = Extras { fields: &[0], prefix: Some("mine"), look: Some(("Custom", "Custom")), align: Some(0), ..Extras::default() };
    let mut doc = open(&mut worker, dir.path(), "mine.pdf", &pdf_with(page, &[], &mine));
    worker.insert_file(doc, 1, &other).expect("inserts");
    doc = worker.now(doc).expect("the state");
    let out = dir.path().join("out.pdf");
    worker.save(doc, &out).expect("saves");
    let saved = String::from_utf8_lossy(&std::fs::read(&out).expect("reads")).into_owned();
    assert!(saved.contains("(/Custom_2 24 Tf 1 0 0 rg)"), "the escaped name is renamed too");
    assert!(saved.contains("/Q 2"), "right-aligned as in its own form");
}
