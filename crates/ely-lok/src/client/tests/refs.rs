use super::input::{open, saved};
use super::*;
use crate::review;

fn run(worker: &mut Worker, doc: &Doc, id: &str, values: &[&str]) {
    worker
        .run(doc, id, values)
        .unwrap_or_else(|error| panic!("{id}: {error}"));
}

fn values(worker: &mut Worker, doc: &Doc, command: &str) -> String {
    worker
        .values(doc, command)
        .unwrap_or_else(|error| panic!("{command}: {error}"))
}

fn type_in(worker: &mut Worker, doc: &Doc, text: &str) {
    for key in text.chars() {
        worker.post_key(doc, key as u32, 0).expect("a key");
    }
}

/// Reads until `done`: the kernel lists comments after layout.
fn awaited<T>(mut read: impl FnMut() -> T, done: impl Fn(&T) -> bool) -> T {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        let value = read();
        if done(&value) || std::time::Instant::now() > deadline {
            return value;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[test]
fn references_land() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    run(
        &mut worker,
        &doc,
        "format.style",
        &["Heading 1", "ParagraphStyles"],
    );
    run(&mut worker, &doc, "bookmark.insert", &["Start"]);
    let marks = review::bookmarks(&values(&mut worker, &doc, ".uno:Bookmarks"))
        .expect("the bookmarks parse");
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].name, "Start");
    assert!(marks[0].rect.is_some(), "the mark has a place");
    run(
        &mut worker,
        &doc,
        "reference.insert",
        &["13", "2", "Start", "2"],
    );
    run(
        &mut worker,
        &doc,
        "link.insert",
        &["Ely", "https://example.org/"],
    );
    run(&mut worker, &doc, "note.footnote", &[]);
    type_in(&mut worker, &doc, "Foot");
    click(&mut worker, &doc, 150, 150);
    run(&mut worker, &doc, "note.endnote", &[]);
    click(&mut worker, &doc, 150, 150);
    run(&mut worker, &doc, "index.insert", &["Contents"]);
    run(&mut worker, &doc, "index.update", &[]);
    let text = saved(&dir, &mut worker, &doc, "refs.fodt");
    for mark in [
        "<text:table-of-content ",
        ">Contents<",
        "<text:note text:id=\"ftn",
        "text:note-class=\"endnote\"",
        "<text:bookmark text:name=\"Start\"",
        "<text:bookmark-ref ",
        "xlink:href=\"https://example.org/\"",
    ] {
        assert!(text.contains(mark), "{mark} lands");
    }
}

#[test]
fn formulas_insert_edit_and_take_captions() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    let formula = |worker: &mut Worker| {
        review::formula(&values(worker, &doc, ".uno:Formula")).expect("the formula parses")
    };
    assert_eq!(formula(&mut worker), None, "no formula yet");
    run(&mut worker, &doc, "formula.insert", &["a over b"]);
    assert_eq!(formula(&mut worker).as_deref(), Some("a over b"));
    run(&mut worker, &doc, "formula.insert", &["c over d"]);
    assert_eq!(formula(&mut worker).as_deref(), Some("c over d"));
    run(&mut worker, &doc, "caption.insert", &["Figure", "ratio"]);
    let text = saved(&dir, &mut worker, &doc, "formula.fodt");
    assert_eq!(text.matches("<draw:object").count(), 1, "the edit replaces");
    assert!(text.contains("c over d"), "the edit lands");
    let caption = super::input::paragraphs(&text)
        .into_iter()
        .find(|line| line.contains("Figure"));
    assert!(
        caption
            .as_deref()
            .is_some_and(|line| line.ends_with("Figure 1: ratio")),
        "the caption numbers: {caption:?}"
    );
}

#[test]
fn formula_edits_keep_the_text_and_the_place() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    type_in(&mut worker, &doc, "PREFIX");
    let wide = "\u{1d465} + e\u{301}";
    run(&mut worker, &doc, "formula.insert", &[wide]);
    let formula = |worker: &mut Worker| {
        review::formula(&values(worker, &doc, ".uno:Formula")).expect("the formula parses")
    };
    assert_eq!(formula(&mut worker).as_deref(), Some(wide));
    run(&mut worker, &doc, "image.anchor_page", &[]);
    let placed = spot(&saved(&dir, &mut worker, &doc, "placed.fodt"));
    assert!(placed.is_some(), "the formula sits on the page");
    run(&mut worker, &doc, "formula.insert", &["c over d"]);
    assert_eq!(formula(&mut worker).as_deref(), Some("c over d"));
    let text = saved(&dir, &mut worker, &doc, "edited.fodt");
    assert!(text.contains("PREFIX"), "the text before stays");
    assert_eq!(spot(&text), placed, "the page position stays");
    assert!(
        text.contains("text:anchor-type=\"page\""),
        "the place stays"
    );
    run(&mut worker, &doc, "edit.undo", &[]);
    let text = saved(&dir, &mut worker, &doc, "undone.fodt");
    assert!(text.contains(wide), "one undo restores the formula");
    assert!(text.contains("text:anchor-type=\"page\""), "and its place");
}

/// The page-anchored frame's svg:x and svg:y.
fn spot(text: &str) -> Option<(String, String)> {
    let frame = text.split("<draw:frame ").find(|frame| {
        frame
            .split('>')
            .next()
            .is_some_and(|head| head.contains("text:anchor-type=\"page\""))
    })?;
    let head = frame.split('>').next()?;
    let value = |name: &str| {
        let rest = head.split(&format!("{name}=\"")).nth(1)?;
        rest.split('"').next().map(str::to_string)
    };
    Some((value("svg:x")?, value("svg:y")?))
}

/// An inline formula with its own spacing.
const INLINE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:automatic-styles><style:style style:name="fr1" style:family="graphic"><style:graphic-properties fo:margin-left="0.25in" fo:margin-right="0.5in" style:vertical-pos="top" style:vertical-rel="line"/></style:style></office:automatic-styles>
 <office:body><office:text><text:p>Lead <draw:frame draw:style-name="fr1" draw:name="Formula1" text:anchor-type="as-char" svg:width="0.5in" svg:height="0.4in"><draw:object><math xmlns="http://www.w3.org/1998/Math/MathML" display="block"><semantics><mfrac><mi>a</mi><mi>b</mi></mfrac><annotation encoding="StarMath 5.0">a over b</annotation></semantics></math></draw:object></draw:frame> tail</text:p></office:text></office:body>
</office:document>"#;

#[test]
fn an_inline_formula_keeps_its_spacing_and_alignment() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(INLINE);
    let mut seen = Events::default();
    for x in (1500..4000).step_by(100) {
        click(&mut worker, &doc, x, 1600);
        // Each click answers with a frame or a fresh cursor.
        let before = Events { cursor: None, ..seen };
        seen = poll_until(&mut worker, &doc, &before, HANG, |seen| {
            seen.graphic.is_some() || seen.cursor.is_some()
        });
        if seen.graphic.is_some() {
            break;
        }
    }
    assert!(seen.graphic.is_some(), "a click selects the formula");
    let formula = |worker: &mut Worker| {
        review::formula(&values(worker, &doc, ".uno:Formula")).expect("the formula parses")
    };
    assert_eq!(formula(&mut worker).as_deref(), Some("a over b"));
    run(&mut worker, &doc, "formula.insert", &["c over d"]);
    assert_eq!(formula(&mut worker).as_deref(), Some("c over d"));
    let text = saved(&dir, &mut worker, &doc, "inline.fodt");
    for mark in [
        "fo:margin-left=\"0.25in\"",
        "fo:margin-right=\"0.5in\"",
        "style:vertical-pos=\"top\"",
        "style:vertical-rel=\"line\"",
        "text:anchor-type=\"as-char\"",
    ] {
        assert!(text.contains(mark), "{mark} stays");
    }
    let line = super::input::paragraphs(&text).join("");
    assert!(
        line.starts_with("Lead ") && line.ends_with(" tail"),
        "{line:?}"
    );
    run(&mut worker, &doc, "edit.undo", &[]);
    let text = saved(&dir, &mut worker, &doc, "undone.fodt");
    assert!(text.contains("a over b"), "one undo restores the formula");
}

#[test]
fn comments_and_changes_review() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    worker.set_author("Ada Lovelace").expect("the author");
    click(&mut worker, &doc, 150, 150);
    run(&mut worker, &doc, "comment.insert", &["First note"]);
    let notes = |worker: &mut Worker| {
        review::notes(&values(worker, &doc, ".uno:ViewAnnotations")).expect("the notes parse")
    };
    let first = awaited(|| notes(&mut worker), |notes| !notes.is_empty());
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].text, "First note");
    assert_eq!(first[0].author, "Ada Lovelace", "the set name signs");
    assert!(first[0].anchor.0 > 0, "the note has an anchor");
    let id = first[0].id.clone();
    run(&mut worker, &doc, "comment.reply", &[&id, "An answer"]);
    let thread = awaited(|| notes(&mut worker), |notes| notes.len() == 2);
    assert_eq!(thread.len(), 2);
    assert_eq!(thread[1].parent, id);
    run(&mut worker, &doc, "comment.insert", &["a\n\nNext"]);
    let both = awaited(|| notes(&mut worker), |notes| notes.len() == 3);
    assert_eq!(both[2].text, "a\n\nNext", "empty paragraphs count once");
    run(&mut worker, &doc, "comment.insert", &["a\n\n"]);
    let all = awaited(|| notes(&mut worker), |notes| notes.len() == 4);
    assert_eq!(all[3].text, "a\n\n", "trailing empty paragraphs stay");
    click(&mut worker, &doc, 150, 150);
    run(&mut worker, &doc, "track.toggle", &[]);
    let changes = |worker: &mut Worker| {
        review::changes(&values(worker, &doc, ".uno:AcceptTrackedChanges"))
            .expect("the changes parse")
    };
    type_in(&mut worker, &doc, "Kept ");
    let kept = awaited(|| changes(&mut worker), |list| !list.is_empty());
    assert_eq!(kept.len(), 1, "typing is tracked: {kept:?}");
    assert!(kept[0].anchor.is_some(), "the change has a place");
    run(
        &mut worker,
        &doc,
        "track.accept",
        &[&kept[0].index.to_string()],
    );
    let left = awaited(|| changes(&mut worker), Vec::is_empty);
    assert!(left.is_empty(), "the change is taken");
    type_in(&mut worker, &doc, "Dropped ");
    let dropped = awaited(|| changes(&mut worker), |list| !list.is_empty());
    assert_eq!(dropped.len(), 1);
    run(&mut worker, &doc, "track.reject_all", &[]);
    let left = awaited(|| changes(&mut worker), Vec::is_empty);
    assert!(left.is_empty(), "the change is dropped");
    let text = saved(&dir, &mut worker, &doc, "review.fodt");
    for mark in ["First note", "An answer", "<office:annotation ", "Kept "] {
        assert!(text.contains(mark), "{mark} lands");
    }
    assert!(!text.contains("Dropped"), "the rejected words go");
}
