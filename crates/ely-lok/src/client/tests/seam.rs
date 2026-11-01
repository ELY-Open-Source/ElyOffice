//! §7: neighbouring tiles join; display scaling changes nothing.

use super::*;

/// Words across both tiles of the first text row.
const LINE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>Ely Office opens what Microsoft Office writes, edge to edge.</text:p></office:text></office:body>
</office:document>"#;

/// Columns `cols` of the image's first `rows` rows.
fn cut(pixels: &[u8], width: usize, cols: std::ops::Range<usize>, rows: usize) -> Vec<u8> {
    (0..rows)
        .flat_map(|row| pixels[(row * width + cols.start) * 4..(row * width + cols.end) * 4].to_vec())
        .collect()
}

#[test]
fn two_tiles_match_one_tile_over_both() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("line.fodt");
    write(&source, LINE);
    let doc = worker.load(&crate::file_url(&source)).expect("loads");
    let wide = doc.paint_tile(&mut worker, (0, 0, 7680, 3840), (512, 256)).expect("the wide tile");
    for (n, x) in [0, 3840].into_iter().enumerate() {
        let tile = doc.paint_tile(&mut worker, (x, 0, 3840, 3840), (256, 256)).expect("a tile");
        let part = cut(&wide.pixels, 512, n * 256..n * 256 + 256, 256);
        let inked = part.as_chunks::<4>().0.iter().filter(|px| px[0] < 128).count();
        assert!(inked > 0, "tile {n} holds text");
        let off = part.iter().zip(&tile.pixels).filter(|(a, b)| a.abs_diff(**b) > 48).count();
        // Antialiasing may differ; misplaced text differs in thousands.
        assert!(off < 200, "tile {n} differs from its half of the wide tile in {off} bytes");
    }
    worker.close_doc(doc).expect("closes");
}
