use super::*;

/// How long an edit may take to show.
const SETTLE: std::time::Duration = std::time::Duration::from_secs(3);
/// How long a quiet document is watched.
const QUIET: std::time::Duration = std::time::Duration::from_millis(600);

#[test]
fn a_tile_paints_real_pixels() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("note.fodt");
    write(&source, FODT);
    let doc = worker
        .load(&crate::file_url(&source))
        .expect("the document loads");
    let tile = doc
        .paint_tile(&mut worker, (0, 0, 960, 960), (64, 64))
        .expect("the tile paints");
    assert_eq!(tile.pixels.len(), 64 * 64 * 4, "a 64x64 BGRA tile");
    let ink = |px: &[u8]| u32::from(px[0]) + u32::from(px[1]) + u32::from(px[2]);
    let dark = tile
        .pixels
        .as_chunks::<4>().0.iter()
        .filter(|px| ink(*px) < 720)
        .count();
    let light = tile
        .pixels
        .as_chunks::<4>().0.iter()
        .filter(|px| ink(*px) > 720)
        .count();
    assert!(
        dark > 0 && light > 0,
        "a text tile has dark and light pixels: {dark}/{light}"
    );
    worker.close_doc(doc).expect("the document closes");
}

#[test]
fn an_edit_bumps_the_generation_and_invalidates() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("note.fodt");
    write(&source, FODT);
    let doc = worker
        .load(&crate::file_url(&source))
        .expect("the document loads");
    let base = poll_until(&mut worker, &doc, &Events::default(), QUIET, |_| false);
    let before = doc
        .paint_tile(&mut worker, (0, 960, 4000, 2000), (64, 64))
        .expect("the tile paints");
    assert_eq!(before.generation, base.generation, "idle: no bump");
    click(&mut worker, &doc, 150, 150);
    worker.post_key(&doc, 0, 1280).expect("Return posts");
    let batch = poll_until(&mut worker, &doc, &base, SETTLE, |seen| {
        seen.generation > base.generation
    });
    assert!(
        batch.generation > before.generation,
        "the edit bumps the generation: {} -to {}",
        before.generation,
        batch.generation
    );
    assert!(
        !batch.all && !batch.rects.is_empty(),
        "the edit invalidates precisely: {batch:?}"
    );
    let after = doc
        .paint_tile(&mut worker, (0, 960, 4000, 2000), (64, 64))
        .expect("the tile repaints");
    assert!(
        after.generation >= batch.generation,
        "a fresh tile matches the new generation"
    );
    assert_ne!(before.pixels, after.pixels, "the edit changes the picture");
    worker.close_doc(doc).expect("the document closes");
}

#[test]
fn uno_commands_invalidate_and_repaint() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("note.fodt");
    write(&source, FODT);
    let doc = worker
        .load(&crate::file_url(&source))
        .expect("the document loads");
    let base = poll_until(&mut worker, &doc, &Events::default(), QUIET, |_| false);
    let before = doc
        .paint_tile(&mut worker, (0, 960, 4000, 2000), (64, 64))
        .expect("the tile paints");
    click(&mut worker, &doc, 150, 150);
    worker
        .run(&doc, "edit.select_all", &[])
        .expect("select all");
    worker.cut(&doc).expect("the cut runs");
    let batch = poll_until(&mut worker, &doc, &base, SETTLE, |seen| {
        seen.generation > base.generation
    });
    assert!(
        batch.generation > base.generation,
        "a deleting command bumps the generation"
    );
    let after = doc
        .paint_tile(&mut worker, (0, 960, 4000, 2000), (64, 64))
        .expect("the tile repaints");
    assert_ne!(
        before.pixels, after.pixels,
        "the delete changes the picture"
    );
    worker.close_doc(doc).expect("the document closes");
}

const RED_HTML: &str =
    "<html><body><p><span style=\"color:#FF0000\">red red red red</span></p></body></html>";

#[test]
fn a_red_span_paints_red_bytes() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("red.html");
    write(&source, RED_HTML);
    let doc = worker
        .load(&crate::file_url(&source))
        .expect("the document loads");
    let tile = doc
        .paint_tile(&mut worker, (0, 0, 2000, 1000), (64, 64))
        .expect("the tile paints");
    let red = tile
        .pixels
        .as_chunks::<4>().0.iter()
        .filter(|px| px[2] > 200 && px[0] < 80 && px[1] < 80)
        .count();
    assert!(red > 0, "BGRA red bytes paint red: {red}");
    worker.close_doc(doc).expect("the document closes");
}

#[test]
fn a_key_press_edits_the_document() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("note.fodt");
    write(&source, FODT);
    let doc = worker
        .load(&crate::file_url(&source))
        .expect("the document loads");
    click(&mut worker, &doc, 150, 150);
    worker.post_key(&doc, 0, 1280).expect("Return posts");
    let out = dir.path().join("after.fodt");
    doc.save_as(&mut worker, &crate::file_url(&out), "fodt")
        .expect("the save works");
    let before = std::fs::read_to_string(&source).expect("the source reads");
    let after = std::fs::read_to_string(&out).expect("the save reads");
    let count = |text: &str| text.matches("<text:p").count();
    assert_eq!(count(&before) + 1, count(&after), "Return adds a paragraph");
    worker.close_doc(doc).expect("the document closes");
}

#[test]
fn selection_only_requests_keep_the_generation() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("note.fodt");
    write(&source, FODT);
    let doc = worker
        .load(&crate::file_url(&source))
        .expect("the document loads");
    let base = poll_until(&mut worker, &doc, &Events::default(), QUIET, |_| false);
    click(&mut worker, &doc, 150, 150);
    worker.post_key(&doc, 0, 1027).expect("Right posts");
    let quiet = poll_until(&mut worker, &doc, &base, QUIET, |_| false);
    assert_eq!(
        quiet.generation, base.generation,
        "selection moves stay quiet"
    );
    worker.close_doc(doc).expect("the document closes");
}

#[test]
fn bad_tile_geometry_is_an_error_and_the_worker_lives() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("note.fodt");
    write(&source, FODT);
    let doc = worker
        .load(&crate::file_url(&source))
        .expect("the document loads");
    for (x, y, w, h, pw, ph) in [
        (0, 0, 0, 960, 64, 64),
        (0, 0, -15, 960, 64, 64),
        (0, 0, 960, 960, 0, 64),
        (0, 0, 960, 960, 100_000, 100_000),
    ] {
        assert!(
            doc.paint_tile(&mut worker, (x, y, w, h), (pw, ph)).is_err(),
            "geometry {x},{y},{w},{h},{pw},{ph} is refused"
        );
    }
    let (width, height) = doc.size(&mut worker).expect("the worker still answers");
    assert!(
        width > 0 && height > 0,
        "the document outlives bad requests"
    );
    worker.close_doc(doc).expect("the document closes");
}

#[test]
fn a_long_wait_is_clamped_and_requests_flow() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("note.fodt");
    write(&source, FODT);
    let doc = worker
        .load(&crate::file_url(&source))
        .expect("the document loads");
    let started = std::time::Instant::now();
    let batch = worker
        .poll_events(&doc, u64::MAX, u64::MAX)
        .expect("the long wait returns");
    assert_eq!(batch.generation, 0, "nothing to drain");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(4),
        "the wait is clamped"
    );
    worker.post_key(&doc, 0, 1027).expect("Right posts");
    worker.close_doc(doc).expect("the document closes");
    worker.shutdown().expect("the worker shuts down");
}

#[test]
fn polling_times_out_with_the_same_generation() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("note.fodt");
    write(&source, FODT);
    let doc = worker
        .load(&crate::file_url(&source))
        .expect("the document loads");
    let base = poll_until(&mut worker, &doc, &Events::default(), QUIET, |_| false);
    let again = poll_until(&mut worker, &doc, &base, QUIET, |_| false);
    assert_eq!(again.generation, base.generation, "no edit, no bump");
    assert!(again.rects.is_empty() && !again.all, "nothing invalidated");
    worker.close_doc(doc).expect("the document closes");
}

#[test]
fn uno_commands_reach_their_own_document() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let (first, second) = (dir.path().join("a.fodt"), dir.path().join("b.fodt"));
    write(&first, FODT);
    write(&second, FODT);
    let a = worker
        .load(&crate::file_url(&first))
        .expect("a loads");
    let b = worker
        .load(&crate::file_url(&second))
        .expect("b loads");
    worker.run(&a, "edit.select_all", &[]).expect("select all");
    worker.cut(&a).expect("the cut runs");
    let saved = |doc: &Doc, worker: &mut Worker, name: &str| {
        let out = dir.path().join(name);
        doc.save_as(worker, &crate::file_url(&out), "fodt")
            .expect("the save works");
        std::fs::read_to_string(&out).expect("the save reads")
    };
    assert!(
        !saved(&a, &mut worker, "a-out.fodt").contains("Hello"),
        "a lost its text"
    );
    assert!(
        saved(&b, &mut worker, "b-out.fodt").contains("Hello"),
        "b kept its text"
    );
    worker.close_doc(b).expect("b closes");
    worker.close_doc(a).expect("a closes");
}

#[test]
fn writer_reports_its_page_rects() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("note.fodt");
    write(&source, FODT);
    let doc = worker
        .load(&crate::file_url(&source))
        .expect("the document loads");
    let pages = doc.pages(&mut worker).expect("the page rects");
    let (width, height) = doc.size(&mut worker).expect("the size");
    assert_eq!(pages.len(), 1, "one page: {pages:?}");
    let (x, y, w, h) = pages[0];
    assert!(
        x > 0 && y > 0 && x + w < width as i32 && y + h <= height as i32,
        "inside the document: {pages:?}"
    );
}

/// A tile's mean channel value, 0 to 255.
fn brightness(tile: &Tile) -> u32 {
    let sum: u64 = tile
        .pixels
        .as_chunks::<4>().0.iter()
        .map(|px| u64::from(px[0]) + u64::from(px[1]) + u64::from(px[2]))
        .sum();
    (sum / (tile.pixels.len() as u64 / 4 * 3)) as u32
}

#[test]
fn ely_scheme_paints_the_page_white() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    for (name, text) in [("note.fodt", FODT), ("book.fods", super::input::FODS)] {
        let source = dir.path().join(name);
        write(&source, text);
        let doc = worker
            .load(&crate::file_url(&source))
            .expect("the document loads");
        poll_until(&mut worker, &doc, &Events::default(), QUIET, |_| false);
        worker.run(&doc, "view.theme", &["COLOR_SCHEME_ELY_LIGHT"]).expect("the scheme");
        let tile = doc.paint_tile(&mut worker, (1440, 1440, 3840, 3840), (64, 64)).expect("a tile");
        let shade = brightness(&tile);
        assert!(shade > 200, "{name}: the page is light, {shade}");
        worker.close_doc(doc).expect("the document closes");
    }
}
