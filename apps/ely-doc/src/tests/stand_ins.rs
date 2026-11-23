//! A Word font that is not installed draws as the nearest free font.

use ely_office::prelude::*;
use gpui::TestAppContext;

use crate::tests::{docs, settle, settled, with_window};

/// Missing font, and the family that draws it.
const STAND_INS: [(&str, &str); 6] = [
    ("Calibri Light", "Carlito"),
    ("Verdana", "DejaVu Sans"),
    ("Century Gothic", "DejaVu Sans"),
    ("Georgia", "Noto Serif"),
    ("Consolas", "Liberation Mono"),
    ("Lucida Console", "Liberation Mono"),
];

/// Chinese Word fonts, and the bundled family that must be among those drawing them.
const CHINESE: [(&str, &str); 4] = [
    ("宋体", "Noto Serif SC"),
    ("楷体", "Noto Serif SC"),
    ("黑体", "Noto Sans SC"),
    ("微软雅黑", "Noto Sans SC"),
];

/// A document with one span in each font.
fn document() -> String {
    let (mut faces, mut styles, mut spans) = (String::new(), String::new(), String::new());
    let fonts = STAND_INS.iter().chain(&CHINESE);
    for (at, (font, _)) in fonts.enumerate() {
        faces += &format!("<style:font-face style:name=\"{font}\" svg:font-family=\"&apos;{font}&apos;\"/>");
        styles += &format!(
            "<style:style style:name=\"T{at}\" style:family=\"text\"><style:text-properties style:font-name=\"{font}\"/></style:style>"
        );
        spans += &format!("<text:span text:style-name=\"T{at}\">word{at} 中文字体</text:span> ");
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text"><office:font-face-decls>{faces}</office:font-face-decls><office:automatic-styles>{styles}</office:automatic-styles><office:body><office:text><text:p>{spans}</text:p></office:text></office:body></office:document>"#
    )
}

// Linux draws a missing font by fontconfig; the others have their own tables.
#[cfg(target_os = "linux")]
#[test]
fn missing_word_fonts_draw_as_the_nearest_free_font() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("fonts.fodt");
    std::fs::write(&path, document()).expect("the file writes");
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(path.clone(), cx));
    settle(&cx, |app| {
        let tab = docs.read(app).active_tab();
        tab.is_some_and(|tab| !tab.view.read(app).missing_fonts().is_empty())
    });
    settled(&cx, &docs);
    let missing = cx.update(|app| docs.read(app).active_tab().expect("a tab").view.read(app).missing_fonts().to_vec());
    // A font this computer has is not missing, and is drawn as itself.
    let mut checked = 0;
    for (font, used) in STAND_INS {
        if let Some((_, drawn)) = missing.iter().find(|(name, _)| name == font) {
            assert_eq!(drawn, &[used.to_string()], "{font}");
            checked += 1;
        }
    }
    assert!(checked > 0, "some stand-in was needed: {missing:?}");
    // Their Latin letters may be drawn by another font; the Chinese ones by the bundled family.
    for (font, family) in CHINESE {
        let (_, drawn) = missing.iter().find(|(name, _)| name == font).unwrap_or_else(|| panic!("{font} is missing: {missing:?}"));
        assert!(drawn.iter().any(|used| used == family), "{font}: {drawn:?}");
    }
}
