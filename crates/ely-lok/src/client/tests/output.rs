use super::input::open;
use super::*;
use crate::review;

/// Three pages, two headings, fonts the system lacks.
const LONG: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:font-face-decls><style:font-face style:name="Nowhere Sans" svg:font-family="&apos;Nowhere Sans&apos;" style:font-family-generic="swiss"/><style:font-face style:name="Nowhere Pair" svg:font-family="&apos;Nowhere Sans;Nowhere Serif&apos;"/></office:font-face-decls>
 <office:automatic-styles>
  <style:style style:name="P1" style:family="paragraph"><style:paragraph-properties fo:break-before="page"/></style:style>
  <style:style style:name="T1" style:family="text"><style:text-properties style:font-name="Nowhere Sans"/></style:style>
  <style:style style:name="T2" style:family="text"><style:text-properties style:font-name="Nowhere Pair"/></style:style>
 </office:automatic-styles>
 <office:body><office:text>
  <text:h text:outline-level="1">First part</text:h><text:p>alpha <text:span text:style-name="T1">odd font</text:span> <text:span text:style-name="T2">pair</text:span></text:p>
  <text:h text:style-name="P1" text:outline-level="2">Second part</text:h><text:p>bravo</text:p>
  <text:p text:style-name="P1">charlie</text:p>
 </office:text></office:body>
</office:document>"#;

/// The text of a PDF, if pdftotext is present.
pub(super) fn pdf_text(path: &std::path::Path) -> Option<String> {
    let out = std::process::Command::new("pdftotext")
        .arg(path)
        .arg("-")
        .output()
        .ok()?;
    assert!(out.status.success(), "pdftotext reads {}", path.display());
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn a_locked_document_asks_for_its_password() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker =
        Worker::spawn(&install().expect("lok"), &dir.path().join("profile")).expect("the worker");
    let locked =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/client/tests/locked.odt");
    let url = crate::file_url(&locked);
    for password in ["", "wrong"] {
        let error = worker
            .unlock(&url, password)
            .expect_err("a locked load fails");
        assert_eq!(
            error.kind(),
            std::io::ErrorKind::PermissionDenied,
            "{password}: {error}"
        );
    }
    let doc = worker
        .unlock(&url, "secret")
        .expect("the right password opens it");
    let out = dir.path().join("open.fodt");
    doc.save_as(&mut worker, &crate::file_url(&out), "fodt")
        .expect("saves");
    assert!(
        std::fs::read_to_string(&out)
            .expect("reads")
            .contains("Teh cat")
    );
}

#[test]
fn pdfs_take_pages_and_the_outline_has_places() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(LONG);
    let seen = poll_until(
        &mut worker,
        &doc,
        &Events::default(),
        HANG,
        |seen| !seen.fonts.is_empty(),
    );
    let mut fonts = seen.fonts.clone();
    fonts.sort();
    assert_eq!(
        fonts,
        ["Nowhere Sans", "Nowhere Sans;Nowhere Serif"],
        "a declared list is named too"
    );
    let outline = review::outline(&worker.values(&doc, ".uno:Outline").expect("the outline"))
        .expect("the outline parses");
    let texts: Vec<_> = outline
        .iter()
        .map(|heading| (heading.text.as_str(), heading.level))
        .collect();
    assert_eq!(texts, [("First part", 0), ("Second part", 1)]);
    assert!(
        outline[0].current && !outline[1].current,
        "the cursor sits under the first"
    );
    assert!(
        outline[1].rect.1 > outline[0].rect.1,
        "the second heading sits lower"
    );
    let pdf = dir.path().join("two.pdf");
    let only_two = crate::Pdf {
        pages: Some((2, 2)),
        ..crate::Pdf::default()
    };
    doc.export_pdf(&mut worker, &crate::file_url(&pdf), &only_two)
        .expect("the export works");
    let backwards = crate::Pdf {
        pages: Some((3, 2)),
        ..crate::Pdf::default()
    };
    assert!(
        doc.export_pdf(&mut worker, &crate::file_url(&pdf), &backwards)
            .is_err()
    );
    let Some(text) = pdf_text(&pdf) else {
        eprintln!("SKIP: no pdftotext to read the PDF");
        return;
    };
    assert!(
        text.contains("bravo") && !text.contains("alpha") && !text.contains("charlie"),
        "{text}"
    );
}

#[test]
fn comments_and_changes_print_as_chosen() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker
        .run(&doc, "comment.insert", &["Marginal"])
        .expect("a comment");
    worker.run(&doc, "track.toggle", &[]).expect("tracking");
    for _ in 0.."Hello".len() {
        worker.post_key(&doc, 0, 1286).expect("delete");
    }
    let mut texts = Vec::new();
    for (n, (comments, changes)) in [(false, false), (true, true)].into_iter().enumerate() {
        let pdf = dir.path().join(format!("{n}.pdf"));
        let options = crate::Pdf {
            pages: None,
            comments,
            changes,
            handouts: 0,
        };
        doc.export_pdf(&mut worker, &crate::file_url(&pdf), &options)
            .expect("the export works");
        let Some(text) = pdf_text(&pdf) else {
            eprintln!("SKIP: no pdftotext to read the PDF");
            return;
        };
        texts.push(text);
    }
    assert!(
        !texts[0].contains("Marginal"),
        "comments stay out: {}",
        texts[0]
    );
    assert!(
        texts[1].contains("Marginal"),
        "comments go in the margin: {}",
        texts[1]
    );
    assert!(
        !texts[0].contains("Hello"),
        "the final text drops the deletion"
    );
    assert!(texts[1].contains("Hello"), "marked changes keep it");
}

#[test]
fn a_bookmark_knows_its_place() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open(LONG);
    let pages = doc.pages(&mut worker).expect("pages");
    click(&mut worker, &doc, pages[2].0 + 1500, pages[2].1 + 1500);
    worker
        .run(&doc, "bookmark.insert", &["Far"])
        .expect("a mark");
    let marks = review::bookmarks(&worker.values(&doc, ".uno:Bookmarks").expect("the marks"))
        .expect("the marks parse");
    let place = marks[0].rect.expect("the mark has a place");
    assert!(place.1 > pages[2].1, "the mark sits on page three");
}
