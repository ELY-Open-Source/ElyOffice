//! The Home rows, checked by what the saved file holds.

use super::super::input::{open, paragraphs, saved};
use super::super::*;
use super::state;

const HEAD: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:automatic-styles>
  <style:style style:name="Big" style:family="paragraph"><style:text-properties fo:font-size="28pt"/></style:style>
  <style:style style:name="Twelve" style:family="paragraph"><style:text-properties fo:font-size="12pt"/></style:style>
  <style:style style:name="Bold" style:family="text"><style:text-properties fo:font-weight="bold"/></style:style>
 </office:automatic-styles>
 <office:body><office:text>"##;

const TAIL: &str = "</office:text></office:body></office:document>";

/// The file after `ids` run on all of `body`.
fn after(body: &str, ids: &[&str]) -> String {
    let (dir, mut worker, doc) = open(&format!("{HEAD}{body}{TAIL}"));
    worker.run(&doc, "edit.select_all", &[]).expect("select all");
    for id in ids {
        worker.run(&doc, id, &[]).unwrap_or_else(|error| panic!("{id}: {error}"));
    }
    saved(&dir, &mut worker, &doc, "home.fodt")
}

/// Some `tag` in `xml` has `mark` among its attributes.
fn carries(xml: &str, tag: &str, mark: &str) -> bool {
    xml.split(tag).skip(1).any(|it| it.split('>').next().is_some_and(|props| props.contains(mark)))
}

/// Every `fo:font-size` in points the file declares.
fn sizes(xml: &str) -> Vec<f64> {
    xml.split("fo:font-size=\"")
        .skip(1)
        .filter_map(|rest| rest.split("pt\"").next()?.parse().ok())
        .collect()
}

/// Style names whose text properties make text bold.
fn bold_styles(xml: &str) -> Vec<String> {
    xml.split("<style:style ")
        .skip(1)
        .filter(|style| style.split("</style:style>").next().is_some_and(|it| it.contains("fo:font-weight=\"bold\"")))
        .filter_map(|style| Some(style.split("style:name=\"").nth(1)?.split('"').next()?.to_string()))
        .collect()
}

/// The `<text:p>` holding `words`, markup and all.
fn paragraph_of<'a>(xml: &'a str, words: &str) -> &'a str {
    xml.split("<text:p")
        .find(|p| p.contains(words))
        .and_then(|p| p.split("</text:p>").next())
        .unwrap_or_else(|| panic!("no paragraph holds {words}"))
}

#[test]
fn shading_colors_the_paragraph_not_its_letters() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(&format!("{HEAD}<text:p>shaded</text:p><text:p/>{TAIL}"));
    worker.run(&doc, "edit.select_all", &[]).expect("select all");
    worker.run(&doc, "paragraph.shading", &["16711680"]).expect("shading");
    let shown = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        state(seen, ".uno:BackgroundColor") == Some("16711680")
    });
    assert_eq!(state(&shown, ".uno:BackgroundColor"), Some("16711680"), "the panel reads the shading");
    let red = saved(&dir, &mut worker, &doc, "red.fodt");
    assert!(carries(&red, "<style:paragraph-properties", "#ff0000"), "the paragraph is shaded");
    assert!(!carries(&red, "<style:text-properties", "background-color"), "no letters are highlighted");
    for p in red.split("<text:p").skip(1).take(2) {
        let name = p.split("text:style-name=\"").nth(1).and_then(|it| it.split('"').next());
        let name = name.unwrap_or_else(|| panic!("an unstyled paragraph: {p}"));
        let style = red.split(&format!("style:name=\"{name}\"")).nth(1).expect("its style");
        assert!(style.split("</style:style>").next().is_some_and(|it| it.contains("#ff0000")), "{name} is red, empty or not");
    }
    worker.run(&doc, "edit.select_all", &[]).expect("select all");
    worker.run(&doc, "paragraph.shading", &["-1"]).expect("no shading");
    assert!(!saved(&dir, &mut worker, &doc, "none.fodt").contains("#ff0000"), "the shading is gone");
}

#[test]
fn grow_and_shrink_step_the_size() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let body = r#"<text:p text:style-name="Twelve">sized</text:p>"#;
    let grown = sizes(&after(body, &["format.grow"]));
    assert!(grown.iter().any(|&size| size > 12.0), "grown past 12: {grown:?}");
    let shrunk = sizes(&after(body, &["format.shrink", "format.shrink"]));
    assert!(shrunk.iter().any(|&size| size < 12.0), "shrunk below 12: {shrunk:?}");
}

#[test]
fn clear_drops_direct_formatting() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let body = r#"<text:p><text:span text:style-name="Bold">bold</text:span> plain</text:p>"#;
    assert!(!bold_styles(&after(body, &[])).is_empty(), "the source is bold");
    let cleared = after(body, &["format.clear"]);
    assert_eq!(bold_styles(&cleared), Vec::<String>::new(), "no bold is left");
    assert_eq!(paragraphs(&cleared), ["bold plain"], "the words stay");
}

/// Drags the left button across the middle of `line`.
fn drag(worker: &mut Worker, doc: &Doc, line: Rect) {
    let (middle, end) = (line.1 + line.3 / 2, line.0 + line.2 - 20);
    for (kind, x) in [(0, line.0 + 20), (2, line.0 + line.2 / 2), (2, end), (1, end)] {
        let event = Mouse { kind, x, y: middle, count: 1, buttons: 1, modifiers: 0 };
        worker.post_mouse(doc, event).expect("the drag posts");
    }
}

#[test]
fn the_brush_copies_bold_to_the_dragged_words_once() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let lines = ["<text:span text:style-name=\"Bold\">Source</text:span>", "Target", "Other"];
    let body: String = lines.iter().map(|line| format!(r#"<text:p text:style-name="Big">{line}</text:p>"#)).collect();
    let (dir, mut worker, doc) = open(&format!("{HEAD}{body}{TAIL}"));
    worker.run(&doc, "edit.select_all", &[]).expect("select all");
    let seen = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| seen.selection.len() >= 3);
    let mut rows = seen.selection.clone();
    rows.sort_by_key(|line| line.1);
    assert_eq!(rows.len(), 3, "one selected line a paragraph: {rows:?}");
    let source = rows[0];
    click(&mut worker, &doc, source.0 + source.2 / 2, source.1 + source.3 / 2);
    worker.run(&doc, "format.paint", &[]).expect("the brush");
    let brush = |seen: &Events| state(seen, ".uno:FormatPaintbrush").map(str::to_string);
    let armed = poll_until(&mut worker, &doc, &seen, HANG, |seen| brush(seen).as_deref() == Some("true"));
    assert_eq!(brush(&armed).as_deref(), Some("true"), "the brush is armed");
    drag(&mut worker, &doc, rows[1]);
    let spent = poll_until(&mut worker, &doc, &armed, HANG, |seen| brush(seen).as_deref() == Some("false"));
    assert_eq!(brush(&spent).as_deref(), Some("false"), "one stroke puts the brush away");
    drag(&mut worker, &doc, rows[2]);
    let xml = saved(&dir, &mut worker, &doc, "brushed.fodt");
    let bold = bold_styles(&xml);
    let is_bold = |words: &str| bold.iter().any(|name| paragraph_of(&xml, words).contains(&format!("\"{name}\"")));
    assert!(is_bold("Source") && is_bold("Target"), "the source and the stroke are bold");
    assert!(!is_bold("Other"), "a second drag copies nothing");
}
