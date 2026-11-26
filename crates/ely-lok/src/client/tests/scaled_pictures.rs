//! Cached reduced pictures must never go stale after edits.

use std::io::Cursor;

use base64::Engine as _;
use image::{DynamicImage, ImageFormat, Rgb, RgbImage};

use super::*;

/// Four flat quadrants, much larger than the frame shows it.
fn picture(colors: [[u8; 3]; 4]) -> Vec<u8> {
    let pixels = RgbImage::from_fn(2048, 1536, |x, y| {
        Rgb(colors[usize::from(x >= 700) + 2 * usize::from(y >= 500)])
    });
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(pixels)
        .write_to(&mut out, ImageFormat::Png)
        .expect("encodes");
    out.into_inner()
}

const FIRST: [[u8; 3]; 4] = [[220, 40, 40], [40, 160, 60], [30, 60, 200], [240, 200, 40]];
const SECOND: [[u8; 3]; 4] = [[10, 10, 10], [250, 120, 0], [120, 0, 160], [0, 190, 200]];

fn deck(picture: &[u8]) -> String {
    let data = base64::engine::general_purpose::STANDARD.encode(picture);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:body><office:presentation><draw:page draw:name="One">
 <draw:frame svg:x="1cm" svg:y="1cm" svg:width="12cm" svg:height="9cm"><draw:image><office:binary-data>{data}</office:binary-data></draw:image></draw:frame>
 </draw:page></office:presentation></office:body>
</office:document>"#
    )
}

/// The 2x2 tiles over the slide's top left, joined.
fn shown(worker: &mut Worker, doc: &Doc) -> Vec<u8> {
    let side = crate::TILE_TWIPS;
    let mut all = Vec::new();
    for (x, y) in [(0, 0), (side, 0), (0, side), (side, side)] {
        // Twice: the second paint is the one a cache serves.
        for pass in 0..2 {
            let tile = doc
                .paint_tile(worker, (x, y, side, side), (256, 256))
                .expect("paints");
            if pass == 1 {
                all.extend(tile.pixels);
            }
        }
    }
    all
}

/// Pixels that differ in any channel.
fn differing(a: &[u8], b: &[u8]) -> usize {
    assert_eq!(a.len(), b.len(), "tiles of one size");
    a.chunks(4).zip(b.chunks(4)).filter(|(a, b)| a != b).count()
}

#[test]
fn an_edited_large_picture_paints_as_a_fresh_load_does() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let source = dir.path().join("in.fodp");
    std::fs::write(&source, deck(&picture(FIRST))).expect("writes");
    let other = dir.path().join("other.png");
    std::fs::write(&other, picture(SECOND)).expect("writes");
    let mut worker = Worker::spawn(&install, &dir.path().join("live")).expect("the worker");
    let doc = worker.load(&crate::file_url(&source)).expect("loads");
    let mut before = shown(&mut worker, &doc);
    let replacement = crate::file_url(&other);
    // Ely's picture edits: (name, command, values, selects first).
    let steps: [(&str, &str, Vec<&str>, bool); 9] = [
        ("replace", "image.replace", vec![&replacement], true),
        ("undo the replace", "edit.undo", vec![], false),
        ("redo the replace", "edit.redo", vec![], false),
        // Reduced copies of other sizes, then the first size again.
        ("narrower", "object.width", vec!["6000"], true),
        ("much smaller", "object.height", vec!["2000"], true),
        ("undo the height", "edit.undo", vec![], false),
        ("undo the width", "edit.undo", vec![], false),
        (
            "turned",
            "object.rotate",
            vec!["9000", "7000", "5500"],
            true,
        ),
        ("undo the turn", "edit.undo", vec![], false),
    ];
    let mut report = Vec::new();
    for (n, (name, command, values, select)) in steps.into_iter().enumerate() {
        // Ely refuses numbered commands made before the last change.
        if select {
            let now = worker.now(&doc).expect("the revision");
            worker
                .run(&now, "object.select", &["0", "false"])
                .unwrap_or_else(|error| panic!("select before {name}: {error}"));
        }
        let now = worker.now(&doc).expect("the revision");
        worker
            .run(&now, command, &values)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        // Let the command and its idle work land.
        let mut seq = 0;
        loop {
            let next = worker.poll_events(&doc, seq, 400).expect("polls");
            if next.seq <= seq {
                break;
            }
            seq = next.seq;
        }
        let live = shown(&mut worker, &doc);
        let saved = dir.path().join(format!("step{n}.fodp"));
        doc.save_as(&mut worker, &crate::file_url(&saved), "fodp")
            .expect("saves");
        let mut fresh_worker =
            Worker::spawn(&install, &dir.path().join(format!("fresh{n}"))).expect("the worker");
        let fresh_doc = fresh_worker
            .load(&crate::file_url(&saved))
            .expect("reloads");
        let fresh = shown(&mut fresh_worker, &fresh_doc);
        let (changed, stale) = (differing(&before, &live), differing(&live, &fresh));
        report.push(format!(
            "{name}: {changed} pixels changed, {stale} differ from a fresh load"
        ));
        before = live;
    }
    println!("{}", report.join("\n"));
    let wrong: Vec<&String> = report
        .iter()
        .filter(|line| {
            line.contains(": 0 pixels changed") || !line.ends_with(" 0 differ from a fresh load")
        })
        .collect();
    assert!(wrong.is_empty(), "{report:#?}");
}
