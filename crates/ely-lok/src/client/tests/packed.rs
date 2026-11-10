//! A snapshot gives way while its zip is written (R18.8).

use std::{io::ErrorKind, path::Path};

use super::snapshot::stopped_then_whole;
use super::*;

/// A square PNG of noise: packing shrinks it little.
fn noise(side: u32, seed: u64) -> Vec<u8> {
    let mut state = seed;
    let image = image::RgbaImage::from_fn(side, side, |_, _| {
        state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let [red, green, blue, ..] = (state >> 32).to_le_bytes();
        image::Rgba([red, green, blue, 255])
    });
    let mut png = Vec::new();
    image.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).expect("the png encodes");
    png
}

/// The two-slide deck with three pictures of noise.
fn deck(side: u32) -> String {
    use base64::Engine;
    let picture = |seed: u64| {
        let data = base64::engine::general_purpose::STANDARD.encode(noise(side, seed));
        format!(r#"<draw:frame svg:x="{seed}cm" svg:y="8cm" svg:width="4cm" svg:height="4cm"><draw:image><office:binary-data>{data}</office:binary-data></draw:image></draw:frame>"#)
    };
    let (head, tail) = super::show::DECK.split_once("</draw:page>").expect("the first slide's end");
    format!("{head}{}</draw:page>{tail}", (1..4).map(picture).collect::<String>())
}

/// How many input checks a whole pptx snapshot makes.
fn checks(worker: &mut Worker, doc: &Doc, dir: &Path) -> u64 {
    let point = crate::file_url(&dir.join("probe.pptx"));
    let mut reaches = |from: u64| match doc.snapshot_probed(worker, &point, "pptx", from) {
        Ok(()) => false,
        Err(error) if error.kind() == ErrorKind::Interrupted => true,
        Err(error) => panic!("the probed snapshot failed: {error}"),
    };
    // Check 1 precedes the export: always reached.
    let (mut reached, mut missed) = (1, 2);
    while reaches(missed) {
        (reached, missed) = (missed, missed * 2);
    }
    while missed - reached > 1 {
        let middle = reached + (missed - reached) / 2;
        match reaches(middle) {
            true => reached = middle,
            false => missed = middle,
        }
    }
    reached
}

/// Under this, a stream deflates in place: no copy.
const SMALL: u32 = 64;
/// 640 KB of noise: deflated aside, then copied in.
const LARGE: u32 = 400;
/// Each large picture's copy asks at least this often.
const COPIED: u64 = 8;

#[test]
fn a_snapshot_asks_for_input_while_pictures_are_copied_into_its_zip() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn_dev(&install, &dir.path().join("profile")).expect("the worker");
    let mut asked = |side: u32| {
        let file = dir.path().join(format!("deck-{side}.fodp"));
        write(&file, &deck(side));
        let doc = worker.load(&crate::file_url(&file)).expect("the deck loads");
        let asked = checks(&mut worker, &doc, dir.path());
        worker.close_doc(doc).expect("closes");
        asked
    };
    let (small, large) = (asked(SMALL), asked(LARGE));
    eprintln!("checks: {small} with small pictures, {large} with large ones");
    // The XML is the same; only the copies differ.
    assert!(large >= small + 3 * COPIED, "the zip's writing asked {} more times than {small}", large.saturating_sub(small));
    let file = dir.path().join("deck.fodp");
    write(&file, &deck(LARGE));
    // Past every check of the export: inside the copies.
    let (before, again) = stopped_then_whole(&file, ("pptx", "fodp"), small + COPIED, |worker, doc| {
        worker.run(doc, "slide.hide", &[]).expect("hides a slide");
    });
    let held = |text: &str| text.matches("<draw:image").count();
    assert_eq!(held(&before), 3, "the three pictures are on the slide");
    assert_eq!(held(&again), 3, "the next point holds every picture");
}

#[test]
fn a_snapshot_stopped_in_an_odf_package_keeps_the_deck() {
    if install().is_none() {
        return eprintln!("SKIP: no pinned LOK build");
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let file = dir.path().join("deck.fodp");
    write(&file, &deck(LARGE));
    // ODF asks once before the store, then in the package.
    let (before, again) = stopped_then_whole(&file, ("odp", "fodp"), 1 + COPIED, |worker, doc| {
        worker.run(doc, "slide.hide", &[]).expect("hides a slide");
    });
    let held = |text: &str| text.matches("<draw:image").count();
    assert_eq!((held(&before), held(&again)), (3, 3), "the point holds every picture");
}
