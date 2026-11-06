//! The worker writes valid JSON numbers and strings (#66).

use std::path::Path;

use ely_pdf_engine::Worker;

/// One page: fractional and positionless targets, a square, tiny text.
fn fractional() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /Outlines 5 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Annots [7 0 R 8 0 R 10 0 R] \
         /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> >>",
        "<< /Length 38 >>\nstream\nBT /F1 0.5 Tf 0.25 700 Td (Tiny) Tj ET\nendstream",
        "<< /Type /Outlines /First 6 0 R /Last 9 0 R /Count 2 >>",
        "<< /Title (Slide\\0011) /Parent 5 0 R /Next 9 0 R /Dest [3 0 R /XYZ 0 791.5 null] >>",
        "<< /Type /Annot /Subtype /Link /Rect [0.25 0.5 100.25 20.5] /Dest [3 0 R /XYZ 0 791.75 null] >>",
        "<< /Type /Annot /Subtype /Square /Rect [0.5 0.5 10 10] >>",
        "<< /Title (Whole) /Parent 5 0 R /Prev 6 0 R /Dest [3 0 R /Fit] >>",
        "<< /Type /Annot /Subtype /Link /Rect [200 0 300 20] /Dest [3 0 R /Fit] >>",
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (number, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend(format!("{} 0 obj\n{object}\nendobj\n", number + 1).bytes());
    }
    let xref = pdf.len();
    pdf.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).bytes());
    for offset in offsets {
        pdf.extend(format!("{offset:010} 00000 n \n").bytes());
    }
    pdf.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).bytes());
    pdf
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.001
}

#[test]
fn fractions_and_control_characters_read_as_json() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("fractional.pdf");
    std::fs::write(&path, fractional()).expect("writes");
    let mut worker = Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker");
    let doc = worker.open(&path).expect("opens").doc;
    let marks = worker.outline(doc).expect("the outline reads");
    assert_eq!((marks[0].title.as_str(), marks[0].page), ("Slide\u{1}1", Some(0)));
    assert!(near(marks[0].y, 0.5), "bookmark y {}", marks[0].y);
    assert_eq!((marks[1].title.as_str(), marks[1].y), ("Whole", 0.0), "no position is the top");
    let links = worker.links(doc, 0).expect("the links read");
    assert!(near(links[0].rect[0], 0.25) && near(links[0].y, 0.25), "link {:?}", links[0]);
    assert_eq!((links[1].page, links[1].y), (Some(0), 0.0), "no position is the top");
    let drawn = worker.marks(doc, 0).expect("the annotations read");
    assert!(drawn.annots.iter().any(|mark| near(mark.rect[0], 0.5)), "square {:?}", drawn.annots);
    let blocks = worker.text_blocks(doc, 0).expect("the text reads");
    assert!(near(blocks[0].size, 0.5), "text size {}", blocks[0].size);
}

