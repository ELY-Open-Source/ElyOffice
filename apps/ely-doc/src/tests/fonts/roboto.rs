//! The kernel ships Roboto, so Roboto is never missing.

use ely_office::prelude::*;
use gpui::TestAppContext;

use crate::tests::{docs, settle, settled, with_window};

const ROBOTO: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:font-face-decls>
  <style:font-face style:name="Roboto" svg:font-family="Roboto"/>
  <style:font-face style:name="Nowhere Sans" svg:font-family="&apos;Nowhere Sans&apos;"/>
 </office:font-face-decls>
 <office:automatic-styles>
  <style:style style:name="T1" style:family="text"><style:text-properties style:font-name="Roboto"/></style:style>
  <style:style style:name="T2" style:family="text"><style:text-properties style:font-name="Nowhere Sans"/></style:style>
 </office:automatic-styles>
 <office:body><office:text><text:p><text:span text:style-name="T1">found</text:span> <text:span text:style-name="T2">lost</text:span></text:p></office:text></office:body>
</office:document>"#;

#[test]
fn roboto_is_not_missing_but_an_unknown_font_is() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("roboto.fodt");
    std::fs::write(&path, ROBOTO).expect("the file writes");
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(path.clone(), cx));
    // The unknown font proves the report came before the check.
    settle(&cx, |app| {
        let tab = docs.read(app).active_tab();
        tab.is_some_and(|tab| !tab.view.read(app).missing_fonts().is_empty())
    });
    settled(&cx, &docs);
    let missing: Vec<String> = cx.update(|app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        tab.view.read(app).missing_fonts().iter().map(|(name, _)| name.clone()).collect()
    });
    assert_eq!(missing, ["Nowhere Sans"], "Roboto is installed with the kernel");
}
