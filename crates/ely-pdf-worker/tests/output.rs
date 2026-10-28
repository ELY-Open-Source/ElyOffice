//! Images, text, optimized copies, printing leave, repaired files (C11.8).

use std::path::Path;

use ely_pdf_engine::{
    Doc, Lock, Optimizing, Worker,
    proto::may,
    sample::{Extras, one_page, pdf_with},
};

fn worker() -> Worker {
    Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker")
}

fn opened(path: &Path) -> (Worker, Doc) {
    let mut worker = worker();
    let doc = worker.open(path).expect("opens").doc;
    (worker, doc)
}

fn two_pages(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("doc.pdf");
    std::fs::write(&path, pdf_with(&[(612.0, 792.0, &["First page"]), (612.0, 792.0, &["Second page"])], &[], &Extras::default())).expect("writes");
    path
}

#[test]
fn a_page_exports_as_a_png_of_its_size() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&two_pages(dir.path()));
    let png = dir.path().join("page.png");
    worker.export_image(doc, 1, 144.0, &png).expect("exports");
    let bytes = std::fs::read(&png).expect("reads");
    assert_eq!(&bytes[1..4], b"PNG");
    let size = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().expect("four bytes"));
    assert_eq!((size(16), size(20)), (1224, 1584), "twice the points");
    assert!(worker.export_image(doc, 0, 5000.0, &png).is_err(), "an absurd resolution");
}

#[test]
fn text_exports_page_by_page() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&two_pages(dir.path()));
    let txt = dir.path().join("doc.txt");
    worker.export_text(doc, &txt).expect("exports");
    let text = std::fs::read_to_string(&txt).expect("UTF-8");
    let pages: Vec<&str> = text.split('\u{c}').collect();
    assert_eq!(pages.len(), 2);
    assert!(pages[0].contains("First page") && pages[1].contains("Second page"), "{text:?}");
}

/// A 600-pixel picture drawn 100 points wide: 432 dpi.
fn heavy(dir: &Path) -> std::path::PathBuf {
    let pixels = vec![120u8; 600 * 600 * 3];
    let mut picture = format!("<< /Type /XObject /Subtype /Image /Width 600 /Height 600 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length {} >>\nstream\n", pixels.len()).into_bytes();
    picture.extend_from_slice(&pixels);
    picture.extend_from_slice(b"\nendstream");
    let path = dir.join("heavy.pdf");
    let mut bytes = one_page("", " /Resources << /XObject << /Im 5 0 R >> >>", "q 100 0 0 100 72 600 cm /Im Do Q", &["PICTURE".to_string()]);
    let at = bytes.windows(7).position(|w| w == b"PICTURE").expect("the slot");
    bytes.splice(at..at + 7, picture);
    std::fs::write(&path, &bytes).expect("writes");
    path
}

#[test]
fn an_optimized_copy_is_smaller_and_its_picture_downsampled() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let source = heavy(dir.path());
    let (mut worker, doc) = opened(&source);
    let out = dir.path().join("small.pdf");
    worker.optimize(doc, &out, Optimizing { compress: true, from: 300, to: 150 }).expect("optimizes");
    let (before, after) = (std::fs::metadata(&source).expect("size").len(), std::fs::metadata(&out).expect("size").len());
    assert!(after * 10 < before, "{before} -> {after}");
    let mut other = self::worker();
    let read = other.open(&out).expect("opens").doc;
    assert_eq!(other.pages(read).expect("pages"), 1);
    let text = String::from_utf8_lossy(&std::fs::read(&out).expect("reads")).into_owned();
    let width: u32 = text.split("/Width ").nth(1).and_then(|rest| rest.split(|c: char| !c.is_ascii_digit()).next()).and_then(|n| n.parse().ok()).expect("a width");
    assert!((205..=212).contains(&width), "150 dpi over 100 points, not {width}");
    assert!(worker.optimize(doc, &out, Optimizing { compress: true, from: 150, to: 300 }).is_err(), "up is not down");
}

#[test]
fn printing_and_copying_need_leave() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&two_pages(dir.path()));
    let limited = dir.path().join("limited.pdf");
    worker.protect(doc, &limited, Some(&Lock { user: String::new(), owner: "owner".into(), allow: may::EDIT })).expect("protects");
    let (mut other, read) = opened(&limited);
    for refused in [other.print_copy(read, &dir.path().join("p.pdf")), other.export_text(read, &dir.path().join("t.txt"))] {
        let refused = refused.expect_err("refused");
        assert!(ely_pdf_engine::refused(&refused).is_some(), "refused: {refused}");
    }
    worker.print_copy(doc, &dir.path().join("print.pdf")).expect("an unprotected file prints");
}

#[test]
fn a_damaged_file_opens_as_a_repaired_copy_and_stays_as_it_was() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = two_pages(dir.path());
    let mut bytes = std::fs::read(&path).expect("reads");
    // Breaks every cross-reference offset.
    let at = bytes.windows(9).rposition(|w| w == b"startxref").expect("the trailer");
    bytes.truncate(at);
    bytes.extend_from_slice(b"startxref\n9\n%%EOF\n");
    std::fs::write(&path, &bytes).expect("writes");
    let (mut worker, mut doc) = opened(&path);
    let report = worker.repair_report(doc).expect("asks").expect("repaired");
    assert!(!report.trim().is_empty(), "MuPDF says what it found");
    assert_eq!(worker.pages(doc).expect("pages"), 2);
    worker.rotate_pages(doc, &[0], 90).expect("edits the copy");
    doc = worker.now(doc).expect("the state");
    worker.save(doc, &dir.path().join("fixed.pdf")).expect("saves the copy");
    assert_eq!(std::fs::read(&path).expect("reads"), bytes, "the damaged original stays");
    let elsewhere = tempfile::tempdir().expect("a temp dir");
    let (mut fresh, clean) = opened(&two_pages(elsewhere.path()));
    assert_eq!(fresh.repair_report(clean).expect("asks"), None);
}

#[test]
fn a_broken_object_with_a_sound_trailer_is_repaired_on_open() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = two_pages(dir.path());
    let mut bytes = std::fs::read(&path).expect("reads");
    // The page tree's offset points nowhere; startxref is sound.
    let table = bytes.windows(6).position(|w| w == b"\nxref\n").expect("the table") + 6;
    let entries = table + bytes[table..].iter().position(|b| *b == b'\n').expect("a header") + 1;
    let at = entries + 2 * 20;
    bytes[at..at + 10].copy_from_slice(b"0000000009");
    std::fs::write(&path, &bytes).expect("writes");
    let (mut worker, mut doc) = opened(&path);
    assert!(worker.repair_report(doc).expect("asks").is_some(), "found at open, not mid-edit");
    worker.rotate_pages(doc, &[0], 90).expect("edits");
    doc = worker.now(doc).expect("the state");
    worker.save(doc, &dir.path().join("fixed.pdf")).expect("saves");
    assert_eq!(std::fs::read(&path).expect("reads"), bytes);
}

#[test]
fn a_broken_stream_under_a_sound_table_is_damage_and_fails_exports() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let broken = "<< /Length 12 /Filter /FlateDecode >>\nstream\nnot a zlib!!\nendstream".to_string();
    let path = dir.path().join("doc.pdf");
    let bytes = one_page("", " /Contents 5 0 R", "", &[broken]);
    std::fs::write(&path, &bytes).expect("writes");
    let (mut worker, mut doc) = opened(&path);
    let report = worker.repair_report(doc).expect("asks").expect("damage is reported");
    assert!(!report.trim().is_empty());
    assert!(worker.export_text(doc, &dir.path().join("doc.txt")).is_err(), "partial text is no export");
    assert!(worker.export_image(doc, 0, 72.0, &dir.path().join("page.png")).is_err(), "nor a partial page");
    worker.rotate_pages(doc, &[0], 90).expect("edits the copy");
    doc = worker.now(doc).expect("the state");
    worker.save(doc, &dir.path().join("fixed.pdf")).expect("saves the copy");
    assert_eq!(std::fs::read(&path).expect("reads"), bytes, "the original stays");
}

#[test]
fn a_print_copy_carries_no_lock() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&two_pages(dir.path()));
    let locked = dir.path().join("locked.pdf");
    worker.protect(doc, &locked, Some(&Lock { user: "user".into(), owner: "owner".into(), allow: may::PRINT })).expect("protects");
    let (mut other, read) = opened(&locked);
    assert!(other.unlock(read, "user").expect("tries"));
    let print = dir.path().join("print.pdf");
    other.print_copy(read, &print).expect("writes");
    let mut third = self::worker();
    let opened = third.open(&print).expect("opens");
    assert!(!opened.locked, "the printer needs no password");
    assert_eq!(third.pages(opened.doc).expect("pages"), 2);
}

#[test]
fn a_print_copy_shows_what_prints_to_any_screen_renderer() {
    let dir = tempfile::tempdir().expect("a temp dir");
    // No looks of their own: MuPDF makes them.
    let square = |flags: u8, rect: &str, color: &str| format!("<< /Type /Annot /Subtype /Square /F {flags} /Rect [{rect}] /C [{color}] /IC [{color}] >>");
    let annots = [
        square(4, "50 100 150 180", "1 0 0"),
        square(0, "200 100 300 180", "0 1 0"),
        // Print and NoView: printed, never shown.
        square(36, "350 100 450 180", "0 0 1"),
        // Print and Hidden: neither.
        square(6, "50 300 150 380", "0 1 0"),
        "<< /Type /Annot /Subtype /Link /F 4 /Rect [200 300 300 380] /A << /S /URI /URI (https://example.com) >> >>".to_string(),
    ];
    let path = dir.path().join("doc.pdf");
    std::fs::write(&path, one_page("", " /Annots [5 0 R 6 0 R 7 0 R 8 0 R 9 0 R]", "", &annots)).expect("writes");
    let (mut worker, doc) = opened(&path);
    let print = dir.path().join("print.pdf");
    worker.print_copy(doc, &print).expect("writes");
    // Only the link stays an annotation.
    let copy = lopdf::Document::load(&print).expect("a PDF");
    let left = copy.get_dictionary(copy.get_pages()[&1]).and_then(|page| page.get(b"Annots")).and_then(lopdf::Object::as_array).expect("annotations");
    let kinds: Vec<&[u8]> = left.iter().map(|annot| copy.dereference(annot).and_then(|(_, a)| a.as_dict()?.get(b"Subtype")?.as_name()).expect("a kind")).collect();
    assert_eq!(kinds, [b"Link".as_slice()]);
    assert!(copy.catalog().expect("a catalog").get(b"AcroForm").is_err(), "no form left");
    // MuPDF draws by screen rules, as Windows' renderer does.
    let mut third = self::worker();
    let printed = third.open(&print).expect("opens").doc;
    let bgra = third.render(printed, 0, (1.0, 0), [0, 0, 612, 792]).expect("draws").bgra;
    let at = |x: usize, y: usize| -> [u8; 3] {
        let p = &bgra[((792 - y) * 612 + x) * 4..][..3];
        [p[2], p[1], p[0]]
    };
    assert_eq!(at(100, 140), [255, 0, 0], "printable");
    assert_eq!(at(250, 140), [255, 255, 255], "not for print");
    assert_eq!(at(400, 140), [0, 0, 255], "print, not view");
    assert_eq!(at(100, 340), [255, 255, 255], "hidden");
}

#[test]
fn a_broken_picture_shows_when_drawn_or_checked() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let picture = "<< /Type /XObject /Subtype /Image /Width 4 /Height 4 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length 12 >>\nstream\nnot a zlib!!\nendstream".to_string();
    let path = dir.path().join("doc.pdf");
    let bytes = one_page("", " /Resources << /XObject << /Im 5 0 R >> >>", "q 100 0 0 100 72 600 cm /Im Do Q", &[picture]);
    std::fs::write(&path, &bytes).expect("writes");
    let (mut worker, doc) = opened(&path);
    assert_eq!(worker.repair_report(doc).expect("asks"), None, "pictures wait to be drawn");
    let tile = worker.render(doc, 0, (1.0, 0), [0, 0, 612, 792]).expect("draws");
    assert!(tile.damaged, "drawing finds it");
    assert!(worker.repair_report(doc).expect("asks").is_some());
    let (mut other, again) = opened(&path);
    assert!(other.check_pictures(again).expect("checks"), "the check finds it undrawn");
    assert_eq!(std::fs::read(&path).expect("reads"), bytes);
}

/// As many Chinese files name their fonts.
const SIMSUN: &str = "<< /Type /Font /Subtype /Type0 /BaseFont /SimSun /Encoding /Identity-H /DescendantFonts [<< /Type /Font /Subtype /CIDFontType2 /BaseFont /SimSun /CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 2 >> /FontDescriptor << /Type /FontDescriptor /FontName /SimSun /Flags 6 /FontBBox [0 -141 996 855] /ItalicAngle 0 /Ascent 859 /Descent -141 /CapHeight 859 /StemV 91 >> >>] >>";

/// Two runs, each in its own unembedded font.
const TWO_RUNS: &str = "BT /F1 24 Tf 72 700 Td <00410042> Tj /F2 24 Tf <0043> Tj ET";

#[test]
fn an_unembedded_font_is_no_damage() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("doc.pdf");
    // Two fonts, one notice: a repeat is no damage either.
    let fonts = " /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >>";
    let bytes = one_page("", fonts, TWO_RUNS, &[SIMSUN.to_string(), SIMSUN.to_string()]);
    std::fs::write(&path, &bytes).expect("writes");
    // Each on a fresh worker: no font loaded before.
    let (mut worker, doc) = opened(&path);
    worker.export_text(doc, &dir.path().join("doc.txt")).expect("the text exports");
    let (mut worker, doc) = opened(&path);
    worker.export_image(doc, 0, 72.0, &dir.path().join("page.png")).expect("the page exports");
    let (mut worker, doc) = opened(&path);
    // Read unwatched first: its repeat is pending.
    worker.text(doc, 0).expect("reads");
    let tile = worker.render(doc, 0, (1.0, 0), [0, 0, 612, 792]).expect("draws");
    assert!(!tile.damaged, "a substituted font draws whole");
    assert_eq!(worker.repair_report(doc).expect("asks"), None);
}

#[test]
fn unreadable_content_still_draws_as_damage() {
    let dir = tempfile::tempdir().expect("a temp dir");
    // Text with no font; a stray bracket.
    for content in ["BT 72 700 Td (Missing text) Tj ET", "BT ] ET"] {
        let path = dir.path().join("doc.pdf");
        std::fs::write(&path, one_page("", "", content, &[])).expect("writes");
        let (mut worker, doc) = opened(&path);
        let tile = worker.render(doc, 0, (1.0, 0), [0, 0, 612, 792]).expect("draws");
        assert!(tile.damaged, "{content} draws as damage");
    }
}

/// Two pictures whose zlib checksums are wrong.
fn broken_pictures(dir: &Path) -> std::path::PathBuf {
    let hex = format!("{}>", "ff0000".repeat(16));
    // One stored deflate block, its checksum wrong.
    let mut zlib = vec![0x78, 0x01, 0x01];
    zlib.extend_from_slice(&(hex.len() as u16).to_le_bytes());
    zlib.extend_from_slice(&(!(hex.len() as u16)).to_le_bytes());
    zlib.extend_from_slice(hex.as_bytes());
    zlib.extend_from_slice(&[0, 0, 0, 0]);
    let slot = "#".repeat(zlib.len());
    let image = format!("<< /Type /XObject /Subtype /Image /Width 4 /Height 4 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter [/FlateDecode /ASCIIHexDecode] /Length {} >>
stream
{slot}
endstream", zlib.len());
    let names = " /Resources << /XObject << /Im1 5 0 R /Im2 6 0 R >> >>";
    let mut bytes = one_page("", names, "q 100 0 0 100 72 600 cm /Im1 Do Q q 100 0 0 100 72 400 cm /Im2 Do Q", &[image.clone(), image]);
    for _ in 0..2 {
        let at = bytes.windows(slot.len()).position(|w| w == slot.as_bytes()).expect("the slot");
        bytes[at..at + zlib.len()].copy_from_slice(&zlib);
    }
    let path = dir.join("doc.pdf");
    std::fs::write(&path, &bytes).expect("writes");
    path
}

/// A picture whose colour key mask is invalid.
fn masked() -> String {
    let hex = format!("{}>", "ff0000".repeat(16));
    format!("<< /Type /XObject /Subtype /Image /Width 4 /Height 4 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Mask [null null null null null null] /Filter /ASCIIHexDecode /Length {} >>
stream
{hex}
endstream", hex.len())
}

/// The bad mask, alone or drawn beside two font notices.
fn bad_mask(dir: &Path, fonts: Option<bool>) -> std::path::PathBuf {
    let picture = "q 100 0 0 100 72 600 cm /Im1 Do Q";
    let (resources, content, objects) = match fonts {
        None => (String::new(), picture.to_string(), vec![masked()]),
        Some(first) => (
            " /Font << /F1 6 0 R /F2 7 0 R >>".to_string(),
            if first { format!("{picture} {TWO_RUNS}") } else { format!("{TWO_RUNS} {picture}") },
            vec![masked(), SIMSUN.to_string(), SIMSUN.to_string()],
        ),
    };
    let names = format!(" /Resources << /XObject << /Im1 5 0 R >>{resources} >>");
    let path = dir.join(format!("mask-{fonts:?}.pdf"));
    std::fs::write(&path, one_page("", &names, &content, &objects)).expect("writes");
    path
}

#[test]
fn a_broken_picture_under_two_filters_fails_exports() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = broken_pictures(dir.path());
    let (mut worker, doc) = opened(&path);
    assert!(worker.export_text(doc, &dir.path().join("doc.txt")).is_err(), "its text read whole");
    let (mut worker, doc) = opened(&path);
    assert!(worker.export_image(doc, 0, 72.0, &dir.path().join("page.png")).is_err(), "its page read whole");
    let (mut worker, doc) = opened(&path);
    let tile = worker.render(doc, 0, (1.0, 0), [0, 0, 612, 792]).expect("draws");
    assert!(tile.damaged, "the picture draws as damage");
}

#[test]
fn a_broken_picture_read_before_is_still_damage() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let files = [broken_pictures(dir.path()), bad_mask(dir.path(), None), bad_mask(dir.path(), Some(true)), bad_mask(dir.path(), Some(false))];
    // Reading caches the pictures; their repeat waits.
    let read: [fn(&mut Worker, Doc); 2] = [|w, d| drop(w.text(d, 0).expect("reads")), |w, d| drop(w.search(d, 0, "x").expect("searches"))];
    for (path, first) in files.iter().flat_map(|p| read.map(|r| (p, r))) {
        let (mut worker, doc) = opened(path);
        first(&mut worker, doc);
        let tile = worker.render(doc, 0, (1.0, 0), [0, 0, 612, 792]).expect("draws");
        assert!(tile.damaged, "the picture draws as damage");
        assert!(worker.repair_report(doc).expect("asks").is_some());
        let (mut worker, doc) = opened(path);
        first(&mut worker, doc);
        assert!(worker.export_image(doc, 0, 72.0, &dir.path().join("page.png")).is_err(), "its page read whole");
        let (mut worker, doc) = opened(path);
        first(&mut worker, doc);
        assert!(worker.export_text(doc, &dir.path().join("doc.txt")).is_err(), "its text read whole");
        let (mut worker, doc) = opened(path);
        first(&mut worker, doc);
        assert!(worker.check_pictures(doc).expect("checks"), "the check finds it");
    }
}
