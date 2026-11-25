use super::input::open;
use super::*;

/// A heading, a styled line, a list and a table.
const NOTES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:automatic-styles>
  <style:style style:name="B" style:family="text"><style:text-properties fo:font-weight="bold"/></style:style>
 </office:automatic-styles>
 <office:body><office:text>
  <text:h text:outline-level="1">Plan</text:h>
  <text:p>one <text:span text:style-name="B">strong</text:span> word</text:p>
  <text:list><text:list-item><text:p>first</text:p></text:list-item><text:list-item><text:p>second</text:p></text:list-item></text:list>
  <table:table table:name="T"><table:table-column table:number-columns-repeated="2"/>
   <table:table-row><table:table-cell><text:p>a</text:p></table:table-cell><table:table-cell><text:p>b</text:p></table:table-cell></table:table-row>
  </table:table>
 </office:text></office:body>
</office:document>"#;

#[test]
fn a_document_saves_as_markdown() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(NOTES);
    let out = dir.path().join("out.md");
    doc.save_as(&mut worker, &crate::file_url(&out), "md").expect("saves as Markdown");
    let text = std::fs::read_to_string(&out).expect("reads");
    for line in ["# Plan", "one **strong** word", "- first", "- second", "| a | b |"] {
        assert!(text.contains(line), "{line} in {text}");
    }
}
