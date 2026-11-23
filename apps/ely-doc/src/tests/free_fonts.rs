//! The free fonts ship with the kernel, so a document in them loses nothing.

use ely_office::prelude::*;
use gpui::TestAppContext;

use crate::tests::{docs, settle, settled, with_window};

/// The families in `packaging/libreoffice/fonts`, as the kernel names them.
const FAMILIES: [&str; 6] = ["Roboto", "Open Sans", "Montserrat", "Lato", "Source Sans 3", "Inter"];

/// A document with a span in each family and one in a font that does not exist.
fn document() -> String {
    let fonts = FAMILIES.iter().chain(&["Nowhere Sans"]);
    let (mut faces, mut styles, mut spans) = (String::new(), String::new(), String::new());
    for (at, font) in fonts.enumerate() {
        faces += &format!("<style:font-face style:name=\"{font}\" svg:font-family=\"&apos;{font}&apos;\"/>");
        styles += &format!(
            "<style:style style:name=\"T{at}\" style:family=\"text\"><style:text-properties style:font-name=\"{font}\"/></style:style>"
        );
        spans += &format!("<text:span text:style-name=\"T{at}\">word{at}</text:span> ");
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text"><office:font-face-decls>{faces}</office:font-face-decls><office:automatic-styles>{styles}</office:automatic-styles><office:body><office:text><text:p>{spans}</text:p></office:text></office:body></office:document>"#
    )
}

#[test]
fn the_free_fonts_are_not_missing_but_an_unknown_font_is() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("fonts.fodt");
    std::fs::write(&path, document()).expect("the file writes");
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
    assert_eq!(missing, ["Nowhere Sans"], "the free fonts are installed with the kernel");
}
