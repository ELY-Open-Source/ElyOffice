//! Saves after an edit stay near the original size.

use std::path::{Path, PathBuf};

use ely_pdf_engine::{Digest, Doc, FieldKind, NewAnnot, NewField, Region, Shape, Signing, Worker, Writing, Wrote};

/// Bytes a small edit may add.
const SMALL: u64 = 2_000;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/pdf/sizes.pdf")
}

fn worker() -> Worker {
    Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker")
}

/// A 2000 by 1500 gradient PNM and its raw size.
fn photo(dir: &Path) -> (PathBuf, u64) {
    let (w, h) = (2000u32, 1500u32);
    let mut bytes = format!("P6\n{w} {h}\n255\n").into_bytes();
    for y in 0..h {
        for x in 0..w {
            bytes.extend([(x / 8) as u8, (y / 6) as u8, ((x + y) / 14) as u8]);
        }
    }
    let path = dir.join("photo.pnm");
    std::fs::write(&path, bytes).expect("writes");
    (path, u64::from(w * h * 3))
}

/// The fixture edited by `edit` and saved.
fn saved(dir: &Path, name: &str, edit: impl FnOnce(&mut Worker, Doc) -> Doc) -> PathBuf {
    let path = dir.join(format!("{name}-in.pdf"));
    std::fs::copy(fixture(), &path).expect("copies");
    let mut worker = worker();
    let doc = worker.open(&path).expect("opens").doc;
    let doc = edit(&mut worker, doc);
    let out = dir.join(format!("{name}.pdf"));
    worker.save(doc, &out).expect("saves");
    out
}

fn size(path: &Path) -> u64 {
    std::fs::metadata(path).expect("the file").len()
}

fn within(out: &Path, room: u64) {
    let (was, now) = (size(&fixture()), size(out));
    assert!(now <= was + room, "{}: {was} grew to {now}", out.display());
}

fn put(worker: &mut Worker, doc: Doc, w: &Writing) -> Doc {
    let wrote = worker.put_text(doc, 0, w, 0.0).expect("writes");
    assert!(matches!(wrote, Wrote::Done(_)), "{wrote:?}");
    worker.now(doc).expect("the state")
}

fn write(font: &'static str, text: &'static str) -> impl FnOnce(&mut Worker, Doc) -> Doc {
    move |worker, doc| {
        let w = Writing { block: None, expected: String::new(), text: text.into(), font: font.into(), size: 12.0, rgb: 0, bx: [72.0, 72.0, 300.0] };
        put(worker, doc, &w)
    }
}

/// Compressed streams' raw bytes, sorted.
fn compressed(path: &Path) -> Vec<Vec<u8>> {
    let pdf = lopdf::Document::load(path).expect("loads");
    let streams = pdf.objects.values().filter_map(|o| o.as_stream().ok());
    let mut all: Vec<Vec<u8>> = streams.filter(|s| s.dict.has(b"Filter")).map(|s| s.content.clone()).collect();
    all.sort();
    all
}

#[test]
fn a_plain_save_copies_every_compressed_stream() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let out = saved(dir.path(), "plain", |_, doc| doc);
    within(&out, 0);
    assert_eq!(compressed(&out), compressed(&fixture()));
}

#[test]
fn text_adds_little() {
    let dir = tempfile::tempdir().expect("a temp dir");
    within(&saved(dir.path(), "latin", write("helvetica", "Added")), SMALL);
    within(&saved(dir.path(), "cjk", write("cjk-sc", "加了字")), SMALL);
    let out = saved(dir.path(), "edit", |worker, doc| {
        let read = worker.text_blocks(doc, 0).expect("blocks")[0].clone();
        let text = format!("{} edited", read.text.split('.').next().expect("a sentence"));
        let bx = [read.rect[0], read.rect[1], read.rect[2] - read.rect[0]];
        let w = Writing { block: Some(0), expected: read.text, text, font: "original".into(), size: read.size, rgb: read.rgb, bx };
        put(worker, doc, &w)
    });
    within(&out, SMALL);
}

#[test]
fn marks_and_pages_add_little() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let out = saved(dir.path(), "annot", |worker, doc| {
        let shape = Shape::Highlight(vec![[72.0, 72.0, 200.0, 90.0]]);
        worker.annotate(doc, &[NewAnnot { page: 0, shape, rgb: None, contents: String::new(), author: "t".into() }]).expect("annotates");
        worker.now(doc).expect("the state")
    });
    within(&out, SMALL);
    let out = saved(dir.path(), "blank", |worker, doc| {
        worker.insert_blank(doc, 1, (612.0, 792.0)).expect("inserts");
        worker.now(doc).expect("the state")
    });
    within(&out, SMALL);
    let out = saved(dir.path(), "delete", |worker, doc| {
        worker.delete_pages(doc, &[1]).expect("deletes");
        worker.now(doc).expect("the state")
    });
    within(&out, 0);
}

#[test]
fn a_filled_form_adds_little() {
    let dir = tempfile::tempdir().expect("a temp dir");
    for (name, value) in [("form", "Zoe Smith"), ("form-cjk", "张三")] {
        let out = saved(dir.path(), name, |worker, doc| {
            let field = NewField { kind: FieldKind::Text, name: "Name".into(), rect: [72.0, 300.0, 272.0, 320.0], options: Vec::new() };
            worker.add_field(doc, 0, &field).expect("adds");
            let doc = worker.now(doc).expect("the state");
            let id = worker.marks(doc, 0).expect("marks").fields[0].id;
            worker.fill(doc, 0, id, value).expect("fills");
            worker.now(doc).expect("the state")
        });
        eprintln!("{name}: {}", size(&out));
        within(&out, SMALL);
    }
}

#[test]
fn a_redaction_adds_little() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let red = dir.path().join("red.pdf");
    let out = saved(dir.path(), "redact", |worker, doc| {
        worker.redact(doc, &[Region { page: 0, rect: [72.0, 72.0, 300.0, 100.0] }], &red).expect("redacts");
        worker.open(&red).expect("opens").doc
    });
    within(&red, SMALL);
    within(&out, SMALL);
}

#[test]
fn a_signature_embeds_its_glyphs_and_reads_back() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let out = saved(dir.path(), "script", write("script", "Signed"));
    // The whole face added 228 KB.
    within(&out, 8_000);
    let mut worker = worker();
    let doc = worker.open(&out).expect("opens").doc;
    let blocks = worker.text_blocks(doc, 0).expect("blocks");
    assert!(blocks.iter().any(|b| b.text.contains("Signed")), "{blocks:?}");
}

#[test]
fn an_added_photo_is_compressed() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (photo, raw) = photo(dir.path());
    let added = saved(dir.path(), "image", |worker, doc| {
        worker.add_image(doc, 0, &photo, [72.0, 300.0, 272.0, 450.0]).expect("adds");
        worker.now(doc).expect("the state")
    });
    let replaced = saved(dir.path(), "replace", |worker, doc| {
        let last = worker.objects(doc, 0).expect("objects").len() - 1;
        worker.replace_image(doc, 0, last, &photo).expect("replaces");
        worker.now(doc).expect("the state")
    });
    for out in [added, replaced] {
        let grew = size(&out).saturating_sub(size(&fixture()));
        assert!(grew < raw / 4, "{} grew {grew} for {raw} raw bytes", out.display());
    }
}

/// Signs `doc` into `out` and checks the signature.
fn sign(worker: &mut Worker, doc: Doc, out: &Path, reason: &str, location: &str) {
    let pfx = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/pdf/signer.p12");
    let how = Signing {
        page: 0,
        object: None,
        rect: [72.0, 600.0, 272.0, 660.0],
        name: "Signature1".into(),
        pfx: pfx.to_str().expect("UTF-8").into(),
        pfx_password: "ely".into(),
        reason: reason.into(),
        location: location.into(),
    };
    worker.sign(doc, out, &how).expect("signs");
    eprintln!("signed {reason} {location}: {}", size(out));
    let mut other = self::worker();
    let read = other.open(out).expect("opens").doc;
    let signatures = other.signatures(read, out).expect("reads");
    let signed = signatures.first().and_then(|s| s.signed.as_ref()).expect("signed");
    assert_eq!((signed.digest, signed.later, signed.broken), (Digest::Intact, false, false));
    assert_eq!((signed.reason.as_str(), signed.location.as_str()), (reason, location));
}

fn opened(dir: &Path) -> (Worker, Doc) {
    let path = dir.join("in.pdf");
    std::fs::copy(fixture(), &path).expect("copies");
    let mut worker = worker();
    let doc = worker.open(&path).expect("opens").doc;
    (worker, doc)
}

#[test]
fn a_signed_copy_of_an_edited_file_is_compressed() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (photo, raw) = photo(dir.path());
    let (mut worker, doc) = opened(dir.path());
    worker.add_image(doc, 0, &photo, [72.0, 300.0, 272.0, 450.0]).expect("adds");
    let doc = worker.now(doc).expect("the state");
    let out = dir.path().join("signed.pdf");
    sign(&mut worker, doc, &out, "Approved", "Here");
    let grew = size(&out).saturating_sub(size(&fixture()));
    assert!(grew < raw / 4, "grew {grew} for {raw} raw bytes");
}

#[test]
fn signatures_in_cjk_embed_a_subset() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(dir.path());
    let latin = dir.path().join("latin.pdf");
    sign(&mut worker, doc, &latin, "Approved", "Here");
    // Each once cost the whole 23.6 MB face.
    let reasons = [("批准签署", "서울"), ("日本語を承認しました", "Here"), ("核准簽署文件", "臺北"), ("서명했습니다", "부산"), ("承認済み・確認ずみ", "東京都")];
    for (i, (reason, location)) in reasons.into_iter().enumerate() {
        let out = dir.path().join(format!("cjk-{i}.pdf"));
        sign(&mut worker, doc, &out, reason, location);
        let (was, now) = (size(&latin), size(&out));
        assert!(now <= was + 500_000, "{reason}: {was} grew to {now}");
    }
}

#[test]
fn two_signature_texts_carry_their_own_subset_tags() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let out = saved(dir.path(), "two", |worker, doc| {
        let doc = write("script", "Ada Lovelace")(worker, doc);
        let w = Writing { block: None, expected: String::new(), text: "Grace Hopper".into(), font: "script".into(), size: 12.0, rgb: 0, bx: [72.0, 200.0, 300.0] };
        put(worker, doc, &w)
    });
    let pdf = lopdf::Document::load(&out).expect("loads");
    let name = |dict: &lopdf::Dictionary, key: &[u8]| String::from_utf8(dict.get(key).and_then(lopdf::Object::as_name).expect("a name").to_vec()).expect("UTF-8");
    let mut tags = Vec::new();
    for dict in pdf.objects.values().filter_map(|o| o.as_dict().ok()) {
        if dict.get(b"Subtype").and_then(lopdf::Object::as_name).ok() != Some(b"Type0".as_slice()) {
            continue;
        }
        let base = name(dict, b"BaseFont");
        let kids = dict.get(b"DescendantFonts").and_then(lopdf::Object::as_array).expect("descendants");
        let kid = pdf.dereference(&kids[0]).expect("the child").1.as_dict().expect("a dict");
        let descriptor = pdf.dereference(kid.get(b"FontDescriptor").expect("a descriptor")).expect("it").1.as_dict().expect("a dict");
        assert_eq!((name(kid, b"BaseFont"), name(descriptor, b"FontName")), (base.clone(), base.clone()));
        let (tag, rest) = base.split_at(7);
        assert!(tag.bytes().take(6).all(|b| b.is_ascii_uppercase()) && tag.ends_with('+'), "{base}");
        assert_eq!(rest, "GreatVibes");
        tags.push(tag.to_string());
    }
    assert_eq!(tags.len(), 2, "{tags:?}");
    assert_ne!(tags[0], tags[1]);
}

#[test]
fn signing_leaves_two_identical_old_fonts_apart() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let face = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/GreatVibes-Regular.ttf")).expect("the font");
    let hex: String = face.iter().map(|b| format!("{b:02x}")).collect::<String>() + ">";
    let font = |desc: u32| format!("<< /Type /Font /Subtype /TrueType /BaseFont /GreatVibes /FontDescriptor {desc} 0 R >>");
    let descriptor = |file: u32| format!("<< /Type /FontDescriptor /FontName /GreatVibes /Flags 32 /FontBBox [0 -300 1300 900] /ItalicAngle 0 /Ascent 900 /Descent -300 /CapHeight 700 /StemV 80 /FontFile2 {file} 0 R >>");
    let file = format!("<< /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}\nendstream", hex.len());
    let objects = [font(7), font(8), descriptor(9), descriptor(10), file.clone(), file];
    let content = "BT /F1 18 Tf 72 700 Td (Old one) Tj ET BT /F2 18 Tf 72 650 Td (Old two) Tj ET";
    let path = dir.path().join("two.pdf");
    std::fs::write(&path, ely_pdf_engine::sample::one_page("", " /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >>", content, &objects)).expect("writes");
    let mut worker = worker();
    let doc = worker.open(&path).expect("opens").doc;
    let out = dir.path().join("signed.pdf");
    sign(&mut worker, doc, &out, "Approved", "Here");
    let (old, new) = (lopdf::Document::load(&path).expect("loads"), lopdf::Document::load(&out).expect("loads"));
    for id in 5..=10 {
        assert_eq!(old.get_object((id, 0)).expect("old"), new.get_object((id, 0)).expect("kept"), "object {id}");
    }
}
