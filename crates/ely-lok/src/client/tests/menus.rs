use super::input::open;
use super::layout::png;
use super::*;
use crate::menu::{self, Target};

pub(super) const DOC: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>The cat ran.</text:p>
 <table:table table:name="T"><table:table-column table:number-columns-repeated="2"/>
 <table:table-row><table:table-cell><text:p>left</text:p></table:table-cell><table:table-cell><text:p>right</text:p></table:table-cell></table:table-row>
 <table:table-row><table:table-cell><text:p>down</text:p></table:table-cell><table:table-cell><text:p>more</text:p></table:table-cell></table:table-row>
 </table:table><text:p>after</text:p></office:text></office:body>
</office:document>"#;

/// Right-clicks at a point and reads the kernel's menu.
fn right(worker: &mut Worker, doc: &Doc, seen: &mut Events, (x, y): (i32, i32)) -> menu::Offered {
    click(worker, doc, x, y);
    press(worker, doc, seen, (x, y))
}

/// Right-clicks without moving the selection first.
fn press(worker: &mut Worker, doc: &Doc, seen: &mut Events, (x, y): (i32, i32)) -> menu::Offered {
    for kind in [0, 1] {
        let event = Mouse { kind, x, y, count: 1, buttons: 4, modifiers: 0 };
        worker.post_mouse(doc, event).expect("the click posts");
    }
    *seen = poll_until(worker, doc, seen, HANG, |seen| seen.menu.is_some());
    menu::read(&seen.menu.take().expect("a menu")).expect("the menu parses")
}

/// The middle of the first match of `word`.
pub(super) fn find(worker: &mut Worker, doc: &Doc, seen: &mut Events, word: &str) -> (i32, i32) {
    let before = seen.searched;
    worker
        .run(doc, "find.run", &[word, "false", "0", "0", "0", "1"])
        .expect("the search runs");
    *seen = poll_until(worker, doc, seen, HANG, |seen| seen.searched > before);
    let (x, y, _, h) = seen.found[0];
    (x + 60, y + h / 2)
}

#[test]
fn a_right_click_names_its_target() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(DOC);
    click(&mut worker, &doc, 150, 150);
    let mut seen = Events::default();
    let at = find(&mut worker, &doc, &mut seen, "cat");
    let text = right(&mut worker, &doc, &mut seen, at);
    assert_eq!(text.target, Target::Text);
    assert!(text.allows(".uno:InsertAnnotation") && text.allows(".uno:Paste"));
    assert!(!text.allows(".uno:Cut"), "nothing is selected");
    let at = find(&mut worker, &doc, &mut seen, "left");
    let table = right(&mut worker, &doc, &mut seen, at);
    assert_eq!(table.target, Target::Table);
    for uno in [
        ".uno:InsertRowsBefore",
        ".uno:InsertColumnsAfter",
        ".uno:DeleteTable",
        ".uno:SplitCell",
    ] {
        assert!(table.allows(uno), "{uno} runs in a cell");
    }
    assert!(!table.allows(".uno:MergeCells"), "one cell does not merge");
    let at = find(&mut worker, &doc, &mut seen, "after");
    click(&mut worker, &doc, at.0, at.1);
    let blue = png(dir.path(), "blue.png", [0, 0, 255, 255]);
    worker.run(&doc, "image.insert", &[&blue]).expect("the image lands");
    seen = poll_until(&mut worker, &doc, &seen, HANG, |seen| seen.graphic.is_some());
    let (x, y, w, h) = seen.graphic.expect("the image is selected");
    let frame = right(&mut worker, &doc, &mut seen, (x + w / 2, y + h / 2));
    assert_eq!(frame.target, Target::Frame);
    for uno in [".uno:Copy", ".uno:WrapOn", ".uno:BringToFront", ".uno:InsertCaptionDialog"] {
        assert!(frame.allows(uno), "{uno} runs on an image");
    }
    assert!(!frame.allows(".uno:InsertAnnotation"), "no comment on an image");
}

const LOCKED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>open words</text:p>
 <text:section text:name="S" text:protected="true"><text:p>sealed words</text:p></text:section>
 <table:table table:name="T"><table:table-column table:number-columns-repeated="2"/>
 <table:table-row><table:table-cell table:protected="true"><text:p>fixed</text:p></table:table-cell><table:table-cell><text:p>free</text:p></table:table-cell></table:table-row>
 </table:table><text:p>after</text:p></office:text></office:body>
</office:document>"#;

#[test]
fn protected_text_offers_no_link() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open(LOCKED);
    click(&mut worker, &doc, 150, 150);
    let mut seen = Events::default();
    for (word, target, link) in [
        ("open", Target::Text, true),
        ("sealed", Target::Text, false),
        ("free", Target::Table, true),
        ("fixed", Target::Table, false),
    ] {
        let at = find(&mut worker, &doc, &mut seen, word);
        let offered = press(&mut worker, &doc, &mut seen, at);
        assert_eq!(offered.target, target, "{word}");
        assert_eq!(offered.allows(".uno:InsertHyperlink"), link, "{word} selected");
    }
    let at = find(&mut worker, &doc, &mut seen, "open");
    let caret = right(&mut worker, &doc, &mut seen, at);
    assert!(!caret.allows(".uno:InsertHyperlink"), "the kernel links a selection");
}
