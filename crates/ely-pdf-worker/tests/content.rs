//! Text and object edits against the real worker (PDF-03, PDF-04).

use std::path::Path;

use ely_pdf_engine::{
    Doc, Worker, Writing, Wrote,
    sample::{Extras, one_page, pdf_with},
};

fn worker() -> Worker {
    Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker")
}

/// A Letter page: two lines, a stamp, a picture.
fn opened(dir: &Path) -> (Worker, Doc) {
    let path = dir.join("doc.pdf");
    let extras = Extras { stamp: Some("STAMP"), picture: true, ..Extras::default() };
    std::fs::write(&path, pdf_with(&[(612.0, 792.0, &["Hello world", "Second line"])], &[], &extras)).expect("writes");
    let mut worker = worker();
    let doc = worker.open(&path).expect("opens").doc;
    (worker, doc)
}

fn page_text(worker: &mut Worker, doc: Doc) -> String {
    worker.text(doc, 0).expect("reads").iter().map(|c| c.c).collect()
}

/// Rectangles equal to a hundredth of a point.
fn near(got: [f32; 4], want: [f32; 4]) {
    assert!(got.iter().zip(want).all(|(a, b)| (a - b).abs() < 0.01), "{got:?} is not {want:?}");
}

/// A rewrite of block `block` as `text` in `font`.
fn rewrite(worker: &mut Worker, doc: Doc, block: usize, text: &str, font: &str) -> Writing {
    let read = worker.text_blocks(doc, 0).expect("blocks")[block].clone();
    Writing {
        block: Some(block),
        expected: read.text,
        text: text.into(),
        font: font.into(),
        size: read.size,
        rgb: read.rgb,
        bx: [read.rect[0], read.rect[1], read.rect[2] - read.rect[0]],
    }
}

#[test]
fn blocks_say_what_they_hold_and_whether_they_edit() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(dir.path());
    let blocks = worker.text_blocks(doc, 0).expect("blocks");
    let hello = blocks.iter().find(|b| b.text.starts_with("Hello")).expect("the page text");
    assert!(hello.editable && hello.size == 18.0, "{hello:?}");
    let stamp = blocks.iter().find(|b| b.text == "STAMP").expect("the stamp text");
    assert!(!stamp.editable, "text inside a form is not the page's own");
    let drawn = worker.objects(doc, 0).expect("objects");
    assert_eq!(drawn.iter().map(|d| d.form).collect::<Vec<_>>(), [true, false]);
    assert_eq!(drawn[1].rect, [72.0, 242.0, 172.0, 292.0], "the picture, in page space");
}

#[test]
fn edited_text_replaces_the_old_glyphs_in_one_step() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(dir.path());
    let before = worker.history(doc).expect("a mark");
    let index = worker.text_blocks(doc, 0).expect("blocks").iter().position(|b| b.text.starts_with("Hello")).expect("hello");
    let writing = rewrite(&mut worker, doc, index, "Hi world\nSecond line", "helvetica");
    assert!(matches!(worker.put_text(doc, 0, &writing, 0.0).expect("writes"), Wrote::Done(_)));
    doc = worker.now(doc).expect("the state");
    let now = page_text(&mut worker, doc);
    assert!(now.contains("Hi world") && !now.contains("Hello"), "{now}");
    worker.undo(doc, false).expect("undoes");
    doc = worker.now(doc).expect("the state");
    assert_eq!(worker.history(doc).expect("a mark"), before);
    assert!(page_text(&mut worker, doc).contains("Hello world"));
}

#[test]
fn a_font_without_the_glyph_changes_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(dir.path());
    let before = worker.history(doc).expect("a mark");
    let index = worker.text_blocks(doc, 0).expect("blocks").iter().position(|b| b.text.starts_with("Hello")).expect("hello");
    let writing = rewrite(&mut worker, doc, index, "你好", "helvetica");
    assert_eq!(worker.put_text(doc, 0, &writing, 0.0).expect("answers"), Wrote::Missing('你'));
    doc = worker.now(doc).expect("the state");
    assert_eq!(worker.history(doc).expect("a mark"), before);
    assert!(page_text(&mut worker, doc).contains("Hello world"), "the old text stays");
    let chinese = Writing { font: "cjk-sc".into(), ..writing };
    assert!(matches!(worker.put_text(doc, 0, &chinese, 0.0).expect("writes"), Wrote::Done(_)));
    doc = worker.now(doc).expect("the state");
    assert!(page_text(&mut worker, doc).contains("你好"));
}

#[test]
fn a_preview_draws_the_change_and_keeps_none_of_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(dir.path());
    let before = worker.history(doc).expect("a mark");
    let index = worker.text_blocks(doc, 0).expect("blocks").iter().position(|b| b.text.starts_with("Hello")).expect("hello");
    let mut writing = rewrite(&mut worker, doc, index, "A much longer line that will not fit the old box at all", "times");
    writing.bx[2] = 150.0;
    let Wrote::Preview(pixels, (w, h), grew) = worker.put_text(doc, 0, &writing, 2.0).expect("previews") else {
        panic!("a preview");
    };
    doc = worker.now(doc).expect("the state");
    assert!(grew, "more lines than before");
    assert_eq!(pixels.len(), (w * h * 4) as usize);
    assert!(pixels.chunks_exact(4).any(|p| p[0] < 128), "it draws text");
    assert_eq!(worker.history(doc).expect("a mark"), before);
    assert!(page_text(&mut worker, doc).contains("Hello world"));
}

#[test]
fn new_text_goes_where_asked() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(dir.path());
    let writing = Writing {
        block: None,
        expected: String::new(),
        text: "Added".into(),
        font: "courier".into(),
        size: 12.0,
        rgb: 0xff0000,
        bx: [100.0, 600.0, 300.0],
    };
    assert!(matches!(worker.put_text(doc, 0, &writing, 0.0).expect("writes"), Wrote::Done(_)));
    doc = worker.now(doc).expect("the state");
    let added = worker.text_blocks(doc, 0).expect("blocks").into_iter().find(|b| b.text == "Added").expect("added");
    assert!((added.rect[0] - 100.0).abs() < 1.0 && (added.rect[1] - 600.0).abs() < 3.0, "{added:?}");
    assert_eq!(added.rgb, 0xff0000);
}

#[test]
fn objects_move_scale_swap_and_go() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(dir.path());
    worker.place(doc, 0, 1, [1.0, 0.0, 0.0, 1.0, 50.0, 20.0]).expect("moves");
    doc = worker.now(doc).expect("the state");
    near(worker.objects(doc, 0).expect("objects")[1].rect, [122.0, 262.0, 222.0, 312.0]);
    // Twice the size about its top left.
    worker.place(doc, 0, 1, [2.0, 0.0, 0.0, 2.0, -122.0, -262.0]).expect("scales");
    doc = worker.now(doc).expect("the state");
    near(worker.objects(doc, 0).expect("objects")[1].rect, [122.0, 262.0, 322.0, 362.0]);
    let square = dir.path().join("square.pnm");
    std::fs::write(&square, b"P6\n2 2\n255\n\xff\xff\xff\x00\x00\x00\x00\x00\x00\xff\xff\xff").expect("writes");
    worker.replace_image(doc, 0, 1, &square).expect("replaces");
    doc = worker.now(doc).expect("the state");
    let swapped = worker.objects(doc, 0).expect("objects")[1].rect;
    near(swapped, [172.0, 262.0, 272.0, 362.0]);
    worker.delete_object(doc, 0, 0).expect("deletes the stamp");
    doc = worker.now(doc).expect("the state");
    assert!(!page_text(&mut worker, doc).contains("STAMP"));
    assert_eq!(worker.objects(doc, 0).expect("objects").len(), 1);
    worker.add_image(doc, 0, &square, [400.0, 100.0, 450.0, 150.0]).expect("adds");
    doc = worker.now(doc).expect("the state");
    near(worker.objects(doc, 0).expect("objects").last().expect("added").rect, [400.0, 100.0, 450.0, 150.0]);
    for _ in 0..5 {
        worker.undo(doc, false).expect("undoes");
        doc = worker.now(doc).expect("the state");
    }
    near(worker.objects(doc, 0).expect("objects")[1].rect, [72.0, 242.0, 172.0, 292.0]);
    assert!(page_text(&mut worker, doc).contains("STAMP"));
}

#[test]
fn the_documents_own_font_writes_and_edits_edit_again() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(dir.path());
    let hello = |worker: &mut Worker, doc: Doc, start: &str| {
        worker.text_blocks(doc, 0).expect("blocks").iter().position(|b| b.text.starts_with(start)).expect("the block")
    };
    let index = hello(&mut worker, doc, "Hello");
    let first = rewrite(&mut worker, doc, index, "Hi world\nSecond line", "original");
    assert!(matches!(worker.put_text(doc, 0, &first, 0.0).expect("writes"), Wrote::Done(_)));
    doc = worker.now(doc).expect("the state");
    let again = hello(&mut worker, doc, "Hi");
    assert!(worker.text_blocks(doc, 0).expect("blocks")[again].editable, "new text is the page's own");
    let second = rewrite(&mut worker, doc, again, "Yo world\nSecond line", "original");
    assert!(matches!(worker.put_text(doc, 0, &second, 0.0).expect("writes again"), Wrote::Done(_)));
    doc = worker.now(doc).expect("the state");
    let now = page_text(&mut worker, doc);
    assert!(now.contains("Yo world") && !now.contains("Hi world"), "{now}");
}

#[test]
fn a_font_added_in_a_preview_or_undone_is_added_again() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(dir.path());
    let index = worker.text_blocks(doc, 0).expect("blocks").iter().position(|b| b.text.starts_with("Hello")).expect("hello");
    let writing = rewrite(&mut worker, doc, index, "你好", "cjk-sc");
    assert!(matches!(worker.put_text(doc, 0, &writing, 2.0).expect("previews"), Wrote::Preview(..)));
    doc = worker.now(doc).expect("the state");
    assert!(matches!(worker.put_text(doc, 0, &writing, 0.0).expect("writes"), Wrote::Done(_)));
    doc = worker.now(doc).expect("the state");
    worker.undo(doc, false).expect("undoes");
    doc = worker.now(doc).expect("the state");
    assert!(matches!(worker.put_text(doc, 0, &writing, 0.0).expect("writes again"), Wrote::Done(_)));
    doc = worker.now(doc).expect("the state");
    let out = dir.path().join("out.pdf");
    worker.save(doc, &out).expect("saves");
    let saved = worker.open(&out).expect("reopens").doc;
    assert!(page_text(&mut worker, saved).contains("你好"), "the font is in the file");
}

#[test]
fn previews_and_refusals_keep_the_redo() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(dir.path());
    let index = worker.text_blocks(doc, 0).expect("blocks").iter().position(|b| b.text.starts_with("Hello")).expect("hello");
    let first = rewrite(&mut worker, doc, index, "Hi world\nSecond line", "helvetica");
    assert!(matches!(worker.put_text(doc, 0, &first, 0.0).expect("writes"), Wrote::Done(_)));
    doc = worker.now(doc).expect("the state");
    worker.undo(doc, false).expect("undoes");
    doc = worker.now(doc).expect("the state");
    let preview = rewrite(&mut worker, doc, index, "Yo world\nSecond line", "times");
    assert!(matches!(worker.put_text(doc, 0, &preview, 2.0).expect("previews"), Wrote::Preview(..)));
    doc = worker.now(doc).expect("the state");
    let refused = Writing { text: "你好".into(), font: "helvetica".into(), ..preview };
    assert!(matches!(worker.put_text(doc, 0, &refused, 0.0).expect("answers"), Wrote::Missing(_)));
    doc = worker.now(doc).expect("the state");
    worker.undo(doc, true).expect("the edit still redoes");
    doc = worker.now(doc).expect("the state");
    assert!(page_text(&mut worker, doc).contains("Hi world"));
}

#[test]
fn bigger_glyphs_count_as_a_new_layout() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(dir.path());
    let index = worker.text_blocks(doc, 0).expect("blocks").iter().position(|b| b.text.starts_with("Hello")).expect("hello");
    let same = rewrite(&mut worker, doc, index, "Hello world\nSecond line", "original");
    let Wrote::Preview(_, _, grew) = worker.put_text(doc, 0, &same, 2.0).expect("previews") else { panic!("a preview") };
    doc = worker.now(doc).expect("the state");
    assert!(!grew, "the same text in the same place");
    let big = Writing { text: "Hi".into(), size: 72.0, ..same };
    let Wrote::Preview(_, _, grew) = worker.put_text(doc, 0, &big, 2.0).expect("previews") else { panic!("a preview") };
    assert!(grew, "fewer lines, much bigger glyphs");
}

#[test]
fn runs_on_one_baseline_still_wrap_onto_new_lines() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let content = "BT /F1 18 Tf 1 0 0 1 72 720 Tm (Hello) Tj 1 0 0 1 160 720 Tm (world) Tj ET";
    let objs = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (n, obj) in objs.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{obj}\nendobj\n", n + 1).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes());
    let path = dir.path().join("runs.pdf");
    std::fs::write(&path, bytes).expect("writes");
    let mut worker = worker();
    let mut doc = worker.open(&path).expect("opens").doc;
    let block = worker.text_blocks(doc, 0).expect("blocks")[0].clone();
    let mut writing = rewrite(&mut worker, doc, 0, "One two three four five", "helvetica");
    writing.expected = block.text;
    writing.bx[2] = 50.0;
    assert!(matches!(worker.put_text(doc, 0, &writing, 0.0).expect("writes"), Wrote::Done(_)));
    doc = worker.now(doc).expect("the state");
    let mut tops: Vec<i32> = worker.text(doc, 0).expect("reads").iter().map(|c| c.rect[1].round() as i32).collect();
    tops.dedup();
    assert!(tops.len() >= 3, "each wrapped line has its own baseline: {tops:?}");
}

#[test]
fn a_subset_font_writes_with_its_own_glyphs_and_reads_back() {
    // LibreOffice's export: a subset font whose own tables are placeholders.
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office/t.pdf");
    let mut worker = worker();
    let mut doc = worker.open(&path).expect("opens").doc;
    let at = worker.text_blocks(doc, 0).expect("blocks").iter().position(|b| b.text.contains("BUNDLE")).expect("the block");
    let mut writing = rewrite(&mut worker, doc, at, "TEXT BUNDLE", "original");
    // Room for one line; the space stays a space.
    writing.bx[2] *= 2.0;
    assert!(matches!(worker.put_text(doc, 0, &writing, 0.0).expect("answers"), Wrote::Done(_)), "the page's glyphs suffice");
    doc = worker.now(doc).expect("the state");
    let now = page_text(&mut worker, doc);
    assert!(now.contains("TEXT BUNDLE"), "the new words read back: {now:?}");
}

/// A PDF of `objects`, numbered from 1.
fn assembled(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (at, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend([format!("{} 0 obj\n", at + 1).as_bytes(), body, b"\nendobj\n"].concat());
    }
    let xref = pdf.len();
    pdf.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    offsets.iter().for_each(|at| pdf.extend(format!("{at:010} 00000 n \n").as_bytes()));
    pdf.extend(format!("trailer\n<</Size {}/Root 1 0 R>>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).as_bytes());
    pdf
}

/// "AB" in WinAnsi Helvetica whose ToUnicode maps only "A".
fn partial_unicode_pdf() -> Vec<u8> {
    let stream = |data: &[u8]| [format!("<</Length {}>>stream\n", data.len()).as_bytes(), data, b"\nendstream"].concat();
    let unicode = b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /U def\n1 begincodespacerange <00> <FF> endcodespacerange\n1 beginbfchar <41> <0041> endbfchar\nendcmap CMapName currentdict /CMap defineresource pop end end";
    assembled(&[
        b"<</Type/Catalog/Pages 2 0 R>>".to_vec(),
        b"<</Type/Pages/Kids[3 0 R]/Count 1>>".to_vec(),
        b"<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R>>".to_vec(),
        b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica/Encoding/WinAnsiEncoding/ToUnicode 6 0 R>>".to_vec(),
        stream(b"BT /F1 24 Tf 72 700 Td (AB) Tj ET"),
        stream(unicode),
    ])
}

/// "AyB", where "x" and "y" share one glyph.
fn shared_glyph_pdf() -> Vec<u8> {
    let font = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/GreatVibes-Regular.ttf")).expect("the font");
    let stream = |dict: &str, data: &[u8]| [format!("<<{dict}/Length {}>>stream\n", data.len()).as_bytes(), data, b"\nendstream"].concat();
    let unicode = b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /U def\n1 begincodespacerange <0000> <FFFF> endcodespacerange\n4 beginbfchar <0001> <0041> <0002> <0078> <0003> <0079> <0004> <0042> endbfchar\nendcmap CMapName currentdict /CMap defineresource pop end end";
    let gids: Vec<u8> = [0u16, 36, 16, 16, 37].iter().flat_map(|g| g.to_be_bytes()).collect();
    let objects = [
        b"<</Type/Catalog/Pages 2 0 R>>".to_vec(),
        b"<</Type/Pages/Kids[3 0 R]/Count 1>>".to_vec(),
        b"<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R>>".to_vec(),
        b"<</Type/Font/Subtype/Type0/BaseFont/GreatVibes/Encoding/Identity-H/DescendantFonts[6 0 R]/ToUnicode 7 0 R>>".to_vec(),
        stream("", b"BT /F1 24 Tf 72 700 Td <000100030004> Tj ET"),
        b"<</Type/Font/Subtype/CIDFontType2/BaseFont/GreatVibes/CIDSystemInfo<</Registry(Adobe)/Ordering(Identity)/Supplement 0>>/FontDescriptor 8 0 R/CIDToGIDMap 9 0 R/DW 600>>".to_vec(),
        stream("", unicode),
        b"<</Type/FontDescriptor/FontName/GreatVibes/Flags 4/FontBBox[-200 -300 1200 1000]/ItalicAngle 0/Ascent 900/Descent -300/CapHeight 700/StemV 80/FontFile2 10 0 R>>".to_vec(),
        stream("", &gids),
        stream(&format!("/Length1 {}", font.len()), &font),
    ];
    assembled(&objects)
}

#[test]
fn a_letter_that_shares_its_glyph_keeps_its_own_code() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("shared.pdf");
    std::fs::write(&path, shared_glyph_pdf()).expect("writes");
    let mut worker = worker();
    let mut doc = worker.open(&path).expect("opens").doc;
    assert_eq!(page_text(&mut worker, doc), "AyB", "the sample reads as written");
    let writing = rewrite(&mut worker, doc, 0, "ByA", "original");
    assert!(matches!(worker.put_text(doc, 0, &writing, 0.0).expect("answers"), Wrote::Done(_)));
    doc = worker.now(doc).expect("the state");
    assert_eq!(page_text(&mut worker, doc), "ByA", "y stays y, not the glyph's other letter");
}

#[test]
fn a_partial_to_unicode_falls_back_as_reading_does() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("partial.pdf");
    std::fs::write(&path, partial_unicode_pdf()).expect("writes");
    let mut worker = worker();
    let mut doc = worker.open(&path).expect("opens").doc;
    assert_eq!(page_text(&mut worker, doc), "AB", "the sample reads as written");
    let writing = rewrite(&mut worker, doc, 0, "BA", "original");
    assert!(matches!(worker.put_text(doc, 0, &writing, 0.0).expect("answers"), Wrote::Done(_)), "B is in the font");
    doc = worker.now(doc).expect("the state");
    let out = dir.path().join("saved.pdf");
    worker.save(doc, &out).expect("saves");
    let doc = worker.open(&out).expect("reopens").doc;
    assert_eq!(page_text(&mut worker, doc), "BA", "saved and read again");
}

#[test]
fn a_dense_page_lists_its_blocks_in_one_pass() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut content = String::new();
    for col in 0..10 {
        for row in 0..100 {
            content.push_str(&format!("BT /F1 2 Tf {} {} Td (Cell {col} {row}) Tj ET\n", 20 + col * 58, 780 - row * 7));
        }
    }
    let font = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string();
    let path = dir.path().join("dense.pdf");
    std::fs::write(&path, one_page("", " /Resources << /Font << /F1 5 0 R >> >>", &content, &[font])).expect("writes");
    let mut worker = worker();
    let doc = worker.open(&path).expect("opens").doc;
    let started = std::time::Instant::now();
    let blocks = worker.text_blocks(doc, 0).expect("blocks");
    let took = started.elapsed();
    assert_eq!((blocks.len(), blocks.iter().filter(|b| b.editable).count()), (1000, 1000));
    // Per block took 1.5 s here; one pass 0.1 s.
    assert!(took < std::time::Duration::from_millis(750), "{took:?}");
}

/// One page: `content`, font F1, and form Fm1 drawing `form`.
fn layered(dir: &Path, content: &str, form: &str) -> (Worker, Doc) {
    let objects = [
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        format!("<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Length {} >>\nstream\n{form}\nendstream", form.len()),
    ];
    let page = " /Resources << /Font << /F1 5 0 R >> /XObject << /Fm1 6 0 R >> >>";
    let path = dir.join("layered.pdf");
    std::fs::write(&path, one_page("", page, content, &objects)).expect("writes");
    let mut worker = worker();
    let doc = worker.open(&path).expect("opens").doc;
    (worker, doc)
}

fn editable(worker: &mut Worker, doc: Doc) -> Vec<(String, bool)> {
    let blocks = worker.text_blocks(doc, 0).expect("blocks");
    blocks.into_iter().map(|b| (b.text, b.editable)).collect()
}

#[test]
fn turned_text_first_leaves_the_later_blocks_their_own_answer() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let content = "BT /F1 18 Tf 0 1 -1 0 50 500 Tm (ROTATED) Tj ET q 1 0 0 1 300 400 cm /Fm1 Do Q BT /F1 18 Tf 72 200 Td (OWN) Tj ET";
    let (mut worker, doc) = layered(dir.path(), content, "BT /F1 12 Tf 0 10 Td (STAMP) Tj ET");
    let mut got = editable(&mut worker, doc);
    got.sort();
    assert_eq!(got, [("OWN".into(), true), ("ROTATED".into(), false), ("STAMP".into(), false)]);
}

#[test]
fn blocks_that_share_a_glyph_origin_each_count_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let content = "q /Fm1 Do Q BT /F1 18 Tf 72 300 Td (BREAK) Tj ET BT /F1 18 Tf 72 700 Td (AB) Tj ET";
    let (mut worker, doc) = layered(dir.path(), content, "BT /F1 36 Tf 72 700 Td (AX) Tj ET");
    let got = editable(&mut worker, doc);
    assert!(got.contains(&("AB".into(), true)), "{got:?}");
    assert!(got.contains(&("AX".into(), false)), "{got:?}");
}
