//! The worker against generated PDFs.

use std::path::Path;

use ely_pdf_engine::{Doc, Worker, sample::pdf};

fn worker() -> Worker {
    Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker")
}


fn opened(dir: &Path, bytes: &[u8]) -> (Worker, Doc) {
    let path = dir.join("doc.pdf");
    std::fs::write(&path, bytes).expect("writes");
    let mut worker = worker();
    let opened = worker.open(&path).expect("opens");
    assert!(!opened.locked);
    (worker, opened.doc)
}

#[test]
fn pages_measure_render_and_read_their_text() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let bytes = pdf(
        &[
            (612.0, 792.0, &["Ely Office contract", "Second line"]),
            (842.0, 595.0, &["Wide page"]),
        ],
        &[],
    );
    let (mut worker, doc) = opened(dir.path(), &bytes);
    assert_eq!(worker.pages(doc).expect("counts"), 2);
    assert_eq!(
        worker.bounds(doc, 0, 2).expect("bounds"),
        [[0.0, 0.0, 612.0, 792.0], [0.0, 0.0, 842.0, 595.0]]
    );
    // Page one's top left, twice its size: text is dark.
    let tile = worker
        .render(doc, 0, (2.0, 0), [0, 0, 600, 300])
        .expect("renders").bgra;
    assert_eq!(tile.len(), 600 * 300 * 4);
    let dark = tile.chunks_exact(4).filter(|p| p[0] < 128).count();
    assert!(dark > 100, "text shows: {dark}");
    // A quarter turn makes the page tall side wide.
    assert!(
        worker
            .render(doc, 1, (1.0, 90), [0, 0, 595, 842])
            .is_ok()
    );
    let text: String = worker
        .text(doc, 0)
        .expect("reads")
        .iter()
        .map(|c| c.c)
        .collect();
    assert!(
        text.contains("Ely Office contract") && text.contains("Second line"),
        "{text}"
    );
    let chars = worker.text(doc, 0).expect("reads");
    assert!(
        chars.windows(2).all(|w| w[0].line <= w[1].line),
        "reading order"
    );
    assert!(
        chars[0].rect[1] < chars.last().expect("chars").rect[1],
        "first line above"
    );
}

#[test]
fn search_finds_every_hit_and_bookmarks_nest() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let pages: &[(f32, f32, &[&str])] = &[
        (612.0, 792.0, &["alpha beta alpha"]),
        (612.0, 792.0, &["no match here"]),
        (612.0, 792.0, &["Alpha at last"]),
    ];
    let bytes = pdf(pages, &[("Intro", 0, &[("Detail", 1)]), ("End", 2, &[])]);
    let (mut worker, doc) = opened(dir.path(), &bytes);
    let hits: Vec<usize> = (0..3)
        .map(|page| worker.search(doc, page, "alpha").expect("searches").len())
        .collect();
    assert_eq!(hits, [2, 0, 1], "case is ignored");
    let quad = worker.search(doc, 0, "beta").expect("searches")[0][0];
    assert!(quad[0] > 72.0 && quad[1] > 0.0, "{quad:?}");
    let marks = worker.outline(doc).expect("outline");
    let flat: Vec<(&str, Option<usize>, usize)> = marks
        .iter()
        .map(|m| (m.title.as_str(), m.page, m.down.len()))
        .collect();
    assert_eq!(flat, [("Intro", Some(0), 1), ("End", Some(2), 0)]);
    assert_eq!(
        (marks[0].down[0].title.as_str(), marks[0].down[0].page),
        ("Detail", Some(1))
    );
    worker.close(doc).expect("closes");
    assert!(worker.pages(doc).is_err(), "a closed document is gone");
}

#[test]
fn a_broken_file_is_an_error_and_the_worker_lives_on() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("junk.pdf");
    std::fs::write(&path, b"this is not a pdf at all").expect("writes");
    let mut worker = worker();
    let failed = worker.open(&path).expect_err("not a PDF");
    assert!(!failed.to_string().is_empty());
    assert!(worker.open(&dir.path().join("missing.pdf")).is_err());
    let good = dir.path().join("good.pdf");
    std::fs::write(&good, pdf(&[(612.0, 792.0, &["Still here"])], &[])).expect("writes");
    assert_eq!(
        worker.open(&good).map(|o| o.locked).ok(),
        Some(false),
        "the worker still works"
    );
}

#[test]
fn a_thousand_pages_show_the_first_in_time() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let lines: &[&str] = &["A long report, page after page."];
    let pages: Vec<(f32, f32, &[&str])> = (0..1000).map(|_| (612.0, 792.0, lines)).collect();
    let bytes = pdf(&pages, &[]);
    let started = std::time::Instant::now();
    let (mut worker, doc) = opened(dir.path(), &bytes);
    assert_eq!(worker.pages(doc).expect("counts"), 1000);
    // What the viewer asks before its first frame.
    worker.bounds(doc, 0, 100).expect("the first sizes");
    worker.render(doc, 0, (2.0, 0), [0, 0, 512, 512]).expect("the first tile");
    let took = started.elapsed();
    assert!(took < std::time::Duration::from_secs(3), "PERF-05: {took:?}");
}

#[test]
fn every_hit_counts_once_however_many_or_however_split() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let busy = ["alpha "; 12].concat();
    let many: Vec<&str> = (0..50).map(|_| busy.as_str()).collect();
    let pages: &[(f32, f32, &[&str])] = &[(1000.0, 1400.0, &many), (612.0, 792.0, &["hello", "world"])];
    let (mut worker, doc) = opened(dir.path(), &pdf(pages, &[]));
    assert_eq!(worker.search(doc, 0, "alpha").expect("searches").len(), 600, "no cap on hits");
    let split = worker.search(doc, 1, "hello world").expect("searches");
    assert_eq!(split.len(), 1, "one phrase over two lines");
    assert_eq!(split[0].len(), 2, "with a quad on each line");
}

#[cfg(unix)]
#[test]
fn a_worker_from_other_sources_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    // Answers HELLO with someone else's stamp.
    let stale = dir.path().join("stale-worker");
    std::fs::write(
        &stale,
        "#!/usr/bin/env python3\nimport struct, sys\nhead = sys.stdin.buffer.read(13)\nstamp = b'old'\nbody = struct.pack('<I', len(stamp)) + stamp\nsys.stdout.buffer.write(struct.pack('<I', len(body)) + bytes([100]) + head[5:13] + body)\nsys.stdout.flush()\n",
    )
    .expect("writes");
    std::fs::set_permissions(&stale, std::os::unix::fs::PermissionsExt::from_mode(0o755)).expect("executable");
    let refused = Worker::spawn_from(&stale).err().expect("refused");
    assert!(refused.to_string().contains("other sources"), "{refused}");
}

#[cfg(unix)]
#[test]
fn a_reply_to_another_request_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    // Answers HELLO under the next request's id.
    let crossed = dir.path().join("crossed-worker");
    std::fs::write(
        &crossed,
        "#!/usr/bin/env python3\nimport struct, sys\nhead = sys.stdin.buffer.read(13)\nasked = struct.unpack('<Q', head[5:13])[0]\nbody = struct.pack('<I', 3) + b'any'\nsys.stdout.buffer.write(struct.pack('<I', len(body)) + bytes([100]) + struct.pack('<Q', asked + 1) + body)\nsys.stdout.flush()\n",
    )
    .expect("writes");
    std::fs::set_permissions(&crossed, std::os::unix::fs::PermissionsExt::from_mode(0o755)).expect("executable");
    let refused = Worker::spawn_from(&crossed).err().expect("refused");
    assert!(refused.to_string().contains("not 1"), "{refused}");
}

/// A noisy RGB PNG, stored uncompressed: large on disk.
#[cfg(unix)]
fn big_png(path: &Path, side: u32) {
    let crc = |bytes: &[u8]| {
        let mut crc = 0xffff_ffffu32;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 { 0xedb8_8320 ^ (crc >> 1) } else { crc >> 1 };
            }
        }
        !crc
    };
    let chunk = |kind: &[u8], data: &[u8]| {
        let body = [kind, data].concat();
        [&(data.len() as u32).to_be_bytes()[..], &body, &crc(&body).to_be_bytes()].concat()
    };
    let mut raw = Vec::new();
    let mut seed = 7u32;
    for _ in 0..side {
        // Each row: filter byte none, then its noise.
        let noise = (0..side * 3).map(|_| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            (seed >> 16) as u8
        });
        raw.extend(std::iter::once(0).chain(noise));
    }
    // zlib with stored blocks; Adler-32 at the end.
    let mut zlib = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (n, block) in blocks.iter().enumerate() {
        zlib.push(u8::from(n + 1 == blocks.len()));
        let len = block.len() as u16;
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for byte in &raw {
        a = (a + u32::from(*byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    zlib.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let header = [&side.to_be_bytes()[..], &side.to_be_bytes(), &[8, 2, 0, 0, 0]].concat();
    let png = [&b"\x89PNG\r\n\x1a\n"[..], &chunk(b"IHDR", &header), &chunk(b"IDAT", &zlib), &chunk(b"IEND", &[])].concat();
    std::fs::write(path, png).expect("writes");
}

#[cfg(unix)]
#[test]
fn a_failed_open_keeps_no_memory() {
    // An image MuPDF opens, then refuses to journal.
    let dir = tempfile::tempdir().expect("a temp dir");
    let png = dir.path().join("big.png");
    big_png(&png, 1200);
    let mut worker = Worker::spawn().expect("a worker");
    let rss = |pid: u32| -> u64 {
        let out = std::process::Command::new("ps").args(["-o", "rss=", "-p", &pid.to_string()]).output().expect("ps");
        String::from_utf8_lossy(&out.stdout).trim().parse().expect("kilobytes")
    };
    // MuPDF's 256 MB store fills first; then nothing grows.
    for _ in 0..70 {
        let refused = worker.open(&png).expect_err("not a PDF");
        assert!(refused.to_string().contains("not a PDF"), "{refused}");
    }
    let before = rss(worker.pid());
    for _ in 0..50 {
        assert!(worker.open(&png).is_err(), "not a PDF");
    }
    let grown = rss(worker.pid()).saturating_sub(before);
    assert!(grown < 40_000, "50 more failed opens of a 4 MB image kept {grown} KiB");
}

#[test]
fn a_document_belongs_to_its_worker() {
    let first = tempfile::tempdir().expect("a temp dir");
    let (old_worker, old) = opened(first.path(), &pdf(&[(300.0, 500.0, &["A"])], &[]));
    drop(old_worker);
    let second = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(second.path(), &pdf(&[(300.0, 500.0, &["B"])], &[]));
    // An old worker's handle never reaches this one's documents.
    assert!(worker.close(old).is_err(), "another worker's document closed");
    assert!(worker.pages(old).is_err(), "another worker's document answered");
    assert_eq!(worker.pages(doc).expect("its own document"), 1);
}

#[test]
fn an_edit_made_for_an_older_state_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let pages: &[(f32, f32, &[&str])] = &[(300.0, 500.0, &["A"]), (300.0, 500.0, &["B"]), (300.0, 500.0, &["C"])];
    let (mut worker, before) = opened(dir.path(), &pdf(pages, &[]));
    let mark = worker.delete_pages(before, &[0]).expect("deletes A");
    let after = before.at(&mark).expect("the new state");
    // Page 1 was B, now C; the old aim fails.
    let stale = worker.rotate_pages(before, &[1], 90).expect_err("an older state");
    assert!(ely_pdf_engine::is_stale(&stale), "{stale}");
    assert!(worker.bounds(before, 0, 1).is_err(), "a read for an older state");
    assert_eq!(worker.bounds(after, 1, 1).expect("C")[0], [0.0, 0.0, 300.0, 500.0], "C stayed upright");
    worker.rotate_pages(after, &[1], 90).expect("rotates C");
    // Whole-document requests take any state.
    assert_eq!(worker.pages(before).expect("pages"), 2);
}
