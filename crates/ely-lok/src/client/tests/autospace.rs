//! PowerPoint puts no gap between Asian and Latin text (#83).

use super::*;

const DECK: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:body><office:presentation><draw:page draw:name="p1">
  <draw:frame svg:x="1cm" svg:y="1cm" svg:width="20cm" svg:height="3cm"><draw:text-box><text:p>北京市2025年</text:p></draw:text-box></draw:frame>
 </draw:page></office:presentation></office:body>
</office:document>"#;

/// `source` loaded and saved as flat ODF.
fn as_flat(worker: &mut Worker, dir: &std::path::Path, source: &str, name: &str) -> String {
    let doc = worker.load(&crate::file_url(&dir.join(source))).expect("the deck loads");
    let out = dir.join(name);
    doc.save_as(worker, &crate::file_url(&out), "fodp").expect("the deck saves");
    worker.close_doc(doc).expect("the deck closes");
    std::fs::read_to_string(&out).expect("the save reads")
}

#[test]
fn powerpoint_decks_put_no_gap_between_asian_and_latin_text() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    write(&dir.path().join("in.fodp"), DECK);
    let doc = worker.load(&crate::file_url(&dir.path().join("in.fodp"))).expect("the deck loads");
    for (name, filter) in [("in.pptx", "pptx"), ("in.ppt", "ppt")] {
        doc.save_as(&mut worker, &crate::file_url(&dir.path().join(name)), filter).expect("the deck saves");
    }
    worker.close_doc(doc).expect("the deck closes");
    let odf = as_flat(&mut worker, dir.path(), "in.fodp", "odf.fodp");
    assert!(odf.contains(r#"style:text-autospace="ideograph-alpha""#), "ODF keeps its gap");
    for source in ["in.pptx", "in.ppt"] {
        let xml = as_flat(&mut worker, dir.path(), source, "out.fodp");
        assert!(xml.contains(r#"style:text-autospace="none""#), "{source} drew a gap");
        assert!(!xml.contains("ideograph-alpha"), "{source} kept a gap somewhere");
    }
}
