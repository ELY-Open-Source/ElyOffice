//! SHOW-03: a carried object lands where it is dropped.

use super::analysis::steps;
use super::*;
use crate::show;

/// A text box, a shape with text, an empty shape.
const THREE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
 xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:body><office:presentation>
 <draw:page draw:name="One"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="10cm" svg:height="3cm"><draw:text-box><text:p>Words in a box</text:p></draw:text-box></draw:frame><draw:rect svg:x="2cm" svg:y="8cm" svg:width="6cm" svg:height="4cm"><text:p>Shape text</text:p></draw:rect><draw:rect svg:x="14cm" svg:y="8cm" svg:width="4cm" svg:height="4cm"/></draw:page>
 </office:presentation></office:body>
</office:document>"#;

/// 1/100 mm in twips.
fn twips(mm100: i64) -> i32 {
    (mm100 * 72 / 127) as i32
}

fn read(worker: &mut Worker, doc: &Doc) -> show::Objects {
    show::objects(&worker.values(doc, show::OBJECTS).expect("reads")).expect("parses")
}

#[test]
fn a_move_lands_exactly_in_one_step_rotated_too() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install().expect("LOK"), &dir.path().join("p")).expect("up");
    let path = dir.path().join("three.fodp");
    std::fs::write(&path, THREE).expect("writes");
    let doc = worker.load(&crate::file_url(&path)).expect("loads");
    let read = read(&mut worker, &doc);
    assert!(read.grid.0 > 0 && read.grid.1 > 0, "the snap grid is read: {:?}", read.grid);
    let state = worker.now(&doc).expect("the state");
    worker.run(&state, "object.select", &["2", "false"]).expect("selects");
    worker.run(&doc, "object.rotate", &["3000", &twips(16000).to_string(), &twips(10000).to_string()]).expect("rotates");
    for index in 0..3 {
        let state = worker.now(&doc).expect("the state");
        worker.run(&state, "object.select", &[&index.to_string(), "false"]).expect("selects");
        let before = self::read(&mut worker, &doc);
        let rect = before.objects[index].rect;
        assert_eq!(before.selection.as_ref().map(|sel| (sel.rect.0, sel.rect.1)), Some((rect.0, rect.1)), "the dialog's place is the frame's");
        let to = (twips(rect.0 + 3000), twips(rect.1 - 1500));
        let undo = steps(&mut worker, &doc);
        worker.run(&doc, "object.move", &[&to.0.to_string(), &to.1.to_string()]).expect("moves");
        assert_eq!(steps(&mut worker, &doc), undo + 1, "one undo step");
        let moved = self::read(&mut worker, &doc).objects[index].rect;
        // A twip is 1.76 hundredths of a millimetre.
        assert!((moved.0 - rect.0 - 3000).abs() <= 2 && (moved.1 - rect.1 + 1500).abs() <= 2, "object {index}: {rect:?} -> {moved:?}");
        assert_eq!((moved.2, moved.3), (rect.2, rect.3), "object {index} keeps its size");
    }
}

/// Painting another slide's picture keeps this slide's selection.
#[test]
fn a_thumbnail_of_another_slide_keeps_the_selection() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install().expect("LOK"), &dir.path().join("p")).expect("up");
    let path = dir.path().join("two.fodp");
    std::fs::write(&path, THREE.replace("</draw:page>", r#"</draw:page><draw:page draw:name="Two"/>"#)).expect("writes");
    let doc = worker.load(&crate::file_url(&path)).expect("loads");
    let state = worker.now(&doc).expect("the state");
    worker.run(&state, "object.select", &["2", "false"]).expect("selects");
    let before = read(&mut worker, &doc).selection.map(|sel| sel.rect);
    assert!(before.is_some(), "the shape is selected");
    doc.paint_part(&mut worker, (1, false), (320, 180)).expect("the thumbnail paints");
    assert_eq!(read(&mut worker, &doc).selection.map(|sel| sel.rect), before, "the selection stays");
    // Painted aside, a slide looks as when shown.
    doc.show_sheet(&mut worker, 1).expect("shows slide two");
    assert!(read(&mut worker, &doc).objects.is_empty(), "slide two is shown");
    let aside = doc.paint_part(&mut worker, (0, false), (320, 180)).expect("slide one aside");
    doc.show_sheet(&mut worker, 0).expect("shows slide one");
    let shown = doc.paint_part(&mut worker, (0, false), (320, 180)).expect("slide one shown");
    assert!(aside.iter().any(|&byte| byte != aside[0]), "slide one has ink");
    assert!(aside == shown, "the aside picture differs from the shown one");
}
