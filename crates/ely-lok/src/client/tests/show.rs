use super::analysis::steps;
use super::*;
use crate::show;

/// Two slides on the default master.
pub(crate) const DECK: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
 xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:body><office:presentation>
 <draw:page draw:name="One"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="10cm" svg:height="3cm"><draw:text-box><text:p>First</text:p></draw:text-box></draw:frame></draw:page>
 <draw:page draw:name="Two"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="10cm" svg:height="3cm"><draw:text-box><text:p>Second</text:p></draw:text-box></draw:frame></draw:page>
 </office:presentation></office:body>
</office:document>"#;

pub(super) fn open_deck() -> (tempfile::TempDir, Worker, Doc) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install().expect("LOK"), &dir.path().join("p")).expect("up");
    let path = dir.path().join("deck.fodp");
    std::fs::write(&path, DECK).expect("writes");
    let doc = worker.load(&crate::file_url(&path)).expect("loads");
    (dir, worker, doc)
}

fn slides(worker: &mut Worker, doc: &Doc) -> Vec<(String, bool)> {
    let sheets = doc.sheets(worker).expect("the slides");
    sheets
        .list
        .into_iter()
        .map(|slide| (slide.name, slide.visible))
        .collect()
}

fn design(worker: &mut Worker, doc: &Doc) -> show::Design {
    show::design(&worker.values(doc, show::DESIGN).expect("reads")).expect("parses")
}

/// Runs `id`; the kernel's history gains one step.
fn one_step(worker: &mut Worker, doc: &Doc, id: &str, values: &[&str]) {
    let before = steps(worker, doc);
    worker.run(doc, id, values).expect(id);
    assert_eq!(steps(worker, doc), before + 1, "{id} is one undo step");
}

#[test]
fn slides_insert_copy_hide_move_and_delete() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_deck();
    one_step(&mut worker, &doc, "slide.insert", &[]);
    one_step(&mut worker, &doc, "slide.duplicate", &[]);
    assert_eq!(slides(&mut worker, &doc).len(), 4);
    one_step(&mut worker, &doc, "slide.hide", &[]);
    assert!(!slides(&mut worker, &doc)[2].1, "the copy hides");
    one_step(&mut worker, &doc, "slide.show", &[]);
    one_step(&mut worker, &doc, "slide.last", &[]);
    assert_eq!(slides(&mut worker, &doc)[2].0, "Two", "Two moved up one");
    one_step(&mut worker, &doc, "slide.first", &[]);
    assert_eq!(doc.sheets(&mut worker).expect("slides").current, 0);
    one_step(&mut worker, &doc, "slide.delete", &[]);
    assert_eq!(slides(&mut worker, &doc).len(), 3);
}

#[test]
fn size_layout_master_and_theme_change_in_one_step() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_deck();
    one_step(&mut worker, &doc, "slide.size", &["25400", "19050", "true"]);
    let (w, h) = doc.size(&mut worker).expect("the size");
    assert!(
        (w - 14400).abs() <= 2 && (h - 10800).abs() <= 2,
        "4:3 in twips: {w} x {h}"
    );
    one_step(&mut worker, &doc, "slide.layout", &["1"]);
    assert_eq!(design(&mut worker, &doc).layout, 1);
    worker
        .run(&doc, "master.edit", &[])
        .expect("the master view");
    worker.run(&doc, "master.insert", &[]).expect("a master");
    worker.run(&doc, "master.close", &[]).expect("back");
    let read = design(&mut worker, &doc);
    assert_eq!(read.masters, ["Default", "Default 1"]);
    assert_eq!(read.master, "Default");
    one_step(&mut worker, &doc, "slide.master", &["Default 1"]);
    assert_eq!(design(&mut worker, &doc).master, "Default 1");
    assert!(
        worker.run(&doc, "slide.master", &["Nope"]).is_err(),
        "no such master"
    );
    let theme = read.themes.first().expect("a color set").clone();
    one_step(&mut worker, &doc, "slide.theme", &[&theme]);
    assert_eq!(design(&mut worker, &doc).theme, theme);
    assert!(
        worker.run(&doc, "slide.theme", &["Nope"]).is_err(),
        "no such set"
    );
}

/// Issue #2: more masters than slides, a slide hidden.
#[test]
fn the_master_view_lists_its_masters() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_deck();
    worker.run(&doc, "slide.hide", &[]).expect("hides One");
    worker.run(&doc, "master.edit", &[]).expect("the master view");
    worker.run(&doc, "master.insert", &[]).expect("a master");
    worker.run(&doc, "master.insert", &[]).expect("another");
    let masters = design(&mut worker, &doc).masters;
    assert_eq!(masters.len(), 3, "more masters than slides");
    let shown: Vec<(String, bool)> = masters.into_iter().map(|name| (name, true)).collect();
    assert_eq!(slides(&mut worker, &doc), shown, "masters, all shown");
}

#[test]
fn a_master_edit_shows_on_its_slides() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_deck();
    let side = (160, 90);
    let before = doc
        .paint_part(&mut worker, (1, false), side)
        .expect("paints");
    assert_eq!(before.len(), 160 * 90 * 4);
    worker
        .run(&doc, "master.edit", &[])
        .expect("the master view");
    assert_eq!(slides(&mut worker, &doc).len(), 1, "the parts are masters");
    let master = doc
        .paint_part(&mut worker, (0, true), side)
        .expect("paints");
    assert_ne!(master, before, "a master paints as a master");
    worker
        .paste(&doc, "text/plain;charset=utf-8", b"Brand")
        .expect("pastes onto the master");
    worker
        .run(&doc, "master.close", &[])
        .expect("back to slides");
    assert_eq!(slides(&mut worker, &doc).len(), 2, "slides again");
    let after = doc
        .paint_part(&mut worker, (1, false), side)
        .expect("paints");
    assert_ne!(before, after, "slide two shows the master's change");
    let out = dir.path().join("out.fodp");
    doc.save_as(&mut worker, &crate::file_url(&out), "fodp")
        .expect("saves");
    let xml = std::fs::read_to_string(out).expect("reads");
    let master = &xml[xml.find("<style:master-page").expect("a master")..];
    assert!(master.contains("Brand"), "the master holds the edit");
}

#[test]
fn tiles_keep_their_picture_during_text_edit() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_deck();
    let far = (8000, 6000, 3000, 2000);
    let before = doc.paint_tile(&mut worker, far, (64, 64)).unwrap().pixels;
    click(&mut worker, &doc, 2500, 1500);
    for c in "Hey".chars() {
        worker.post_key(&doc, c as u32, 0).unwrap();
    }
    let during = doc.paint_tile(&mut worker, far, (64, 64)).unwrap().pixels;
    assert!(before == during, "a tile away from the edit changed");
}

fn objects(worker: &mut Worker, doc: &Doc) -> show::Objects {
    show::objects(&worker.values(doc, show::OBJECTS).expect("reads")).expect("parses")
}

#[test]
fn objects_read_select_move_and_guide() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_deck();
    worker
        .run(&doc, "draw.shape", &["rectangle", "true"])
        .expect("draws");
    let read = objects(&mut worker, &doc);
    assert_eq!(read.objects.len(), 2);
    assert_eq!(read.objects[1].kind, "shape");
    assert!(read.objects[1].selected && !read.objects[0].selected);
    let state = worker.now(&doc).expect("the state");
    worker
        .run(&state, "object.select", &["0", "false"])
        .expect("selects");
    let read = objects(&mut worker, &doc);
    assert_eq!(read.selection.expect("one").kind, "text");
    let state = worker.now(&doc).expect("the state");
    worker
        .run(&state, "object.select", &["1", "true"])
        .expect("adds");
    let selection = objects(&mut worker, &doc).selection.expect("two");
    assert!(selection.count == 2 && selection.can_group);
    worker
        .run(&doc, "object.deselect", &["false"])
        .expect("deselects");
    assert!(
        objects(&mut worker, &doc).selection.is_none(),
        "Add alone picks none"
    );
    // Transform arguments are twips: 1701 twips is 3 cm.
    let state = worker.now(&doc).expect("the state");
    worker
        .run(&state, "object.select", &["1", "false"])
        .expect("selects");
    worker.run(&doc, "object.x", &["1701"]).expect("moves");
    let rect = objects(&mut worker, &doc).selection.expect("one").rect;
    assert!((rect.0 - 3000).abs() <= 1, "{rect:?}");
    worker
        .run(&doc, "guide.add", &["0", "0", "4000"])
        .expect("adds");
    let state = worker.now(&doc).expect("the state");
    worker
        .run(&state, "guide.move", &["0", "0", "5000"])
        .expect("moves");
    let guides = objects(&mut worker, &doc).guides;
    assert_eq!(guides.len(), 1);
    assert_eq!(
        (guides[0].kind, guides[0].at.1),
        (show::GuideKind::Horizontal, 5000)
    );
    let state = worker.now(&doc).expect("the state");
    worker
        .run(&state, "guide.delete", &["0", "true"])
        .expect("deletes");
    assert!(objects(&mut worker, &doc).guides.is_empty());
}

#[test]
fn pictures_land_mid_slide_and_replace_in_place() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_deck();
    let red = super::layout::png(dir.path(), "red.png", [255, 0, 0, 255]);
    let blue = super::layout::png(dir.path(), "blue.png", [0, 0, 255, 255]);
    worker.run(&doc, "image.insert", &[&red]).expect("inserts");
    let first = objects(&mut worker, &doc).selection.expect("the picture");
    let (w, h) = doc.size(&mut worker).expect("the slide");
    let mid = |(x, y, width, height): (i64, i64, i64, i64)| (x + width / 2, y + height / 2);
    let (cx, cy) = mid(first.rect);
    let (sw, sh) = (w * 127 / 72, h * 127 / 72);
    assert!(
        (cx - sw / 2).abs() < 20 && (cy - sh / 2).abs() < 20,
        "{first:?}"
    );
    worker
        .run(&doc, "image.replace", &[&blue])
        .expect("replaces");
    let read = objects(&mut worker, &doc);
    assert_eq!(read.objects.len(), 2, "replaced, not added");
    assert_eq!(mid(read.selection.expect("the picture").rect), (cx, cy));
}

/// A sideways-growing box and a small 20 pt box.
const FITS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
 xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
 xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:automatic-styles>
  <style:style style:name="wide" style:family="graphic"><style:graphic-properties draw:auto-grow-width="true" draw:auto-grow-height="false"/></style:style>
  <style:style style:name="small" style:family="graphic"><style:graphic-properties draw:auto-grow-width="false" draw:auto-grow-height="false"/><style:text-properties fo:font-size="20pt"/></style:style>
 </office:automatic-styles>
 <office:body><office:presentation>
 <draw:page draw:name="One">
  <draw:frame draw:style-name="wide" svg:x="1cm" svg:y="1cm" svg:width="10cm" svg:height="1cm"><draw:text-box><text:p>Grows</text:p></draw:text-box></draw:frame>
  <draw:frame draw:style-name="small" svg:x="1cm" svg:y="5cm" svg:width="2cm" svg:height="0.5cm"><draw:text-box><text:p>A long sentence that does not fit in this small box.</text:p></draw:text-box></draw:frame>
 </draw:page>
 </office:presentation></office:body>
</office:document>"#;

#[test]
fn fit_rules_set_both_growths_and_undo_keeps_the_text() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install().expect("LOK"), &dir.path().join("p")).expect("up");
    let path = dir.path().join("fits.fodp");
    std::fs::write(&path, FITS).expect("writes");
    let doc = worker.load(&crate::file_url(&path)).expect("loads");
    let saved = |worker: &mut Worker, name: &str| {
        let out = dir.path().join(name);
        doc.save_as(worker, &crate::file_url(&out), "fodp")
            .expect("saves");
        std::fs::read_to_string(out).expect("reads")
    };
    let state = worker.now(&doc).expect("the state");
    worker
        .run(&state, "object.select", &["0", "false"])
        .expect("selects");
    worker.run(&doc, "text.fit", &["off"]).expect("fits");
    let xml = saved(&mut worker, "off.fodp");
    assert!(
        !xml.contains(r#"draw:auto-grow-width="true""#),
        "off stops sideways growth"
    );
    let state = worker.now(&doc).expect("the state");
    worker
        .run(&state, "object.select", &["1", "false"])
        .expect("selects");
    let before = doc
        .paint_part(&mut worker, (0, false), (320, 180))
        .expect("paints");
    let body = |xml: &str| xml[xml.find("A long sentence").expect("the text") - 80..].to_string();
    let body_before = body(&saved(&mut worker, "before.fodp"));
    worker.run(&doc, "text.fit", &["shrink"]).expect("shrinks");
    worker.run(&doc, "edit.undo", &[]).expect("undoes");
    let after = doc
        .paint_part(&mut worker, (0, false), (320, 180))
        .expect("paints");
    assert!(before == after, "undo brings the picture back");
    assert_eq!(
        body(&saved(&mut worker, "after.fodp")),
        body_before,
        "and the paragraph"
    );
}

#[test]
fn saving_as_pptx_adds_nothing_to_undo() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let path = dir.path().join("t.pptx");
    std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/office/t.pptx"), &path).expect("copies");
    let doc = worker.load(&crate::file_url(&path)).expect("loads");
    worker.run(&doc, "slide.hide", &[]).expect("hides a slide");
    let history = worker.values(&doc, ".uno:Undo").expect("the history");
    // Each layout part borrows a slide while it is written.
    doc.snapshot(&mut worker, &crate::file_url(&dir.path().join("point.pptx")), "pptx").expect("a snapshot");
    doc.save_as(&mut worker, &crate::file_url(&dir.path().join("copy.pptx")), "pptx").expect("a save");
    assert_eq!(worker.values(&doc, ".uno:Undo").expect("the history"), history, "the export left undo steps");
    worker.run(&doc, "edit.undo", &[]).expect("undoes");
    assert_eq!(doc.sheets(&mut worker).expect("the slides").list.len(), 9, "undo took back the hiding alone");
}
