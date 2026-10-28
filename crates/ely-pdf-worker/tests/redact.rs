//! Removal and cleaning, checked in the file (QA-14).

use std::path::Path;

use ely_pdf_engine::{
    Cleaning, Doc, NewAnnot, OcrLine, Region, Shape, Worker,
    sample::{Extras, pdf_with},
};

fn worker() -> Worker {
    Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker")
}

/// Two lines, a picture at 72..172 across, 242..292 down.
fn opened(path: &Path) -> (Worker, Doc) {
    let extras = Extras { picture: true, ..Extras::default() };
    std::fs::write(path, pdf_with(&[(612.0, 792.0, &["Hello world", "Second line"])], &[], &extras)).expect("writes");
    let mut worker = worker();
    let doc = worker.open(path).expect("opens").doc;
    (worker, doc)
}

fn words(worker: &mut Worker, doc: Doc) -> String {
    worker.text(doc, 0).expect("text").iter().map(|c| c.c).collect()
}

/// The box of the first `n` characters.
fn first(worker: &mut Worker, doc: Doc, n: usize) -> [f32; 4] {
    let chars = worker.text(doc, 0).expect("text");
    let (a, b) = (chars[0].rect, chars[n - 1].rect);
    [a[0] - 1.0, a[1] - 1.0, b[2] + 0.5, b[3] + 1.0]
}

fn reopen(path: &Path) -> (Worker, Doc) {
    let mut worker = worker();
    let doc = worker.open(path).expect("opens").doc;
    (worker, doc)
}

#[test]
fn removed_text_is_gone_from_the_file_and_the_rest_stays() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&dir.path().join("doc.pdf"));
    let hello = first(&mut worker, doc, 5);
    let line = OcrLine { rect: hello, text: "Hidden".into() };
    worker.text_layer(doc, &[(0, vec![line])], false).expect("an invisible layer over it");
    doc = worker.now(doc).expect("the state");
    let mark = worker.history(doc).expect("a mark");
    let out = dir.path().join("out.pdf");
    let report = worker.redact(doc, &[Region { page: 0, rect: hello }], &out).expect("removes");
    assert_eq!((report.text, report.light, report.versions), (0, 0, 1));
    assert_eq!(worker.history(doc).expect("a mark"), mark, "the open document is untouched");
    assert!(words(&mut worker, doc).contains("Hello"));
    let (mut other, written) = reopen(&out);
    let left = words(&mut other, written);
    assert!(!left.contains("Hello") && !left.contains("Hidden"), "{left}");
    assert!(left.contains("world") && left.contains("Second line"), "{left}");
    let bytes = std::fs::read(&out).expect("reads");
    assert!(!bytes.windows(5).any(|w| w == b"Hello"), "no trace in the bytes");
    assert_eq!(bytes.windows(9).filter(|w| w == b"startxref").count(), 1, "one revision");
}

#[test]
fn a_picture_under_a_region_loses_those_pixels() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&dir.path().join("doc.pdf"));
    let out = dir.path().join("out.pdf");
    worker.redact(doc, &[Region { page: 0, rect: [72.0, 242.0, 122.0, 292.0] }], &out).expect("removes");
    let (mut other, written) = reopen(&out);
    let left = other.render(written, 0, (1.0, 0), [124, 244, 46, 46]).expect("draws").bgra;
    assert!(left.chunks_exact(4).any(|p| p[0] > 100 || p[1] > 100 || p[2] > 100), "the right half keeps its colour");
    let gone = other.render(written, 0, (1.0, 0), [74, 244, 46, 46]).expect("draws").bgra;
    assert!(gone.chunks_exact(4).all(|p| p[0] < 40 && p[1] < 40 && p[2] < 40), "the left half is black");
}

#[test]
fn notes_in_a_region_go_and_others_stay() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&dir.path().join("doc.pdf"));
    let note = |at| NewAnnot { page: 0, shape: Shape::Note(at), rgb: None, contents: "secret".into(), author: String::new() };
    worker.annotate(doc, &[note((80.0, 400.0)), note((400.0, 600.0))]).expect("notes");
    doc = worker.now(doc).expect("the state");
    let out = dir.path().join("out.pdf");
    worker.redact(doc, &[Region { page: 0, rect: [70.0, 390.0, 120.0, 430.0] }], &out).expect("removes");
    let (mut other, written) = reopen(&out);
    let kept = other.marks(written, 0).expect("marks").annots;
    assert_eq!(kept.len(), 1, "the far note stays");
    assert!(kept[0].rect[0] > 300.0);
}

#[test]
fn bad_regions_write_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&dir.path().join("doc.pdf"));
    let out = dir.path().join("out.pdf");
    assert!(worker.redact(doc, &[], &out).is_err());
    assert!(worker.redact(doc, &[Region { page: 4, rect: [0.0, 0.0, 10.0, 10.0] }], &out).is_err());
    assert!(!out.exists());
}

#[test]
fn cleaning_reaches_what_it_counted_and_leaves_none() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&dir.path().join("doc.pdf"));
    worker.set_properties(doc, &[("Title", "Secret plan"), ("Author", "Zoe")]).expect("sets");
    doc = worker.now(doc).expect("the state");
    let note = NewAnnot { page: 0, shape: Shape::Note((300.0, 300.0)), rgb: None, contents: "x".into(), author: String::new() };
    worker.annotate(doc, &[note]).expect("notes");
    doc = worker.now(doc).expect("the state");
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "attached").expect("writes");
    worker.attach(doc, "a.txt", &file).expect("attaches");
    doc = worker.now(doc).expect("the state");
    worker.text_layer(doc, &[(0, vec![OcrLine { rect: [72.0, 600.0, 200.0, 620.0], text: "invisible".into() }])], false).expect("lays");
    doc = worker.now(doc).expect("the state");
    let reach = worker.clean_scan(doc, &dir.path().join("scan.pdf")).expect("scans");
    assert!(reach.meta >= 2 && reach.annots == 1 && reach.files == 1 && reach.hidden == 1, "{reach:?}");
    let out = dir.path().join("out.pdf");
    let all = Cleaning { meta: true, annots: true, files: true, hidden: true };
    let left = worker.clean(doc, all, &out).expect("cleans");
    assert_eq!((left.meta, left.annots, left.files, left.hidden, left.scripts), (0, 0, 0, 0, 0));
    let (mut other, written) = reopen(&out);
    assert_eq!(other.properties(written).expect("reads").title, None);
    assert!(other.attachments(written).expect("lists").is_empty());
    assert!(!words(&mut other, written).contains("invisible"));
    assert!(words(&mut other, written).contains("Hello world"), "seen text stays");
    assert_eq!(worker.clean_scan(doc, &dir.path().join("scan.pdf")).expect("scans"), reach, "the open document is untouched");
}

#[test]
fn cleaning_one_kind_leaves_the_others() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&dir.path().join("doc.pdf"));
    worker.set_properties(doc, &[("Title", "Plan")]).expect("sets");
    doc = worker.now(doc).expect("the state");
    let note = NewAnnot { page: 0, shape: Shape::Note((300.0, 300.0)), rgb: None, contents: "x".into(), author: String::new() };
    worker.annotate(doc, &[note]).expect("notes");
    doc = worker.now(doc).expect("the state");
    let out = dir.path().join("out.pdf");
    let left = worker.clean(doc, Cleaning { annots: true, ..Cleaning::default() }, &out).expect("cleans");
    assert_eq!(left.annots, 0);
    assert!(left.meta >= 1, "properties were not asked for");
    assert!(worker.clean(doc, Cleaning::default(), &dir.path().join("none.pdf")).is_err(), "nothing chosen");
}

/// Helvetica as object 5, for the hand-made pages.
fn helvetica() -> String {
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into()
}

fn written(dir: &Path, bytes: Vec<u8>) -> (Worker, Doc) {
    let path = dir.join("doc.pdf");
    std::fs::write(&path, bytes).expect("writes");
    reopen(&path)
}

fn holds(path: &Path, needle: &str) -> bool {
    std::fs::read(path).expect("reads").windows(needle.len()).any(|w| w == needle.as_bytes())
}

#[test]
fn a_shape_touching_a_region_goes_whole() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = written(dir.path(), ely_pdf_engine::sample::one_page("", "", "1 0 0 rg 100 700 200 40 re f", &[]));
    let out = dir.path().join("out.pdf");
    let report = worker.redact(doc, &[Region { page: 0, rect: [90.0, 45.0, 180.0, 100.0] }], &out).expect("removes");
    assert_eq!(report.objects, 0);
    let (mut other, read) = reopen(&out);
    let right = other.render(read, 0, (1.0, 0), [200, 55, 90, 30]).expect("draws").bgra;
    assert!(right.chunks_exact(4).all(|p| p[2] < 200 || p[1] > 200), "no red left past the box");
}

#[test]
fn a_field_in_a_region_takes_its_parent_and_value() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let objects = [
        "<< /FT /Tx /T (parent) /V (SECRET-FIELD-VALUE) /Kids [6 0 R] >>".to_string(),
        "<< /Type /Annot /Subtype /Widget /Parent 5 0 R /Rect [70 700 220 740] /P 3 0 R >>".to_string(),
    ];
    let (mut worker, doc) = written(dir.path(), ely_pdf_engine::sample::one_page(" /AcroForm << /Fields [5 0 R] >>", " /Annots [6 0 R]", "", &objects));
    let out = dir.path().join("out.pdf");
    worker.redact(doc, &[Region { page: 0, rect: [60.0, 40.0, 230.0, 100.0] }], &out).expect("removes");
    assert!(!holds(&out, "SECRET-FIELD-VALUE"));
}

#[test]
fn hidden_layers_faint_text_and_hidden_notes_clean_out() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let content = "BT /F1 12 Tf 72 700 Td (VISIBLE) Tj ET \
        /OC /H BDC BT /F1 12 Tf 72 680 Td (SECRET-OFF-LAYER) Tj ET EMC \
        q /G0 gs BT /F1 12 Tf 72 660 Td (SECRET-FAINT) Tj ET Q";
    let objects = [
        helvetica(),
        "<< /Type /OCG /Name (Hidden) >>".to_string(),
        "<< /Type /ExtGState /ca 0 >>".to_string(),
        "<< /Type /Annot /Subtype /Text /F 2 /Rect [300 300 320 320] /Contents (SECRET-HIDDEN-NOTE) >>".to_string(),
    ];
    let catalog = " /OCProperties << /OCGs [6 0 R] /D << /OFF [6 0 R] >> >>";
    let page = " /Resources << /Font << /F1 5 0 R >> /Properties << /H 6 0 R >> /ExtGState << /G0 7 0 R >> >> /Annots [8 0 R]";
    let (mut worker, doc) = written(dir.path(), ely_pdf_engine::sample::one_page(catalog, page, content, &objects));
    let scratch = dir.path().join("scan.pdf");
    let reach = worker.clean_scan(doc, &scratch).expect("scans");
    assert_eq!((reach.hidden, reach.hidden_annots, reach.annots), (1, 1, 0), "{reach:?}");
    let out = dir.path().join("out.pdf");
    worker.clean(doc, Cleaning { hidden: true, ..Cleaning::default() }, &out).expect("cleans without /Names");
    for secret in ["SECRET-OFF-LAYER", "SECRET-FAINT", "SECRET-HIDDEN-NOTE"] {
        assert!(!holds(&out, secret), "{secret}");
    }
    assert!(holds(&out, "VISIBLE"));
}

#[test]
fn metadata_on_pages_cleans_out_too() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let xmp = "<x:xmpmeta>SECRET-PAGE-METADATA</x:xmpmeta>";
    let objects = [format!("<< /Type /Metadata /Subtype /XML /Length {} >>\nstream\n{xmp}\nendstream", xmp.len())];
    let (mut worker, doc) = written(dir.path(), ely_pdf_engine::sample::one_page("", " /Metadata 5 0 R", "", &objects));
    let reach = worker.clean_scan(doc, &dir.path().join("scan.pdf")).expect("scans");
    assert_eq!(reach.meta, 1);
    let out = dir.path().join("out.pdf");
    worker.clean(doc, Cleaning { meta: true, ..Cleaning::default() }, &out).expect("cleans");
    assert!(!holds(&out, "SECRET-PAGE-METADATA"));
}

#[test]
fn marked_content_says_nothing_after_removal() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let content = "/Span << /MCID 0 /ActualText (SECRET-ACTUAL) >> BDC BT /F1 12 Tf 72 700 Td (Hello) Tj ET EMC";
    let objects = [
        helvetica(),
        "<< /Type /StructTreeRoot /K [7 0 R] >>".to_string(),
        "<< /Type /StructElem /S /P /P 6 0 R /Pg 3 0 R /K 0 /Alt (SECRET-STRUCT) /E (SECRET-EXPANDED) >>".to_string(),
    ];
    let (mut worker, doc) = written(dir.path(), ely_pdf_engine::sample::one_page(" /StructTreeRoot 6 0 R", " /Resources << /Font << /F1 5 0 R >> >>", content, &objects));
    let out = dir.path().join("out.pdf");
    worker.redact(doc, &[Region { page: 0, rect: [60.0, 80.0, 200.0, 100.0] }], &out).expect("removes");
    for secret in ["SECRET-ACTUAL", "SECRET-STRUCT", "SECRET-EXPANDED", "Hello"] {
        assert!(!holds(&out, secret), "{secret}");
    }
}

#[test]
fn overlapping_regions_both_remove() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&dir.path().join("doc.pdf"));
    let hello = first(&mut worker, doc, 5);
    let after = [hello[2] - 2.0, hello[1], hello[2] + 60.0, hello[3]];
    let out = dir.path().join("out.pdf");
    worker.redact(doc, &[Region { page: 0, rect: hello }, Region { page: 0, rect: after }], &out).expect("removes");
    let (mut other, read) = reopen(&out);
    assert!(!words(&mut other, read).contains("Hello"));
}

#[test]
fn nested_metadata_and_associated_files_clean_out() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let secret = "SECRET-AF-BYTES";
    let objects = [
        "<< /Type /Filespec /F (a.txt) /EF << /F 6 0 R >> >>".to_string(),
        format!("<< /Type /EmbeddedFile /Length {} >>\nstream\n{secret}\nendstream", secret.len()),
    ];
    let page = " /Extra << /Deep << /PieceInfo << /App << /Private (SECRET-PIECE) >> >> >> >>";
    let (mut worker, doc) = written(dir.path(), ely_pdf_engine::sample::one_page(" /AF [5 0 R]", page, "", &objects));
    let reach = worker.clean_scan(doc, &dir.path().join("scan.pdf")).expect("scans");
    assert_eq!((reach.meta, reach.files), (1, 1), "{reach:?}");
    let out = dir.path().join("out.pdf");
    worker.clean(doc, Cleaning { meta: true, files: true, ..Cleaning::default() }, &out).expect("cleans");
    assert!(!holds(&out, secret) && !holds(&out, "SECRET-PIECE") && !holds(&out, "/AF"));
}

#[test]
fn scripts_on_links_fields_and_chains_clean_out() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let objects = [
        "<< /Type /Annot /Subtype /Link /Rect [72 600 200 620] /A << /S /JavaScript /JS (SECRET-LINK-JS) >> >>".to_string(),
        "<< /FT /Tx /T (p) /AA << /K << /S /JavaScript /JS (SECRET-FIELD-JS) >> >> /Kids [7 0 R] >>".to_string(),
        "<< /Type /Annot /Subtype /Widget /Parent 6 0 R /Rect [72 500 200 520] /P 3 0 R >>".to_string(),
    ];
    let catalog = " /AcroForm << /Fields [6 0 R] >> /OpenAction << /S /GoTo /D [3 0 R /Fit] \
        /Next << /S /JavaScript /JS (SECRET-NEXT-JS) /Next << /S /Named /N /LastPage >> >> >>";
    let (mut worker, doc) = written(dir.path(), ely_pdf_engine::sample::one_page(catalog, " /Annots [5 0 R 7 0 R]", "", &objects));
    let reach = worker.clean_scan(doc, &dir.path().join("scan.pdf")).expect("scans");
    assert_eq!(reach.scripts, 3, "{reach:?}");
    let out = dir.path().join("out.pdf");
    worker.clean(doc, Cleaning { hidden: true, ..Cleaning::default() }, &out).expect("cleans");
    for secret in ["SECRET-LINK-JS", "SECRET-FIELD-JS", "SECRET-NEXT-JS"] {
        assert!(!holds(&out, secret), "{secret}");
    }
    assert!(holds(&out, "/LastPage") && holds(&out, "/GoTo"), "what follows a script stays");
}

#[test]
fn elements_placed_by_their_kids_say_nothing_after_removal() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let content = "/Span << /MCID 0 >> BDC BT /F1 12 Tf 72 700 Td (Hello) Tj ET EMC";
    let objects = [
        helvetica(),
        "<< /Type /StructTreeRoot /K [7 0 R] >>".to_string(),
        "<< /Type /StructElem /S /P /P 6 0 R /K << /Type /MCR /Pg 3 0 R /MCID 0 >> /ActualText (SECRET-MCR) >>".to_string(),
    ];
    let (mut worker, doc) = written(dir.path(), ely_pdf_engine::sample::one_page(" /StructTreeRoot 6 0 R", " /Resources << /Font << /F1 5 0 R >> >>", content, &objects));
    let out = dir.path().join("out.pdf");
    worker.redact(doc, &[Region { page: 0, rect: [60.0, 80.0, 200.0, 100.0] }], &out).expect("removes");
    assert!(!holds(&out, "SECRET-MCR"));
}

#[test]
fn a_redacted_page_loses_its_thumbnail() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let pixels = "SECRET-THUMB";
    let objects = [format!("<< /Type /XObject /Subtype /Image /Width 4 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length {} >>\nstream\n{pixels}\nendstream", pixels.len())];
    let page = " /Resources << /XObject << /Im 5 0 R >> >> /Thumb 5 0 R";
    let (mut worker, doc) = written(dir.path(), ely_pdf_engine::sample::one_page("", page, "q 100 0 0 50 0 742 cm /Im Do Q", &objects));
    let out = dir.path().join("out.pdf");
    worker.redact(doc, &[Region { page: 0, rect: [0.0, 0.0, 100.0, 50.0] }], &out).expect("removes");
    assert!(!holds(&out, pixels) && !holds(&out, "/Thumb"));
}

#[test]
fn hidden_text_takes_what_its_marks_say() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let content = "/Span << /ActualText (SECRET-HIDDEN-ACTUAL) >> BDC BT /F1 12 Tf 3 Tr 72 700 Td (Unseen) Tj ET EMC";
    let (mut worker, doc) = written(dir.path(), ely_pdf_engine::sample::one_page("", " /Resources << /Font << /F1 5 0 R >> >>", content, &[helvetica()]));
    let out = dir.path().join("out.pdf");
    worker.clean(doc, Cleaning { hidden: true, ..Cleaning::default() }, &out).expect("cleans");
    assert!(!holds(&out, "SECRET-HIDDEN-ACTUAL") && !holds(&out, "Unseen"));
}

#[test]
fn hidden_links_and_fields_clean_out() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let objects = [
        "<< /Type /Annot /Subtype /Link /F 2 /Rect [72 600 200 620] /Contents (SECRET-HIDDEN-LINK) /A << /S /URI /URI (https://x.test/SECRET-HIDDEN-LINK) >> >>".to_string(),
        "<< /Type /Annot /Subtype /Widget /F 2 /FT /Tx /T (f) /V (SECRET-HIDDEN-WIDGET) /Rect [72 500 200 520] /P 3 0 R >>".to_string(),
    ];
    let (mut worker, doc) = written(dir.path(), ely_pdf_engine::sample::one_page(" /AcroForm << /Fields [6 0 R] >>", " /Annots [5 0 R 6 0 R]", "", &objects));
    let reach = worker.clean_scan(doc, &dir.path().join("scan.pdf")).expect("scans");
    assert_eq!(reach.hidden_annots, 2, "{reach:?}");
    let out = dir.path().join("out.pdf");
    worker.clean(doc, Cleaning { hidden: true, ..Cleaning::default() }, &out).expect("cleans");
    assert!(!holds(&out, "SECRET-HIDDEN-LINK") && !holds(&out, "SECRET-HIDDEN-WIDGET"));
}
